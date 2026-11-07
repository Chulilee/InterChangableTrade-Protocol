# ADR-001: Why Soroban for smart contracts

- **Status:** Accepted
- **Date:** 2026-11-08 (documented retroactively; decision dates to the start of the project)
- **Deciders:** InterChangableTrade contributors

## Context

InterChangableTrade-Protocol needs on-chain business logic for trading tokenized
assets on Stellar: escrow of buyer deposits, price-time priority matching, fee
accrual, and settlement. The contracts must be cheap to execute for every trade,
deployable and updatable by a small team, and composable with the Stellar Asset
Contract so collateral can be held in real token custody.

The Stellar ecosystem ships Soroban as its general-purpose smart-contract
platform: contracts compile from Rust to WebAssembly and run on the Stellar
blockchain under per-transaction CPU/memory budgets.

## Decision

Implement all on-chain logic as Soroban contracts written in Rust
(`#![no_std]`, depending only on `soroban-sdk`), compiled to
`wasm32-unknown-unknown` with `opt-level = "z"`, and deployed with the Stellar CLI.

## Consequences

**Positive**

- Native access to Stellar primitives: the Stellar Asset Contract (via
  `soroban_sdk::token::Client`), contract events for off-chain indexers, and
  1-transaction settlement paths — no bridges or wrapped sidechains required.
- Deterministic, metered execution: per-invocation resource budgets align with a
  fee market and keep trades economically viable.
- Rust's type system and exhaustive `match` on contract error enums push many
  failure modes to compile time, and `#[contracterror]` enums keep runtime
  failures typed for callers.
- `rust-toolchain.toml` pins the toolchain, so CI and contributor machines
  produce identical wasms.

**Negative / trade-offs**

- Soroban contracts must live within a per-contract wasm size limit (64 KiB) and
  strict CPU/memory budgets, which constrains algorithm complexity and rules out
  some off-the-shelf crypto or math libraries.
- Smaller hiring pool and younger ecosystem than EVM tooling; debugging requires
  the Stellar CLI and testnet iteration rather than browser devtools.
- `no_std` eliminates the standard library; anything needed from `core` or `alloc`
  must be justified explicitly.

## Alternatives considered

1. **Off-chain matching with on-chain settlement only.** Simpler and faster off
   the shelf, but moves trust into the sequencer and weakens the "no custodial
   component" property the protocol wants for escrow and settlement.
2. **EVM chain (e.g. an EVM-compatible L2) with Stellar bridge.** Mature tooling
   but introduces a separate trust domain, bridge risk, and fragmented liquidity
   for Stellar-native assets.
3. **CosmWasm.** Closest prior art (Rust-to-wasm), but not native to Stellar and
   would isolate the protocol from Stellar asset liquidity.

## Related

- [docs/architecture.md](../architecture.md)
- ADR-002 (one contract per module), ADR-003 (settlement vs escrow)
