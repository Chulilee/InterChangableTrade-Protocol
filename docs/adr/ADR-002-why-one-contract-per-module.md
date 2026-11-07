# ADR-002: Why one contract per module

- **Status:** Accepted
- **Date:** 2026-11-08 (documented retroactively; decision dates to the start of the project)
- **Deciders:** InterChangableTrade contributors

## Context

The protocol covers distinct responsibilities: access control, escrowed deposits,
bilateral settlement, order matching, fees, incentives, oracle pricing, margin,
governance, and more. Soroban enforces a hard per-contract wasm size limit
(64 KiB) and per-invocation resource budgets, and each contract is deployed,
updated, and authorized independently on-chain.

A single monolithic contract would maximize compile-time coupling and minimize
deploy flexibility, but it would blow the size budget and force lockstep upgrades
of unrelated functionality.

## Decision

Ship **one crate per contract** under `contracts/<name>/`, as members of a single
Cargo workspace (`members = ["contracts/*", "integration-tests", "benchmarks"]`).
Each crate compiles to its own `.wasm`, is deployed on its own schedule, and owns
its storage under a per-contract `DataKey` enum.

## Consequences

**Positive**

- Each artifact stays small enough for the 64 KiB limit; `scripts/check-size.sh`
  can fail the build before deployment when a contract regresses.
- Independent deploys: `matching-engine` can be redeployed without touching
  `escrow`; a module that is still evolving (e.g. the legacy `orderbook`)
  doesn't block releases of the rest.
- Clear trust boundaries: `require_auth()` and role checks are scoped per
  contract, so a bug in one module cannot forge another's storage layout.
- Workspaces share dependencies via `[workspace.dependencies]` (one
  `soroban-sdk = "22"` everywhere) and share CI via a single workspace build.

**Negative / trade-offs**

- Cross-contract calls cost more than in-process calls and must be programmed
  against addresses baked in at initialization time (e.g. `price-oracle` needs the
  `access-control` address; `marketplace` needs
  `set_risk_manager` after deploy). Wiring is manual — see
  [docs/deployment.md](../deployment.md).
- Duplicated plumbing: each contract carries its own storage keys, id cursor, and
  error enum boilerplate. Generated scaffolding would help but was deferred.
- 16 separate deployments to verify instead of one; deployment order matters
  (dependencies first).
- Some contracts (`cross-chain-bridge`, `notification`, `trade-matching`) are
  built and tested standalone and not yet wired into the trading flow — the
  trade-off accepted in exchange for parallel, independent delivery.

## Alternatives considered

1. **One monolithic `interchangeable-trade` contract.** Maximum internal
   composability but violates the wasm size budget, couples all release cycles,
   and concentrates audit surface into one artifact.
2. **A small number of aggregated contracts** (e.g. "trading" + "treasury").
   Arbitrary boundaries; still risks size-limit pressure and slows independent
   deployment where it matters.
3. **Library-crate sharing of core logic across contracts.** Rejected for now:
   serialization boundaries between contracts are explicit and unavoidable, and a
   shared `common` crate would couple every contract's compile to its
   release cadence. Code is currently duplicated deliberately where the copies
   will diverge (error enums, storage keys).

## Related

- [docs/architecture.md](../architecture.md) — the 16 contracts and their roles
- ADR-001 (Soroban), ADR-003 (settlement vs escrow), ADR-004 (matching engine)
