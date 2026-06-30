//! Bottle download, verification, extraction, and symlinking.
//!
//! A "bottle" is a pre-built binary tarball hosted as an OCI blob on
//! ghcr.io. We download the blob (anonymous bearer token, as Homebrew does),
//! verify its sha256, extract it into the Cellar, then symlink the keg's
//! contents into the prefix.

use crate::config::Config;
use crate::formula::Formula;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use tar::Archive;

/// Anonymous bearer token Homebrew uses for ghcr.io bottle blobs.
const GHCR_ANON_TOKEN: &str = "Bearer QQ==";

/// Download a bottle for `formula` matching one of `tags`, verify it, and
/// return the path to the cached tarball plus the resolved keg version.
pub fn download_bottle(
    cfg: &Config,
    formula: &Formula,
    tags: &[String],
) -> Result<(std::path::PathBuf, String), Box<dyn Error>> {
    let file = tags
        .iter()
        .find_map(|t| formula.bottle_for(t))
        .ok_or_else(|| format!("no bottle available for {} on this platform", formula.name))?;

    let version = keg_version(formula);
    fs::create_dir_all(cfg.cache())?;
    let dest = cfg
        .cache()
        .join(format!("{}-{}.tar.gz", formula.name, version));

    // Reuse a previously downloaded, verified bottle.
    if dest.exists() && sha256_file(&dest)? == file.sha256 {
        return Ok((dest, version));
    }

    let resp = ureq::get(&file.url)
        .set("Authorization", GHCR_ANON_TOKEN)
        .call()?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    // Stream the blob in chunks, hashing and updating the progress bar as we
    // go rather than slurping the whole body up front.
    let pb = crate::ui::download_bar(total, &formula.name, &version);
    let mut reader = resp.into_reader();
    let mut buf = Vec::with_capacity(total as usize);
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 65536];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
        buf.extend_from_slice(&chunk[..n]);
        pb.inc(n as u64);
    }
    pb.finish_and_clear();

    let actual = hex(&hasher.finalize());
    if actual != file.sha256 {
        return Err(format!(
            "sha256 mismatch for {}: expected {}, got {}",
            formula.name, file.sha256, actual
        )
        .into());
    }

    let mut f = fs::File::create(&dest)?;
    f.write_all(&buf)?;
    Ok((dest, version))
}

/// Extract a bottle tarball into the Cellar. Bottles are rooted at
/// `<name>/<version>/...`, so the extraction base is the Cellar itself.
pub fn extract_bottle(cfg: &Config, tarball: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(cfg.cellar())?;
    let file = fs::File::open(tarball)?;
    let mut archive = Archive::new(GzDecoder::new(file));
    archive.unpack(cfg.cellar())?;
    Ok(())
}

/// Symlink every file under a keg into the matching prefix directory,
/// mirroring `brew link`.
pub fn link_keg(cfg: &Config, name: &str, version: &str) -> Result<u32, Box<dyn Error>> {
    let keg = cfg.keg(name, version);
    let mut linked = 0;
    for dir in cfg.link_dirs() {
        let src_dir = keg.join(dir);
        if !src_dir.is_dir() {
            continue;
        }
        let dst_dir = cfg.prefix.join(dir);
        link_tree(&src_dir, &dst_dir, &mut linked)?;
    }
    Ok(linked)
}

/// Recursively symlink files from `src` into `dst`, creating directories as
/// needed so two formulae can share a prefix subdirectory.
fn link_tree(src: &Path, dst: &Path, linked: &mut u32) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            link_tree(&src_path, &dst_path, linked)?;
        } else {
            // Replace a stale link so reinstall is idempotent.
            if dst_path.exists() || dst_path.is_symlink() {
                fs::remove_file(&dst_path)?;
            }
            std::os::unix::fs::symlink(&src_path, &dst_path)?;
            *linked += 1;
        }
    }
    Ok(())
}

/// Remove every prefix symlink that points into the given keg.
pub fn unlink_keg(cfg: &Config, name: &str, version: &str) -> Result<u32, Box<dyn Error>> {
    let keg = cfg.keg(name, version);
    let mut removed = 0;
    for dir in cfg.link_dirs() {
        let dst_dir = cfg.prefix.join(dir);
        if dst_dir.is_dir() {
            unlink_tree(&dst_dir, &keg, &mut removed)?;
        }
    }
    Ok(removed)
}

/// Remove symlinks under `dir` whose target lives inside `keg`.
fn unlink_tree(dir: &Path, keg: &Path, removed: &mut u32) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() {
            if let Ok(target) = fs::read_link(&path) {
                if target.starts_with(keg) {
                    fs::remove_file(&path)?;
                    *removed += 1;
                }
            }
        } else if path.is_dir() {
            unlink_tree(&path, keg, removed)?;
        }
    }
    Ok(())
}

/// Create the `<prefix>/opt/<name>` symlink pointing at the active keg,
/// replacing any stale link.
pub fn opt_link(cfg: &Config, name: &str, version: &str) -> Result<(), Box<dyn Error>> {
    let opt = cfg.opt(name);
    if let Some(parent) = opt.parent() {
        fs::create_dir_all(parent)?;
    }
    if opt.exists() || opt.is_symlink() {
        fs::remove_file(&opt)?;
    }
    std::os::unix::fs::symlink(cfg.keg(name, version), &opt)?;
    Ok(())
}

/// Remove the `opt` symlink for a formula, if present.
pub fn opt_unlink(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    let opt = cfg.opt(name);
    if opt.is_symlink() || opt.exists() {
        fs::remove_file(&opt)?;
    }
    Ok(())
}

/// Keg version string, appending `_<revision>` when the formula has a non-zero revision.
///
/// Matches Homebrew's keg path (`Cellar/<name>/<version>_<revision>`). Note this uses the
/// formula's `revision`, not the bottle's `rebuild`: the rebuild count only affects bottle
/// filenames, never the on-disk keg directory.
pub fn keg_version(formula: &Formula) -> String {
    let base = formula.stable_version();
    if formula.revision > 0 {
        format!("{}_{}", base, formula.revision)
    } else {
        base
    }
}

fn sha256_file(path: &Path) -> Result<String, Box<dyn Error>> {
    let mut f = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}
