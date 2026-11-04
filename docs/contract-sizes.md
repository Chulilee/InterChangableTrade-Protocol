# Contract Sizes

Soroban enforces a **64 KiB (65,536 byte)** limit on deployed contract WASM.
This document tracks the compiled size of each contract so regressions are
visible in review.

## How to measure

```bash
# Build all contracts
./scripts/build.sh

# Or check sizes standalone (after building)
./scripts/check-size.sh
```

The CI workflow (`.github/workflows/contracts-check.yml`) also enforces the
64 KiB limit on every PR — any contract exceeding it will fail the build.

## Current sizes

Run `./scripts/check-size.sh` after a release build to populate this table.
Sizes are measured from `target/wasm32-unknown-unknown/release/<name>.wasm`.

| Contract | Size (bytes) | Headroom |
|----------|-------------:|---------:|
| access-control | _measure_ | _measure_ |
| asset-registry | _measure_ | _measure_ |
| escrow | _measure_ | _measure_ |
| fee-commission | _measure_ | _measure_ |
| governance | _measure_ | _measure_ |
| liquidity-incentives | _measure_ | _measure_ |
| margining-liquidation | _measure_ | _measure_ |
| marketplace | _measure_ | _measure_ |
| matching-engine | _measure_ | _measure_ |
| orderbook | _measure_ | _measure_ |
| price-oracle | _measure_ | _measure_ |
| risk-management | _measure_ | _measure_ |
| trade-matching | _measure_ | _measure_ |
| trade-settlement | _measure_ | _measure_ |

> **Note:** Fill in actual sizes after running `./scripts/check-size.sh` on a
> clean release build. The CI will report sizes in its output on every run.

## Why this matters

Contract size regressions are silent failures — the code compiles and tests
pass, but the contract cannot be deployed. Complex contracts like
`matching-engine` and `governance` are closest to the limit; always check
sizes before merging changes that add significant logic.
