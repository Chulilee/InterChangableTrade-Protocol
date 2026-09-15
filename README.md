# InterChangableTrade-Protocol

> Soroban smart contracts powering decentralized trading on Stellar.

## Overview

InterChangableTrade-Protocol contains the on-chain logic for the InterChangableTrade ecosystem. It implements the core business rules that enable secure, transparent, and decentralized trading of tokenized assets using Soroban smart contracts.

## Contracts

The workspace contains 14 composable Soroban contracts under `contracts/`:

| Contract | Responsibility |
|----------|----------------|
| `access-control` | Role-based permissions (admin, operator, pauser, governor) with grant/revoke/renounce. |
| `asset-registry` | Admin-curated list of tokenized assets eligible to trade. |
| `escrow` | Custodies a buyer's deposit per trade; releases to seller or refunds buyer via real token transfers. |
| `fee-commission` | Calculates, collects, and distributes protocol and maker/taker fees. |
| `governance` | On-chain proposal and voting lifecycle. |
| `liquidity-incentives` | Rewards liquidity providers via a reward-per-liquidity accumulator. |
| `margining-liquidation` | Margin accounts, collateral, maintenance margins, and oracle-driven liquidation. |
| `marketplace` | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `matching-engine` | Price-time-priority order matching that emits authoritative trade records. |
| `orderbook` | On-chain order book storage with bid/ask price levels. |
| `price-oracle` | Publishes and serves mark prices used for margin and settlement. |
| `risk-management` | Market pause switch, per-order size limits, and cumulative exposure checks. |
| `trade-matching` | Tag/category/value-range listing matching with ranked suggestions and propose/accept workflow. |
| `trade-settlement` | Atomic, retryable settlement with position netting across a batch of trades. |

## Technology Stack

- Rust
- Soroban SDK 22
- Stellar CLI

## Project Structure

```
contracts/   # the 13 Soroban contracts listed above
docs/        # architecture and design notes
scripts/     # build/deploy helpers
```

## Getting Started

```bash
git clone https://github.com/OthmanImam/InterChangableTrade-Protocol.git

cd InterChangableTrade-Protocol

# Build every contract in the workspace
cargo build

# Run the full test suite
cargo test --workspace

# Build the deployable WASM for one contract
cargo build -p escrow --target wasm32-unknown-unknown --release
```
## Related Repositories

- **[InterChangableTrade-Core](https://github.com/Chulilee/InterChangableTrade-Core)** 
  Soroban smart contracts (access-control, escrow, marketplace, etc.)
- **[InterChangableTrade-Fricks](https://github.com/Chulilee/InterChangableTrade-Fricks)** 

  
## Contributing

Contributions, security reviews, and audits are highly encouraged. See
[`docs/architecture.md`](docs/architecture.md) for how the contracts fit together.

## License

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE).
