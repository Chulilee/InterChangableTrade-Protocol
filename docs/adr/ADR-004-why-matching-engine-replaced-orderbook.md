# ADR-004: Why the matching engine replaced the orderbook

- **Status:** Accepted
- **Date:** 2026-11-08 (documented retroactively; the matching engine became the canonical book with the price-time iteration)
- **Deciders:** InterChangableTrade contributors

## Context

The protocol needs to match buyers and sellers of tokenized assets. The first
iteration, `orderbook`, was a **minimal flat order list**: orders appended to one
unscoped list with basic id-based access. It served as a scaffold to prove the
token/event plumbing, but it has no price-time priority, no partial fills, no
market scoping, and no execution semantics.

`matching-engine` was built as the production book: per-market (asset, quote)
books with price-time priority, partial fills, match simulation, and `executed`
events. The README and architecture doc both list `matching-engine` as the
canonical order book and `orderbook` as a legacy scaffold kept for reference.

## Decision

`matching-engine` is the canonical order book for the protocol. `orderbook`
remains in the workspace for reference and benchmarking baselines, receives no
new features, and must not be wired into production settlement flows.

Concretely:

- New order flows, events, and fill logic go in `matching-engine` only.
- Trading surfaces integrate against `matching-engine`.
- `orderbook` is not deleted outright so historical benchmarks and the original
  scaffold remain runnable.

## Consequences

**Positive**

- Real matching semantics: price-time priority and partial fills are table
  stakes for any credible exchange, and they enable deterministic settlement
  economics on-chain.
- Per-market isolation (`(asset, quote)` books) prevents cross-market
  order leakage and keeps event streams attributable per market.
- Match simulation lets callers price a trade before submitting, which the
  frontend and off-chain tooling depend on.
- Capped per-call work with resumable `MatchResult` keeps the matcher within
  Soroban resource budgets.

**Negative / trade-offs**

- Two order-book codebases now coexist until `orderbook` is retired, which is a
  maintenance and audit burden. New contributors frequently confuse them — this
  ADR and the README table exist to prevent that.
- The matcher's settlement hook is still a placeholder: orders can be marked
  `Filled` without tokens moving, so `matching-engine` is not yet authoritative
  for value transfer (see [docs/architecture.md](../architecture.md), open
  integration gap #2). Matching being "canonical" refers to book semantics, not
  to completed settlement.
- Removing `orderbook` later requires updating benchmarks and docs that reference
  it.

## Alternatives considered

1. **Extend the legacy `orderbook` in place.** Reuse wins but bakes in the flat,
   unscoped list model; retrofitting priority, partial fills and market scoping
   onto it would have been a rewrite with the old API's warts attached.
2. **Off-chain order book with on-chain settlement.** Faster and cheaper per
   match, but requires a trusted sequencer and weakens custody guarantees; kept
   as a future scaling path, not the baseline.
3. **AMM-style matching.** Rejected: fixed-price marketplace semantics and
   an order book are both first-class here; constant-product pricing does not fit
   tokenized-asset trading with discrete asks/bids.

## Related

- [docs/architecture.md](../architecture.md) — "Order-book flow", open integration gap #2
- ADR-001 (Soroban), ADR-002 (one contract per module)
