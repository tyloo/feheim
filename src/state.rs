//! Tracking of explicitly-requested formulae.
//!
//! feheim records which formulae the user installed directly (rather than as a
//! pulled-in dependency) in `<prefix>/.requested`, one name per line. `cleanup`
//! uses this set as the roots of the dependency graph: anything installed but
//! not reachable from a requested formula is an orphan.

use crate::config::Config;
use std::collections::HashSet;
use std::error::Error;
use std::fs;

/// Load the requested set. Missing or unreadable file yields an empty set.
pub fn load(cfg: &Config) -> HashSet<String> {
    fs::read_to_string(cfg.requested_path())
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Persist the requested set, sorted, one name per line.
pub fn save(cfg: &Config, set: &HashSet<String>) -> Result<(), Box<dyn Error>> {
    let mut names: Vec<&str> = set.iter().map(|s| s.as_str()).collect();
    names.sort_unstable();
    fs::create_dir_all(&cfg.prefix)?;
    fs::write(cfg.requested_path(), names.join("\n"))?;
    Ok(())
}

/// Mark `name` as explicitly requested.
pub fn add(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    let mut set = load(cfg);
    if set.insert(name.to_string()) {
        save(cfg, &set)?;
    }
    Ok(())
}

/// Drop `name` from the requested set (e.g. on uninstall).
pub fn remove(cfg: &Config, name: &str) -> Result<(), Box<dyn Error>> {
    let mut set = load(cfg);
    if set.remove(name) {
        save(cfg, &set)?;
    }
    Ok(())
}
