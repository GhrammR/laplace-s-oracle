#!/usr/bin/env bash
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"
export PATH="$HOME/.local/bin:$PATH"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="${1:?usage: ./ci/release.sh <version>}"
ARTIFACT_DIR="$ROOT/.artifacts/release"
COMMIT_MESSAGE_FILE="$ARTIFACT_DIR/release-commit-message.txt"
NOTES_FILE="$ARTIFACT_DIR/release-notes.txt"

mkdir -p "$ROOT/.artifacts" "$ARTIFACT_DIR"

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
printf 'release: v%s
' "$VERSION" > "$COMMIT_MESSAGE_FILE"
printf 'Pre-release v%s
' "$VERSION" > "$NOTES_FILE"

git -C "$ROOT" add Cargo.toml Cargo.lock CHANGELOG.md
git -C "$ROOT" commit -F "$COMMIT_MESSAGE_FILE"
git -C "$ROOT" -c tag.gpgSign=false tag -a "v$VERSION" -F "$NOTES_FILE"
gh release create "v$VERSION" --prerelease --title "v$VERSION" --notes-file "$NOTES_FILE"
