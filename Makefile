# feheim — developer & release automation.
# Run `make` or `make help` for the full target list.

BIN     := feheim
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
PREFIX  ?= /usr/local
TAG     := v$(VERSION)

# Cross-compile targets produced by `make dist` and the release workflow.
TARGETS := aarch64-apple-darwin x86_64-apple-darwin \
           x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

.DEFAULT_GOAL := help

# ---------------------------------------------------------------------------
# Build & run
# ---------------------------------------------------------------------------

.PHONY: build
build: ## Build a debug binary
	cargo build

.PHONY: release
release: ## Build an optimized release binary
	cargo build --release

.PHONY: run
run: ## Run feheim (pass args with ARGS="install jq")
	cargo run -- $(ARGS)

.PHONY: install
install: release ## Install the release binary into $(PREFIX)/bin
	install -d "$(PREFIX)/bin"
	install -m 0755 target/release/$(BIN) "$(PREFIX)/bin/$(BIN)"
	@echo "installed $(BIN) $(VERSION) -> $(PREFIX)/bin/$(BIN)"

.PHONY: uninstall
uninstall: ## Remove the installed binary from $(PREFIX)/bin
	rm -f "$(PREFIX)/bin/$(BIN)"

# ---------------------------------------------------------------------------
# Quality gates
# ---------------------------------------------------------------------------

.PHONY: fmt
fmt: ## Format the code
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Check formatting without writing
	cargo fmt --check

.PHONY: lint
lint: ## Run clippy with warnings denied
	cargo clippy --all-targets -- -D warnings

.PHONY: test
test: ## Run the test suite
	cargo test

.PHONY: check
check: fmt-check lint test ## Run every quality gate (fmt, lint, test)

# ---------------------------------------------------------------------------
# Distribution
# ---------------------------------------------------------------------------

.PHONY: dist
dist: ## Cross-build release archives for all targets into dist/
	@mkdir -p dist
	@for t in $(TARGETS); do \
		echo "==> building $$t"; \
		rustup target add $$t >/dev/null 2>&1 || true; \
		cargo build --release --target $$t || { echo "skip $$t (toolchain missing)"; continue; }; \
		tar -C target/$$t/release -czf dist/$(BIN)-$$t.tar.gz $(BIN); \
		echo "    dist/$(BIN)-$$t.tar.gz"; \
	done
	@cd dist && (shasum -a 256 *.tar.gz > SHA256SUMS 2>/dev/null || sha256sum *.tar.gz > SHA256SUMS)
	@echo "==> checksums: dist/SHA256SUMS"

# ---------------------------------------------------------------------------
# Changelog & versioning
# ---------------------------------------------------------------------------

.PHONY: changelog
changelog: ## Regenerate CHANGELOG.md from conventional commits (needs git-cliff)
	@command -v git-cliff >/dev/null 2>&1 || { \
		echo "git-cliff not found. Install: cargo install git-cliff"; exit 1; }
	git-cliff --config cliff.toml --output CHANGELOG.md
	@echo "==> CHANGELOG.md updated"

.PHONY: bump-patch bump-minor bump-major
bump-patch: ## Bump the patch version (x.y.Z)
	@new=$$(sh scripts/bump.sh patch); echo "==> $(VERSION) -> $$new"
bump-minor: ## Bump the minor version (x.Y.0)
	@new=$$(sh scripts/bump.sh minor); echo "==> $(VERSION) -> $$new"
bump-major: ## Bump the major version (X.0.0)
	@new=$$(sh scripts/bump.sh major); echo "==> $(VERSION) -> $$new"

# ---------------------------------------------------------------------------
# Release
# ---------------------------------------------------------------------------

.PHONY: tag
tag: ## Create an annotated git tag for the current Cargo.toml version
	@git rev-parse "$(TAG)" >/dev/null 2>&1 && { echo "tag $(TAG) already exists"; exit 1; } || true
	git tag -a "$(TAG)" -m "$(TAG)"
	@echo "==> tagged $(TAG)"

.PHONY: release-prep
release-prep: check changelog ## Verify, regenerate changelog, and commit the release
	git add CHANGELOG.md Cargo.toml Cargo.lock
	git commit -m "chore(release): $(TAG)"
	@echo "==> release commit ready. Next: make publish"

.PHONY: publish
publish: tag ## Push the release commit and tag (CI builds + uploads binaries)
	git push origin HEAD
	git push origin "$(TAG)"
	@echo "==> pushed $(TAG). GitHub Actions will build and attach binaries."

.PHONY: clean
clean: ## Remove build artifacts
	cargo clean
	rm -rf dist

.PHONY: help
help: ## Show this help
	@echo "feheim $(VERSION) — make targets:"
	@grep -hE '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort \
		| awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'
