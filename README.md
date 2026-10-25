# InterChangableTrade-Protocol

> Soroban smart contracts powering decentralized trading on Stellar.

## Overview

InterChangableTrade-Protocol contains the on-chain logic for the InterChangableTrade ecosystem. It implements the core business rules that enable secure, transparent, and decentralized trading of tokenized assets using Soroban smart contracts.

## Contracts

The workspace contains **16** standalone contracts, one crate per contract under
`contracts/`. See [docs/architecture.md](docs/architecture.md) for how they fit
together and for the known integration gaps.

| Contract | Responsibility |
| --- | --- |
| `access-control` | Role-based access control (ADMIN, OPERATOR, PAUSER, GOVERNOR roles). |
| `asset-registry` | Admin-curated list of tokenized assets eligible to trade. |
| `marketplace` | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `escrow` | Holds a buyer's deposit per trade; releases to seller or refunds buyer. |
| `trade-settlement` | Atomic bilateral settlement with retry, batching, and netting. |
| `fee-commission` | Calculates, collects, and distributes protocol and maker/taker fees. |
| `liquidity-incentives` | LP rewards via a reward-per-liquidity accumulator. |
| `orderbook` | Minimal flat order list (legacy scaffold; see the architecture doc). |
| `matching-engine` | Price-time priority order book with partial fills and match simulation. |
| `price-oracle` | Price feeds with deviation/freshness checks and fallback source. |
| `margining-liquidation` | Margin accounts, collateral, and liquidation of leveraged positions. |
| `risk-management` | Market pause switch and order-size limits. |
| `governance` | Proposals, voting, timelock, and execution into a parameter store. |
| `cross-chain-bridge` | Asset locking, validator attestations, and emergency withdrawals for cross-chain transfers. |
| `notification` | Per-user on-chain notifications with read/unread state and email preferences. |
| `trade-matching` | Matches listings by category, tags, and price range; ranked trade suggestions. |

## Technology Stack

- Rust
- Soroban SDK
- Stellar CLI

## Project Structure

```
contracts/   one crate per contract (see docs/architecture.md)
docs/        architecture and per-module documentation
scripts/     build / test / deploy helpers
```

## Getting Started

```bash
git clone https://github.com/Chulilee/InterChangableTrade-Protocol.git

cd InterChangableTrade-Protocol

cargo build

cargo test --workspace
```

## Related Repositories

- **[InterChangabletrade-fricks](https://github.com/Chulilee/InterChangabletrade-fricks)** —
  Next.js frontend for trading tokenized assets on Stellar.
- **[InterChangableTrade-Core](https://github.com/Chulilee/InterChangableTrade-Core)** —
  application services, APIs, and blockchain indexer.

## Contributing

Contributions, security reviews, and audits are highly encouraged. See
[docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## License

Apache-2.0
<!-- Updated documentation references -->
