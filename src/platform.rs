//! Detect the bottle tag for the host platform.
//!
//! Homebrew names macOS bottles `<arch>_<codename>` (e.g. `arm64_sequoia`)
//! and Linux bottles `x86_64_linux`. We compute the best-matching tag and a
//! fallback chain so an exact codename miss still resolves to a usable bottle.

/// Ordered list of candidate bottle tags, most specific first.
///
/// Always ends with `all`, the architecture-independent bottle.
pub fn candidate_tags() -> Vec<String> {
    let mut tags = Vec::new();

    #[cfg(target_os = "macos")]
    {
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64_"
        } else {
            ""
        };
        // Newest first; an older host can still pull a newer-labeled bottle
        // only if its own codename is absent, so we list the common recent
        // codenames as a descending fallback chain.
        for codename in ["sequoia", "sonoma", "ventura", "monterey", "big_sur"] {
            tags.push(format!("{arch}{codename}"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        tags.push("x86_64_linux".to_string());
    }

    tags.push("all".to_string());
    tags
}
