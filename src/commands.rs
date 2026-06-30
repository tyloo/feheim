//! Implementations of the user-facing subcommands.

use crate::config::Config;
use crate::formula::Formula;
use crate::install;
use crate::platform;
use crate::relocate;
use crate::state;
use crate::ui;
use crate::{api, error::FeheimError};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// `update` — refresh the cached formula index.
pub fn update(cfg: &Config) -> Result<(), Box<dyn Error>> {
    println!("{} Updating formula index...", ui::arrow());
    let count = api::update_index(cfg)?;
    println!("{} Indexed {} formulae.", ui::check(), ui::version(count));
    Ok(())
}

/// `search <query>` — list formulae whose name or description matches.
pub fn search(cfg: &Config, query: &str) -> Result<(), Box<dyn Error>> {
    let q = query.to_lowercase();
    let formulae = api::load_index(cfg)?;
    let mut hits: Vec<&Formula> = formulae
        .iter()
        .filter(|f| {
            f.name.to_lowercase().contains(&q)
                || f.desc
                    .as_deref()
                    .map(|d| d.to_lowercase().contains(&q))
                    .unwrap_or(false)
        })
        .collect();
    hits.sort_by(|a, b| a.name.cmp(&b.name));

    if hits.is_empty() {
        println!(
            "{} No formulae matching \"{}\".",
            ui::bullet(),
            ui::name(query)
        );
        return Ok(());
    }
    for f in hits.iter().take(50) {
        let desc = f.desc.as_deref().unwrap_or("");
        // Pad the plain name before styling so ANSI codes don't break alignment.
        let padded = format!("{:<28}", f.name);
        println!("{} {}", ui::name(padded), ui::dim(desc));
    }
    if hits.len() > 50 {
        println!("{}", ui::dim(format!("… and {} more", hits.len() - 50)));
    }
    Ok(())
}

/// `info <name>` — show metadata and install status for one formula.
pub fn info(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    let formula = api::find(cfg, name)?.ok_or(FeheimError::NotFound(name.to_string()))?;
    println!(
        "{} {}",
        ui::name(&formula.name),
        ui::version(formula.stable_version())
    );
    if let Some(desc) = &formula.desc {
        println!("{desc}");
    }
    if let Some(home) = &formula.homepage {
        println!("{}", ui::url(home));
    }
    if formula.dependencies.is_empty() {
        println!("{} none", ui::dim("Dependencies:"));
    } else {
        println!(
            "{} {}",
            ui::dim("Dependencies:"),
            formula.dependencies.join(", ")
        );
    }
    let version = install::keg_version(&formula);
    if cfg.keg(&formula.name, &version).is_dir() {
        println!(
            "{} installed {} (linked)",
            ui::check(),
            ui::version(&version)
        );
    } else {
        println!("{} not installed", ui::bullet());
    }
    Ok(())
}

