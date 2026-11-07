# ADR-005: Why basis points for fee calculation

- **Status:** Accepted
- **Date:** 2026-11-08 (documented retroactively; decision dates to the start of the project)
- **Deciders:** InterChangableTrade contributors

## Context

`fee-commission` calculates, collects, and distributes protocol, maker and taker
fees; `margining-liquidation` and `liquidity-incentives` similarly deal in ratios.
Soroban contracts are deterministic and `#![no_std]`: there is no floating point,
and any arithmetic that depends on rounding must produce identical results for
every validator that re-executes the transaction.

Fees are typically small fractions (fractions of a percent), so the representation
must preserve at least four decimal places of percent-level precision.

## Decision

Express all fee and ratio parameters as integers:

- **Basis points** for fees and ratios whose natural scale is percent, with
  `BPS_DENOMINATOR = 10_000` (10_000 bps = 100%). Maker/taker fees are capped at
  `MAX_FEE_BPS = 1_000` (10%); protocol fee is capped at the full denominator.
- **18-decimal fixed point** (`1e18`) for margin ratios and the
  reward-per-liquidity accumulator, where sub-bps precision is needed.
- All math in `i128`; intermediate products are divided by the denominator
  last, so no intermediate rounding occurs.

## Consequences

**Positive**

- Fully deterministic: integer math on `i128` produces identical results on every
  validator and in tests, which is mandatory for consensus.
- No precision surprises for fees: bps is exact for 4-decimal percent precision,
  and `1e18` fixed point covers reward-per-liquidity accumulators without f64.
- Saturation behavior is explicit; `checked_*`/`overflow-checks = true` (release
  profile) turns overflow into a trap instead of silent corruption.
- Human-auditable configs: `maker_fee_bps = 30` reads as 0.30% and round-trips
  exactly through `initialize` and `get_config`.

**Negative / trade-offs**

- Percentages never fit in "nice" base-10 floats mentally; integrators must carry
  the denominator convention themselves (docs and `initialize` help, but mistakes
  happen — see the [docs/deployment.md](../deployment.md) troubleshooting table).
- Bps caps create hard ceilings (10% maker/taker); raising them later is a
  contract upgrade, not a parameter change.
- Amount × bps overflows `i128` only at astronomically large values, but
  accumulator math in rewards does wide multiplication and must stay on the
  audited path (`RewardPerLiquidity` accumulator pattern), not ad-hoc arithmetic.

## Alternatives considered

1. **IEEE-754 floats (`f64`).** Rejected: non-deterministic across platforms,
   unavailable in `no_std`, and consensus-forbidden.
2. **`percent` as 1e6 fixed point (6 decimals).** Works, but bps is the DeFi
   convention and maps directly onto the 10000 denominator used for recipient
   share validation (`set_recipients` requires shares summing to exactly
   `10_000`), keeping one constant everywhere.
3. **Storing fees as fractions `numerator/denominator` per fee.** Flexible but
   doubles the config surface, complicates bounds checking, and breaks the
   "sum of recipient shares == 100%" invariant checks.
4. **Decimal strings.** Off-chain convenience at the cost of on-chain parsing
   complexity and unbounded input size.

## Related

- [docs/architecture.md](../architecture.md) — "Design notes" (bps / 1e18 units)
- ADR-001 (Soroban), ADR-003 (settlement vs escrow)
