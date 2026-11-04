#!/usr/bin/env bash
# Check that no compiled WASM contract exceeds the Soroban size limit.
#
# Soroban enforces a 64 KiB (65536 byte) limit on deployed contract WASM.
# This script reports the size of every built artifact and exits non-zero if
# any single artifact exceeds the limit, so size regressions are caught before
# they make a contract undeployable.
#
# Usage:
#   ./scripts/check-size.sh            # check all artifacts in target/
#   ./scripts/check-size.sh <dir>      # check artifacts in <dir>
set -euo pipefail

LIMIT=65536
DIR="${1:-target/wasm32-unknown-unknown/release}"

if [ ! -d "$DIR" ]; then
    echo "No build output at $DIR — run scripts/build.sh first." >&2
    exit 1
fi

shopt -s nullglob
wasm_files=("$DIR"/*.wasm)
shopt -u nullglob

if [ ${#wasm_files[@]} -eq 0 ]; then
    echo "No .wasm files found in $DIR" >&2
    exit 1
fi

fail=0
printf '%-50s %10s %10s\n' "CONTRACT" "SIZE" "HEADROOM"
printf '%-50s %10s %10s\n' "--------" "----" "--------"

for wasm in "${wasm_files[@]}"; do
    size=$(stat -c%s "$wasm" 2>/dev/null || stat -f%z "$wasm")
    headroom=$((LIMIT - size))
    name=$(basename "$wasm")
    if [ "$size" -gt "$LIMIT" ]; then
        printf '%-50s %10s %10s  ❌ OVER LIMIT\n' "$name" "$size" "$headroom"
        fail=1
    else
        printf '%-50s %10s %10s  ✅\n' "$name" "$size" "$headroom"
    fi
done

echo
echo "Size limit: $LIMIT bytes (64 KiB)"
if [ "$fail" -ne 0 ]; then
    echo "ERROR: one or more contracts exceed the Soroban 64 KiB limit." >&2
    exit 1
fi
echo "All contracts within limit."
