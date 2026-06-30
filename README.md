<p align="center">
  <img src="assets/logo.svg" alt="FEHEIM" width="640">
</p>

<p align="center">
  <em>A Homebrew package manager core, reforged in Rust.</em>
</p>

<p align="center">
  <a href="https://github.com/tyloo/feheim/actions/workflows/ci.yml"><img src="https://github.com/tyloo/feheim/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/tyloo/feheim/releases"><img src="https://img.shields.io/github/v/release/tyloo/feheim?color=2dd4bf&label=release" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-34d399" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/rust-1.74%2B-orange?logo=rust" alt="Rust 1.74+">
  <img src="https://img.shields.io/badge/platform-macOS%20·%20Linux-22d3ee" alt="Platform">
</p>

---

`feheim` consumes the **real** Homebrew formula index and installs **real**
bottles — so it manages the same software Homebrew does, but as a single small
Rust binary with zero Ruby runtime. It installs into a private prefix
(`~/.feheim` by default) and never touches an existing `/opt/homebrew`.

```console
$ feheim install jq
==> Installing jq (+1 dependencies)
  • oniguruma 6.9.10 already installed
  ⬇ downloading jq 1.8.2 ━━━━━━━━━━━━━━━━━━━━ 432.61 KiB/432.61 KiB  4.1 MiB/s
  ✓ jq 1.8.2 (12 files linked)
✓ jq installed.
```

## ✨ Features

- **Real bottles, real index** — pulls ~8,400 formulae from `formulae.brew.sh`
  and OCI bottle blobs from `ghcr.io`, exactly as Homebrew does.
- **Mach-O relocation** — rewrites `@@HOMEBREW_PREFIX@@` / `@@HOMEBREW_CELLAR@@`
  placeholders in compiled binaries so installed software actually runs.
- **Dependency resolution** — depth-first, dependencies-first install order.
- **Orphan cleanup** — `cleanup` (and auto-sweep on `uninstall`) removes
  dependencies no longer required by anything you asked for.
- **Self-diagnosis** — `doctor` reports dead links, broken symlinks, index
  health, and orphans.
- **Pretty CLI** — colorized output and live download progress bars, with
  graceful fallback on non-TTY output and `NO_COLOR`.
- **Single static-ish binary** — no Ruby, no Python, just Rust.

## 📦 Install

### Prebuilt binary

Download the archive for your platform from the
[latest release](https://github.com/tyloo/feheim/releases/latest), then:

```sh
tar -xzf feheim-<target>.tar.gz
sudo mv feheim /usr/local/bin/
feheim --version
```

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`.
Verify integrity against `SHA256SUMS` from the release assets.

### From source

```sh
git clone https://github.com/tyloo/feheim
cd feheim
make install          # builds --release and installs into /usr/local/bin
```

Or with Cargo directly: `cargo install --path .`

## 🚀 Quick start

```sh
feheim update            # fetch & cache the formula index
feheim search json       # find formulae
feheim install jq        # resolve deps, download, relocate, link
~/.feheim/bin/jq --version
feheim doctor            # health check
feheim uninstall jq      # remove + sweep orphaned deps
```

Override the install location with `FEHEIM_PREFIX=/some/path`.

## 🧭 Commands

| Command | Description |
| --- | --- |
| `feheim update` | Download and cache the formula index. |
| `feheim search <query>` | Match formulae by name or description. |
| `feheim info <name>` | Show version, homepage, deps, and install state. |
| `feheim list` | List installed formulae and versions. |
| `feheim install <name>` | Resolve deps, download bottles, relocate, and link. |
| `feheim uninstall <name>` | Unlink, remove, and sweep orphaned dependencies. |
| `feheim cleanup` | Remove all orphaned dependencies. |
| `feheim doctor` | Diagnose the installation. |

## 🔬 How it works

It reimplements the parts of Homebrew that make a bottle install actually run:

1. **Index** (`api.rs`) — fetch + cache the formula JSON.
2. **Resolution** (`commands.rs`) — depth-first, dependencies-first ordering;
   explicit requests recorded in `<prefix>/.requested` to drive orphan cleanup.
3. **Download** (`install.rs`) — stream OCI blobs from ghcr.io with the
   anonymous bearer token Homebrew uses, hashing and verifying sha256 inline.
4. **Relocation** (`relocate.rs`) — rewrite placeholder paths in Mach-O load
   commands (`install_name_tool` + ad-hoc `codesign`) and in text files (`.pc`).
5. **Linking** (`install.rs`) — `opt/<name>` symlinks plus `bin`/`lib`/…
   symlinks into the prefix, mirroring `brew link`.

> **Platform note:** binary relocation currently targets **macOS** Mach-O
> bottles. The CLI builds and runs on Linux, and ELF relocation is on the
> roadmap.

## 🛠️ Development

```sh
make build        # debug build
make check        # fmt + clippy + tests
make run ARGS="install jq"
make help         # list all targets
```

## 📤 Releasing

Releases are cut from git tags; GitHub Actions cross-builds and attaches the
binaries automatically.

```sh
make bump-patch       # or bump-minor / bump-major  → updates Cargo.toml
make release-prep     # check + regenerate CHANGELOG + release commit
make publish          # tag vX.Y.Z and push → CI builds & uploads binaries
```

`make dist` builds the release archives + `SHA256SUMS` locally.
The changelog is generated from [Conventional Commits](https://www.conventionalcommits.org)
via [`git-cliff`](https://git-cliff.org) (`make changelog`).

## 🗺️ Roadmap

- [ ] ELF (Linux) bottle relocation
- [ ] `upgrade` / `outdated`
- [ ] Building from source for formulae without bottles
- [ ] Taps beyond `homebrew/core`
- [ ] Casks and services

## 🤝 Contributing

Issues and PRs welcome. Run `make check` before opening a PR, and use
Conventional Commit messages so the changelog stays tidy.

## 📄 License

[MIT](LICENSE) © Tyloo

## 🙏 Acknowledgements

Built on the shoulders of [Homebrew](https://brew.sh) — feheim consumes its
public formula index and bottle infrastructure. It is an independent project
and not affiliated with or endorsed by the Homebrew project.
