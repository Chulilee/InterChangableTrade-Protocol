#!/usr/bin/env bash
# Deploy a built contract wasm to a Stellar network using the Stellar CLI.
#
# Usage:
#   scripts/deploy.sh <contract-name> [network] [source-account]
#
# Example:
#   scripts/deploy.sh asset-registry testnet alice
#
# Requires the `stellar` CLI: https://developers.stellar.org/docs/tools/cli
set -euo pipefail

cd "$(dirname "$0")/.."

CONTRACT="${1:?contract name required, e.g. asset-registry}"
NETWORK="${2:-testnet}"
SOURCE="${3:-default}"

# Cargo package names use hyphens; wasm files use underscores.
WASM="target/wasm32-unknown-unknown/release/${CONTRACT//-/_}.wasm"

if [[ ! -f "$WASM" ]]; then
    echo "Wasm not found: $WASM"
    echo "Build first: scripts/build.sh"
    exit 1
fi

if ! command -v stellar >/dev/null 2>&1; then
    echo "The 'stellar' CLI is not installed."
    echo "Install: https://developers.stellar.org/docs/tools/cli"
    exit 1
fi

echo "==> Deploying $CONTRACT to $NETWORK as $SOURCE"
stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE" \
    --network "$NETWORK"
