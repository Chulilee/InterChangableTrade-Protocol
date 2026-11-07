# Architecture Decision Records

This directory contains the Architecture Decision Records (ADRs) for
InterChangableTrade-Protocol. Each ADR records one significant decision: the
context in which it was made, the decision itself, the consequences (good and
bad), and the alternatives that were considered.

ADRs are numbered sequentially (`ADR-NNN`) and never reused. A decision that
changes hands is superseded by a new ADR, which references the old one; old
records are kept for history.

| ADR | Title | Status |
|---|---|---|
| [ADR-001](ADR-001-why-soroban-for-smart-contracts.md) | Why Soroban for smart contracts | Accepted |
| [ADR-002](ADR-002-why-one-contract-per-module.md) | Why one contract per module | Accepted |
| [ADR-003](ADR-003-why-settlement-and-escrow-are-separate.md) | Why settlement and escrow are separate | Accepted (with known gap) |
| [ADR-004](ADR-004-why-matching-engine-replaced-orderbook.md) | Why the matching engine replaced the orderbook | Accepted |
| [ADR-005](ADR-005-why-basis-points-for-fee-calculation.md) | Why basis points for fee calculation | Accepted |

## Format

Each ADR follows the same structure:

- **Title** — `ADR-NNN: <the decision as a question answered>`
- **Status** — Proposed / Accepted / Superseded (with link to the superseding ADR)
- **Date** — when the ADR was recorded (retroactive dates are noted explicitly)
- **Deciders**
- **Context** — what forced the decision
- **Decision** — what was decided
- **Consequences** — positive and negative, including known gaps
- **Alternatives considered** — and why they were rejected
