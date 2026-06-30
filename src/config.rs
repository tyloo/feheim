//! Filesystem layout for a feheim installation.
//!
//! Mirrors Homebrew's prefix/Cellar model but defaults to a private prefix
//! (`~/.feheim`) so it never disturbs a real `/opt/homebrew` install.

use std::path::PathBuf;

/// Resolved paths for the active feheim prefix.
pub struct Config {
    /// Install prefix. `bin`, `lib`, etc. are linked here.
    pub prefix: PathBuf,
}

impl Config {
    /// Build config from the environment.
    ///
    /// Honors `FEHEIM_PREFIX`; otherwise defaults to `~/.feheim`.
    pub fn load() -> Config {
        let prefix = std::env::var_os("FEHEIM_PREFIX")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .expect("could not determine home directory")
                    .join(".feheim")
            });
        Config { prefix }
    }

    /// Where extracted kegs live: `<prefix>/Cellar/<name>/<version>`.
    pub fn cellar(&self) -> PathBuf {
        self.prefix.join("Cellar")
    }

    /// Keg directory for a specific formula version.
    pub fn keg(&self, name: &str, version: &str) -> PathBuf {
        self.cellar().join(name).join(version)
    }

    /// Per-formula cellar root: `<prefix>/Cellar/<name>`.
    pub fn formula_cellar(&self, name: &str) -> PathBuf {
        self.cellar().join(name)
    }

    /// Stable per-formula symlink: `<prefix>/opt/<name>` → active keg.
    ///
    /// Bottles reference dependencies through `opt` so the path is version
    /// independent; relocation rewrites placeholders to point here.
    pub fn opt(&self, name: &str) -> PathBuf {
        self.prefix.join("opt").join(name)
    }

    /// Downloaded bottle cache.
    pub fn cache(&self) -> PathBuf {
        self.prefix.join("cache")
    }

    /// Cached copy of the formula index.
    pub fn index_path(&self) -> PathBuf {
        self.cache().join("formula.json")
    }

    /// Record of explicitly-requested formulae, one name per line. Used to
    /// distinguish user-installed formulae from dependencies during `cleanup`.
    pub fn requested_path(&self) -> PathBuf {
        self.prefix.join(".requested")
    }

    /// Directories that receive symlinks into a keg.
    pub fn link_dirs(&self) -> &'static [&'static str] {
        &["bin", "sbin", "etc", "include", "lib", "share"]
    }
}
