//! Bottle relocation.
//!
//! Homebrew bottles are built against placeholder paths (`@@HOMEBREW_PREFIX@@`,
//! `@@HOMEBREW_CELLAR@@`, ...) so they can be poured into any prefix. At install
//! time those placeholders must be rewritten to the real prefix — in Mach-O load
//! commands (via `install_name_tool`, then re-signed) and in text files such as
//! pkg-config `.pc` files.

use crate::config::Config;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;

const TOKEN_PREFIX: &str = "@@HOMEBREW_PREFIX@@";
const TOKEN_CELLAR: &str = "@@HOMEBREW_CELLAR@@";
const TOKEN_REPOSITORY: &str = "@@HOMEBREW_REPOSITORY@@";

/// Fallible unit result used throughout relocation.
type Relocated = Result<(), Box<dyn Error>>;

/// Rewrite all placeholder paths in a freshly extracted keg.
pub fn relocate_keg(cfg: &Config, name: &str, version: &str) -> Result<(), Box<dyn Error>> {
    let keg = cfg.keg(name, version);
    walk(&keg, &|path| relocate_file(cfg, path))
}

/// Substitute the Homebrew path tokens in a string with this prefix's paths.
fn substitute(cfg: &Config, input: &str) -> String {
    let prefix = cfg.prefix.to_string_lossy();
    let cellar_path = cfg.cellar();
    let cellar = cellar_path.to_string_lossy();
    input
        .replace(TOKEN_CELLAR, &cellar)
        // REPOSITORY has no real analogue in a bottle-only install; map to prefix.
        .replace(TOKEN_REPOSITORY, &prefix)
        .replace(TOKEN_PREFIX, &prefix)
}

fn relocate_file(cfg: &Config, path: &Path) -> Result<(), Box<dyn Error>> {
    // Symlinks are relocated via their target; skip them here.
    if path.is_symlink() || !path.is_file() {
        return Ok(());
    }
    // Bottle files are often mode 0444; relocation must write to them.
    ensure_writable(path)?;
    let bytes = fs::read(path)?;
    if is_macho(&bytes) {
        relocate_macho(cfg, path)?;
    } else if let Ok(text) = std::str::from_utf8(&bytes) {
        if text.contains("@@HOMEBREW") {
            let replaced = substitute(cfg, text);
            fs::write(path, replaced)?;
        }
    }
    Ok(())
}

/// Add the owner-write bit so relocation can modify a read-only bottle file.
fn ensure_writable(path: &Path) -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::metadata(path)?;
    let mut perms = meta.permissions();
    let mode = perms.mode();
    if mode & 0o200 == 0 {
        perms.set_mode(mode | 0o200);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

/// Detect a Mach-O (thin or fat) object by magic number.
fn is_macho(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    let m = [bytes[0], bytes[1], bytes[2], bytes[3]];
    matches!(
        m,
        [0xfe, 0xed, 0xfa, 0xce] // 32-bit BE
            | [0xfe, 0xed, 0xfa, 0xcf] // 64-bit BE
            | [0xce, 0xfa, 0xed, 0xfe] // 32-bit LE
            | [0xcf, 0xfa, 0xed, 0xfe] // 64-bit LE
            | [0xca, 0xfe, 0xba, 0xbe] // fat BE
            | [0xbe, 0xba, 0xfe, 0xca] // fat LE
    )
}

/// Rewrite a Mach-O's install id and dependency paths, then re-sign ad-hoc
/// (mandatory on Apple silicon once the binary is modified).
#[cfg(target_os = "macos")]
fn relocate_macho(cfg: &Config, path: &Path) -> Result<(), Box<dyn Error>> {
    // Fix the library's own id, if it carries a placeholder.
    if let Some(id) = macho_id(path)? {
        if id.contains("@@HOMEBREW") {
            run(
                "install_name_tool",
                &["-id", &substitute(cfg, &id), &disp(path)],
            )?;
        }
    }
    // Fix each dependency load command carrying a placeholder.
    for dep in macho_deps(path)? {
        if dep.contains("@@HOMEBREW") {
            let new = substitute(cfg, &dep);
            run("install_name_tool", &["-change", &dep, &new, &disp(path)])?;
        }
    }
    // Re-sign ad-hoc; install_name_tool invalidates the existing signature.
    let _ = run("codesign", &["--force", "--sign", "-", &disp(path)]);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn relocate_macho(_cfg: &Config, _path: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}

/// Parse `otool -D` for a dylib's install id (the line after the filename).
#[cfg(target_os = "macos")]
fn macho_id(path: &Path) -> Result<Option<String>, Box<dyn Error>> {
    let out = capture("otool", &["-D", &disp(path)])?;
    Ok(out
        .lines()
        .nth(1)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty()))
}

/// Parse `otool -L` for dependency paths, skipping the file header line.
#[cfg(target_os = "macos")]
fn macho_deps(path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let out = capture("otool", &["-L", &disp(path)])?;
    let mut deps = Vec::new();
    for line in out.lines().skip(1) {
        let line = line.trim();
        if let Some((p, _)) = line.split_once(" (") {
            deps.push(p.to_string());
        }
    }
    Ok(deps)
}

fn disp(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Run a command, returning an error if it exits non-zero. Child output is
/// suppressed; `install_name_tool` is noisy about signatures we re-create anyway.
fn run(cmd: &str, args: &[&str]) -> Result<(), Box<dyn Error>> {
    use std::process::Stdio;
    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(format!("{cmd} failed for {:?}", args.last().unwrap_or(&"")).into());
    }
    Ok(())
}

/// Run a command and capture stdout as a string.
#[cfg(target_os = "macos")]
fn capture(cmd: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let out = Command::new(cmd).args(args).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Recursively apply `f` to every entry under `root`.
fn walk(root: &Path, f: &dyn Fn(&Path) -> Relocated) -> Relocated {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() && !path.is_symlink() {
            walk(&path, f)?;
        } else {
            f(&path)?;
        }
    }
    Ok(())
}
