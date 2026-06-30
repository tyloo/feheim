# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-06-30

First public release. A single Rust binary that consumes the real Homebrew
formula index and installs real bottles from ghcr.io into a private prefix.

### Added
- Commands: `update`, `search`, `info`, `list`, `install`, `uninstall`,
  `cleanup`, and `doctor`.
- Depth-first dependency resolution with dependencies-first install order.
- OCI bottle download with streaming sha256 verification.
- Cellar extraction and Mach-O relocation (`install_name_tool` + ad-hoc
  `codesign`) so installed binaries resolve their dependencies at runtime.
- `opt/<name>` and prefix symlinking, mirroring `brew link`.
- Orphan cleanup: `cleanup` removes dependencies no longer required by any
  explicitly-requested formula, and `uninstall` sweeps them automatically.
- Explicit-request tracking in `<prefix>/.requested`, bootstrapped from
  installed leaves.
- `doctor` diagnostics: prefix and index health, dead or missing `opt` links,
  broken prefix symlinks, and orphaned dependencies.
- Colorized output and live download progress bars (`indicatif` + `console`),
  auto-disabled on non-TTY output and when `NO_COLOR` is set.

[1.0.0]: https://github.com/tyloo/feheim/releases/tag/v1.0.0
