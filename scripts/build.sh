#!/usr/bin/env bash
# Build all contracts to optimized wasm for the wasm32 target.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Building workspace (release, wasm32-unknown-unknown)"
cargo build --release --target wasm32-unknown-unknown

echo "==> Artifacts:"
ls -1 target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null || {
    echo "No wasm produced. Ensure the wasm32-unknown-unknown target is installed:"
    echo "    rustup target add wasm32-unknown-unknown"
    exit 1
}