/// `list` — show installed formulae and their versions.
pub fn list(cfg: &Config) -> Result<(), Box<dyn Error>> {
    let cellar = cfg.cellar();
    if !cellar.is_dir() {
        println!("{} No formulae installed.", ui::bullet());
        return Ok(());
    }
    let mut names: Vec<String> = Vec::new();
    for entry in fs::read_dir(&cellar)? {
        let entry = entry?;
        if entry.path().is_dir() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    if names.is_empty() {
        println!("{} No formulae installed.", ui::bullet());
        return Ok(());
    }
    for name in names {
        let mut versions: Vec<String> = fs::read_dir(cfg.formula_cellar(&name))?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        versions.sort();
        let padded = format!("{:<28}", name);
        println!("{} {}", ui::name(padded), ui::version(versions.join(", ")));
    }
    Ok(())
}

/// `install <name>` — install a formula and its dependencies.
pub fn install(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    let formulae = api::load_index(cfg)?;
    let by_name: HashMap<String, Formula> =
        formulae.into_iter().map(|f| (f.name.clone(), f)).collect();

    if !by_name.contains_key(name) {
        return Err(FeheimError::NotFound(name.to_string()).into());
    }

    let mut order = Vec::new();
    let mut seen = HashSet::new();
    resolve(name, &by_name, &mut seen, &mut order)?;

    println!(
        "{} Installing {} {}",
        ui::arrow(),
        ui::name(name),
        ui::dim(format!("(+{} dependencies)", order.len() - 1))
    );
    let tags = platform::candidate_tags();
    for dep in &order {
        install_one(cfg, &by_name[dep], &tags)?;
    }
    // Record the top-level request so `cleanup` treats its dependencies as
    // required rather than orphaned.
    state::add(cfg, name)?;
    println!("{} {} installed.", ui::check(), ui::name(name));
    Ok(())
}

/// Depth-first dependency resolution producing a dependencies-first order.
fn resolve(
    name: &str,
    by_name: &HashMap<String, Formula>,
    seen: &mut HashSet<String>,
    order: &mut Vec<String>,
) -> Result<(), Box<dyn Error>> {
    if seen.contains(name) {
        return Ok(());
    }
    seen.insert(name.to_string());
    let formula = by_name
        .get(name)
        .ok_or(FeheimError::NotFound(name.to_string()))?;
    for dep in &formula.dependencies {
        // Skip deps absent from the index (e.g. system-provided) rather than
        // aborting the whole install.
        if by_name.contains_key(dep) {
            resolve(dep, by_name, seen, order)?;
        }
    }
    order.push(name.to_string());
    Ok(())
}

/// Install a single already-resolved formula, skipping if already present.
fn install_one(cfg: &Config, formula: &Formula, tags: &[String]) -> Result<(), Box<dyn Error>> {
    let version = install::keg_version(formula);
    if cfg.keg(&formula.name, &version).is_dir() {
        println!(
            "  {} {} {} {}",
            ui::bullet(),
            ui::name(&formula.name),
            ui::version(&version),
            ui::dim("already installed")
        );
        return Ok(());
    }
    // The download progress bar is shown by `download_bottle`.
    let (tarball, version) = install::download_bottle(cfg, formula, tags)?;
    let pb = ui::step_spinner(format!(
        "installing {} {}",
        ui::name(&formula.name),
        ui::version(&version)
    ));
    install::extract_bottle(cfg, &tarball)?;
    // opt link before relocation: a keg's own placeholders may resolve through
    // its own opt path, and dependencies' opt links already exist.
    install::opt_link(cfg, &formula.name, &version)?;
    relocate::relocate_keg(cfg, &formula.name, &version)?;
    let linked = install::link_keg(cfg, &formula.name, &version)?;
    pb.finish_and_clear();
    println!(
        "  {} {} {} {}",
        ui::check(),
        ui::name(&formula.name),
        ui::version(&version),
        ui::dim(format!("({linked} files linked)"))
    );
    Ok(())
}

/// `uninstall <name>` — unlink and remove a formula, then sweep any orphaned
/// dependencies it leaves behind.
pub fn uninstall(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    if !cfg.formula_cellar(name).is_dir() {
        return Err(FeheimError::NotInstalled(name.to_string()).into());
    }
    let (versions, unlinked) = remove_formula(cfg, name)?;
    println!(
        "{} Uninstalled {} {}",
        ui::check(),
        ui::name(name),
        ui::dim(format!("({versions} versions, {unlinked} links removed)"))
    );

    // The removed formula may have been the sole consumer of some dependencies;
    // sweep them so an uninstall leaves nothing dangling behind.
    let orphans = compute_orphans(cfg)?;
    if !orphans.is_empty() {
        remove_orphans(cfg, &orphans)?;
    }
    Ok(())
}

/// Remove every installed version of a formula: unlink from the prefix, drop
/// its `opt` link, delete the keg, and unregister it. Returns the number of
/// versions removed and prefix links removed.
fn remove_formula(cfg: &Config, name: &str) -> Result<(usize, u32), Box<dyn Error>> {
    let formula_cellar = cfg.formula_cellar(name);
    let versions: Vec<String> = fs::read_dir(&formula_cellar)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let mut unlinked = 0;
    for version in &versions {
        unlinked += install::unlink_keg(cfg, name, version)?;
    }
    install::opt_unlink(cfg, name)?;
    fs::remove_dir_all(&formula_cellar)?;
    state::remove(cfg, name)?;
    Ok((versions.len(), unlinked))
}

/// `cleanup` — remove every orphaned dependency.
pub fn cleanup(cfg: &Config) -> Result<(), Box<dyn Error>> {
    let orphans = compute_orphans(cfg)?;
    if orphans.is_empty() {
        println!("{} No orphan dependencies.", ui::check());
        return Ok(());
    }
    remove_orphans(cfg, &orphans)?;
    Ok(())
}

/// Remove the given orphan formulae, printing a per-formula receipt.
fn remove_orphans(cfg: &Config, orphans: &[String]) -> Result<(), Box<dyn Error>> {
    let noun = if orphans.len() == 1 {
        "orphan"
    } else {
        "orphans"
    };
    println!("{} Removing {} {}", ui::arrow(), orphans.len(), noun);
    for name in orphans {
        let (versions, unlinked) = remove_formula(cfg, name)?;
        println!(
            "  {} {} {}",
            ui::check(),
            ui::name(name),
            ui::dim(format!("({versions} versions, {unlinked} links removed)"))
        );
    }
    Ok(())
}

/// Names of all installed formulae (top-level Cellar directories).
fn installed_names(cfg: &Config) -> Result<Vec<String>, Box<dyn Error>> {
    let cellar = cfg.cellar();
    if !cellar.is_dir() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = fs::read_dir(&cellar)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// Map of formula name → its declared dependencies, from the index.
fn dependency_map(cfg: &Config) -> Result<HashMap<String, Vec<String>>, Box<dyn Error>> {
    let formulae = api::load_index(cfg)?;
    Ok(formulae
        .into_iter()
        .map(|f| (f.name, f.dependencies))
        .collect())
}

/// Resolve the requested roots, bootstrapping the record from installed leaves
/// (formulae nothing else installed depends on) when it doesn't yet exist.
fn requested_roots(
    cfg: &Config,
    installed: &HashSet<String>,
    deps: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut roots = state::load(cfg);
    if roots.is_empty() && !installed.is_empty() {
        let mut depended = HashSet::new();
        for name in installed {
            if let Some(ds) = deps.get(name) {
                for d in ds {
                    if installed.contains(d) {
                        depended.insert(d.clone());
                    }
                }
            }
        }
        roots = installed.difference(&depended).cloned().collect();
        let _ = state::save(cfg, &roots);
    }
    // Drop any stale entries no longer installed.
    roots.retain(|r| installed.contains(r));
    roots
}

/// Installed formulae not reachable from any requested root through the
/// dependency graph. Returned sorted.
fn compute_orphans(cfg: &Config) -> Result<Vec<String>, Box<dyn Error>> {
    let installed: HashSet<String> = installed_names(cfg)?.into_iter().collect();
    if installed.is_empty() {
        return Ok(Vec::new());
    }
    let deps = dependency_map(cfg)?;
    let roots = requested_roots(cfg, &installed, &deps);

    // Walk the dependency graph from the roots, keeping everything reachable.
    let mut keep = HashSet::new();
    let mut stack: Vec<String> = roots.into_iter().collect();
    while let Some(name) = stack.pop() {
        if !keep.insert(name.clone()) {
            continue;
        }
        if let Some(ds) = deps.get(&name) {
            for d in ds {
                if installed.contains(d) && !keep.contains(d) {
                    stack.push(d.clone());
                }
            }
        }
    }

    let mut orphans: Vec<String> = installed.difference(&keep).cloned().collect();
    orphans.sort();
    Ok(orphans)
}

/// Health level for a single doctor check.
enum Health {
    Ok,
    Warn,
    Fail,
}

/// `doctor` — diagnose the installation and report problems.
pub fn doctor(cfg: &Config) -> Result<(), Box<dyn Error>> {
    println!("{} feheim doctor", ui::arrow());
    let mut checks: Vec<(Health, String)> = Vec::new();

    // Prefix.
    if cfg.prefix.is_dir() {
        checks.push((Health::Ok, format!("prefix {}", cfg.prefix.display())));
    } else {
        checks.push((
            Health::Fail,
            format!("prefix missing: {}", cfg.prefix.display()),
        ));
    }

    // Formula index.
    if cfg.index_path().exists() {
        match api::load_index(cfg) {
            Ok(idx) => checks.push((Health::Ok, format!("formula index: {} formulae", idx.len()))),
            Err(e) => checks.push((Health::Fail, format!("formula index unreadable: {e}"))),
        }
    } else {
        checks.push((
            Health::Warn,
            "formula index not fetched (run `feheim update`)".to_string(),
        ));
    }

    // Installed formulae.
    let installed = installed_names(cfg)?;
    checks.push((
        Health::Ok,
        format!("{} formulae installed", installed.len()),
    ));

    // Dead opt links: opt/<name> that is missing or doesn't resolve. This is
    // exactly the failure class behind the pcre2/freetds dyld errors —
    // `exists()` follows the symlink, so it's false for both a dangling link
    // and an absent one.
    let mut dead_opt: Vec<String> = Vec::new();
    for name in &installed {
        if !cfg.opt(name).exists() {
            dead_opt.push(name.clone());
        }
    }
    if dead_opt.is_empty() {
        checks.push((Health::Ok, "all opt links resolve".to_string()));
    } else {
        for name in &dead_opt {
            checks.push((Health::Fail, format!("dead opt link: opt/{name}")));
        }
    }

    // Broken symlinks anywhere under the prefix link dirs.
    let mut broken: Vec<PathBuf> = Vec::new();
    for dir in cfg.link_dirs() {
        let d = cfg.prefix.join(dir);
        if d.is_dir() {
            collect_broken_links(&d, &mut broken)?;
        }
    }
    if broken.is_empty() {
        checks.push((Health::Ok, "no broken prefix symlinks".to_string()));
    } else {
        checks.push((
            Health::Warn,
            format!("{} broken prefix symlink(s)", broken.len()),
        ));
    }

    // Orphaned dependencies.
    let orphans = compute_orphans(cfg)?;
    if orphans.is_empty() {
        checks.push((Health::Ok, "no orphan dependencies".to_string()));
    } else {
        checks.push((
            Health::Warn,
            format!(
                "{} orphan dependenc{} (run `feheim cleanup`): {}",
                orphans.len(),
                if orphans.len() == 1 { "y" } else { "ies" },
                orphans.join(", ")
            ),
        ));
    }

    // Render checks.
    let mut warns = 0;
    let mut fails = 0;
    for (health, msg) in &checks {
        let tag = match health {
            Health::Ok => ui::check(),
            Health::Warn => {
                warns += 1;
                ui::warn()
            }
            Health::Fail => {
                fails += 1;
                ui::fail()
            }
        };
        println!("  {tag} {msg}");
    }
    // List broken-link paths under their summary line.
    for path in broken.iter().take(10) {
        println!("      {}", ui::dim(path.display()));
    }
    if broken.len() > 10 {
        println!(
            "      {}",
            ui::dim(format!("… and {} more", broken.len() - 10))
        );
    }

    // Summary.
    println!();
    if warns == 0 && fails == 0 {
        println!("{} Your system is ready to brew.", ui::check());
    } else {
        println!(
            "{} {} warning(s), {} error(s).",
            if fails > 0 { ui::fail() } else { ui::warn() },
            warns,
            fails
        );
    }
    Ok(())
}

/// Recursively collect dangling symlinks (targets that don't exist) under `dir`.
fn collect_broken_links(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_symlink() {
            // `exists()` follows the link, so a dangling symlink reports false.
            if !path.exists() {
                out.push(path);
            }
        } else if path.is_dir() {
            collect_broken_links(&path, out)?;
        }
    }
    Ok(())
}
