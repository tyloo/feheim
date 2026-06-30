#!/usr/bin/env sh
# Bump the version in Cargo.toml. Usage: scripts/bump.sh major|minor|patch
set -eu

part="${1:-}"
cur=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

maj=$(echo "$cur" | cut -d. -f1)
min=$(echo "$cur" | cut -d. -f2)
pat=$(echo "$cur" | cut -d. -f3)

case "$part" in
  major) maj=$((maj + 1)); min=0; pat=0 ;;
  minor) min=$((min + 1)); pat=0 ;;
  patch) pat=$((pat + 1)) ;;
  *) echo "usage: $0 major|minor|patch" >&2; exit 1 ;;
esac

new="${maj}.${min}.${pat}"

# Replace only the first `version = "..."` line (the [package] one). The
# address range limits the substitution to the first match; the pattern is
# spelled out in full because BSD sed doesn't support empty-regex reuse.
sed -i.bak "1,/^version = \"[^\"]*\"/s/^version = \"[^\"]*\"/version = \"${new}\"/" Cargo.toml
rm -f Cargo.toml.bak

echo "$new"
