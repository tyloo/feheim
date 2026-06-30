//! Data model for a Homebrew formula, as served by the formulae.brew.sh API.
//!
//! Only the fields feheim actually uses are deserialized; the API payload is
//! large and the rest is ignored.

use serde::Deserialize;
use std::collections::HashMap;

/// One formula entry from `formula.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Formula {
    pub name: String,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub versions: Versions,
    /// Homebrew formula revision; appended to the keg version as `_<revision>` when non-zero.
    #[serde(default)]
    pub revision: u32,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub bottle: BottleSpec,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Versions {
    pub stable: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BottleSpec {
    #[serde(default)]
    pub stable: Option<Bottle>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bottle {
    /// Bottle rebuild count. Affects bottle filenames only, not the keg path; retained for
    /// completeness and future use.
    #[serde(default)]
    #[allow(dead_code)]
    pub rebuild: u32,
    /// Per-platform bottle files, keyed by tag (e.g. `arm64_sequoia`, `all`).
    #[serde(default)]
    pub files: HashMap<String, BottleFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BottleFile {
    pub url: String,
    pub sha256: String,
}

impl Formula {
    /// Resolved stable version string, or `"0"` if the API omitted it.
    pub fn stable_version(&self) -> String {
        self.versions
            .stable
            .clone()
            .unwrap_or_else(|| "0".to_string())
    }

    /// Pick the bottle file for the given platform tag, falling back to `all`.
    pub fn bottle_for(&self, tag: &str) -> Option<&BottleFile> {
        let bottle = self.bottle.stable.as_ref()?;
        bottle.files.get(tag).or_else(|| bottle.files.get("all"))
    }
}
