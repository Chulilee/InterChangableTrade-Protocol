# InterChangableTrade-Protocol

> Soroban smart contracts powering decentralized trading on Stellar.

## Overview

InterChangableTrade-Protocol contains the on-chain logic for the InterChangableTrade ecosystem. It implements the core business rules that enable secure, transparent, and decentralized trading of tokenized assets using Soroban smart contracts.

## Features

- Asset Registry
- Marketplace Contract (fixed-price listings)
- Escrow Contract
- Trade Settlement (atomic, batched with netting, retryable)
- Fee & Commission System
- Liquidity Incentives (LP rewards)
- Order Book & Matching Engine (price-time priority, partial fills)
- Price Oracle (deviation/freshness checks, fallback source)
- Margining & Liquidation
- Risk Management (market pause, order-size limits)
- Access Control (role-based)
- Governance (proposals, voting, timelock)
- Contract Events
- Stellar Asset Support

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
git clone https://github.com/InterChangableTrade/InterChangableTrade-Protocol.git

cd InterChangableTrade-Protocol

cargo build

cargo test --workspace
```

## Related Repositories

- InterChangableTrade-Fricks
- InterChangableTrade-Core

## Contributing

Contributions, security reviews, and audits are highly encouraged. See
[docs/CONTRIBUTING.md](docs/CONTRIBUTING.md).

## License

Apache-2.0