//! Fetch and cache the Homebrew formula index.
//!
//! The full index (`formula.json`, ~30 MB) is downloaded once and cached under
//! the prefix. Subsequent commands read the cache, so only `update` hits the
//! network for the index.

use crate::config::Config;
use crate::formula::Formula;
use std::error::Error;
use std::fs;
use std::io::Read;

const FORMULA_INDEX_URL: &str = "https://formulae.brew.sh/api/formula.json";

/// Download the formula index and write it to the cache, overwriting any
/// existing copy.
pub fn update_index(cfg: &Config) -> Result<usize, Box<dyn Error>> {
    fs::create_dir_all(cfg.cache())?;
    let resp = ureq::get(FORMULA_INDEX_URL).call()?;
    let mut body = String::new();
    resp.into_reader().read_to_string(&mut body)?;
    // Validate before persisting so a truncated download can't poison cache.
    let formulae: Vec<Formula> = serde_json::from_str(&body)?;
    fs::write(cfg.index_path(), &body)?;
    Ok(formulae.len())
}

/// Load all formulae from the cache, fetching the index first if absent.
pub fn load_index(cfg: &Config) -> Result<Vec<Formula>, Box<dyn Error>> {
    if !cfg.index_path().exists() {
        update_index(cfg)?;
    }
    let body = fs::read_to_string(cfg.index_path())?;
    let formulae: Vec<Formula> = serde_json::from_str(&body)?;
    Ok(formulae)
}

/// Find a single formula by exact name.
pub fn find(cfg: &Config, name: &str) -> Result<Option<Formula>, Box<dyn Error>> {
    let formulae = load_index(cfg)?;
    Ok(formulae.into_iter().find(|f| f.name == name))
}
