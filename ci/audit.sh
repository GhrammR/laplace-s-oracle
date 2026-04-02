#!/usr/bin/env bash
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="$ROOT/.artifacts/audit"
TEST_DIR="$ARTIFACT_DIR/test-run"

mkdir -p "$ROOT/.artifacts"
mkdir -p "$TEST_DIR"

echo "[audit] fmt"
cargo fmt --all --manifest-path "$ROOT/Cargo.toml" -- --check

echo "[audit] clippy"
cargo clippy --manifest-path "$ROOT/Cargo.toml" --all-targets --all-features -- -D warnings

echo "[audit] test"
(
  cd "$TEST_DIR"
  cargo test --manifest-path "$ROOT/Cargo.toml"
)