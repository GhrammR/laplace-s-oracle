#!/usr/bin/env bash
set -euo pipefail
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="$ROOT/.artifacts/sanity-check"
TEST_DIR="$ARTIFACT_DIR/test-run"

mkdir -p "$ROOT/.artifacts"
mkdir -p "$TEST_DIR"

run_test() {
  local test_name="$1"
  echo "[sanity-check] $test_name"
  (
    cd "$TEST_DIR"
    cargo test --manifest-path "$ROOT/Cargo.toml" --test integrity_check "$test_name" -- --exact
  )
}

run_test test_biological_determinism
run_test test_brain_determinism
run_test telemetry_frame_size_seal
run_test test_seeding_determinism
run_test test_hydrologic_cycle
run_test test_linguistic_trade
run_test test_toroidal_wrapping
run_test test_ipc_nonce_rejection