#!/usr/bin/env bash
# Run the exact same checks the CI runs locally, in order and with the
# commands CI uses. Intended to be the single source of truth for
# "is this workspace green?".
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> 1/5 ruff-equivalent: cargo fmt --check"
cargo fmt --all -- --check

echo "==> 2/5 cargo check (workspace, all targets)"
cargo check --workspace --all-targets

echo "==> 3/5 cargo clippy (workspace, all targets, deny warnings)"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> 4/5 build wasm artifacts (release, wasm32-unknown-unknown)"
scripts/build.sh

echo "==> 5/5 workspace tests"
cargo test --workspace

echo
echo "All checks passed."
