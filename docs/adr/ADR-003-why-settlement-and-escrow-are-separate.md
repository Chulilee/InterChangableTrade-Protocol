# ADR-003: Why settlement and escrow are separate

- **Status:** Accepted (with a known gap — see Consequences)
- **Date:** 2026-11-08 (documented retroactively; decision dates to the start of the project)
- **Deciders:** InterChangableTrade contributors

## Context

Trades in the protocol need two related but distinct guarantees:

1. **A buyer's funds must be locked before the seller commits.** This is the job
   of `escrow`: it records a buyer deposit per trade and can release or refund it.
2. **Two legs of a trade must happen atomically, or not at all.** This is the job
   of `trade-settlement`: it performs bilateral token transfers, optionally
   batched and netted, with retry on partial failure.

Combining both into one contract would conflate custody (holding deposits for an
arbitrary duration) with execution (moving tokens atomically at settlement time).

## Decision

Keep `escrow` (holds buyer deposits; releases to seller or refunds to buyer) and
`trade-settlement` (executes bilateral settlement with retry, batching and a
netting engine) as separate contracts with their own storage and lifecycles.

## Consequences

**Positive**

- Custody risk is isolated in `escrow`; a settlement logic bug cannot strand
  buyer deposits beyond its own failure mode, and the custody surface stays tiny
  and auditable.
- `trade-settlement` can evolve independently: retry with idempotent settlement
  attempts, batch settlement of many trades, and the netting engine that reduces
  the number of token transfers in high-volume periods — none of which require
  touching deposit accounting.
- Each contract enforces its own authorization model: escrow releases on trade
  outcome (release/refund), settlement requires the parties' `require_auth()` for
  the bilateral move.

**Negative / trade-offs (current known gap)**

- **The settlement paths are duplicated today.** `escrow` tracks deposits but
  never moves tokens itself, and `trade-settlement` moves tokens without using
  `escrow`. This is called out in [docs/architecture.md](../architecture.md)
  ("Settlement paths are duplicated", open integration gap #1) and is the main
  consequence of this decision: consistency between the two mechanisms is by
  convention, not by construction.
- Releasing through `escrow` after a `trade-settlement` batch nets positions adds
  an extra hop and must be orchestrated off-chain until the wiring is finished.
- Planned direction: converge on one settlement path (see
  [docs/architecture.md](../architecture.md) "Open integration gaps"); this ADR
  records the separation of concerns, not the final wiring.

## Alternatives considered

1. **Escrow inside settlement.** Fewer cross-contract calls and one atomic path,
   but couples custody and execution: every settlement change would require
   re-auditing deposit handling, and netting would force escrow to understand
   batch semantics.
2. **Settlement inside escrow.** Simplest mental model for a single trade, but
   then escrow must understand bilateral transfers and netting — bloating the most
   security-critical, longest-lived state in the system.
3. **No escrow at all (pure atomic swap).** Rejected: sellers will not release
   assets on a promise; a deposit-escrow step is required for adverse-selection
   control before settlement.

## Related

- [docs/architecture.md](../architecture.md) — Open integration gaps #1
- ADR-001 (Soroban), ADR-002 (one contract per module)
