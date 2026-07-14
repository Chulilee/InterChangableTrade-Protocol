#!/usr/bin/env bash
# Run the full workspace test suite.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Running tests"
cargo test --workspace

echo "==> Checking formatting and lints"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
