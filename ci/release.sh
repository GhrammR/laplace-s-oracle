#!/usr/bin/env bash
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:?usage: ./ci/release.sh <version>}"

mkdir -p "$ROOT/.artifacts"

require_clean_git() {
  git -C "$ROOT" diff --quiet
  git -C "$ROOT" diff --cached --quiet
}

require_tool() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required tool: $1" >&2
    exit 1
  }
}

require_tool cargo
require_tool gh

if ! cargo set-version --help >/dev/null 2>&1; then
  echo "missing required cargo subcommand: cargo set-version (install cargo-edit)" >&2
  exit 1
fi

require_clean_git

cargo set-version "$VERSION" --manifest-path "$ROOT/Cargo.toml"
"$ROOT/ci/audit.sh"

grep -q "^## v$VERSION - " "$ROOT/CHANGELOG.md"

git -C "$ROOT" add Cargo.toml Cargo.lock CHANGELOG.md
git -C "$ROOT" commit -m "release: v$VERSION"
git -C "$ROOT" tag "v$VERSION"
gh release create "v$VERSION" --draft --generate-notes --title "v$VERSION"