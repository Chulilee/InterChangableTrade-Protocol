# ADR-006: Canonical settlement path and staged migration plan

- **Status:** Accepted
- **Date:** 2026-11-09 (documented retroactively; decision dates to architecture review)
- **Deciders:** InterChangableTrade contributors

## Context

Two contracts participate in moving value on a trade:

- `escrow` — records a buyer deposit per trade and releases or refunds it.
  It never moves tokens itself; the token transfer stays on the record as an
  accounting entry.
- `trade-settlement` — executes bilateral token transfers for a trade, with
  retry semantics, batching, and a netting engine. It moves tokens directly
  via the token client.

A fixed-price marketplace flow needs both: the buyer must lock funds before
the seller commits, and those funds must move atomically at settlement time.
Today both contracts exist and both are partially wired:

1. The marketplace lists/fills without moving tokens; settlement is invoked
   separately.
2. `trade-settlement` records trades, transfers tokens bilaterally, and
   exposes retry and batch APIs. It does not use `escrow`'s accounting.
3. `escrow` tracks deposits and releases/refunds, but nothing calls
   `fee-commission` when a trade settles, and nothing calls `trade-settlement`
   from `escrow` release/refund paths.

This duplication is tracked in the architecture docs ("Settlement paths are
duplicated") and in issues #62 and related backlog items. Redesigning both
contracts end-to-end is too large to do safely in one pass.

## Decision

**The canonical settlement path is `trade-settlement`.** For any atomic trade
involving asset movement, `trade-settlement` is the authority: it validates
balances, executes bilateral transfers, records state, emits events, and
supports failed-settlement retry and batched netting. `escrow` remains
scoped to **buyer-deposit accounting** for flows that need pre-locked funds,
but it is no longer considered a token-movement path; any token movement for
an escrow-mediated trade should delegate to `trade-settlement` (or be
clearly documented as a pending migration).

The intended end-to-end flow:

```
registry  ->  listing/marketplace  ->  escrow (deposit lock)  ->  settlement  ->  release/refund
                |                              |                       |
                +-------- risk-management <----+                       +--> fee-commission
```

1. **Registry** — an admin registers tradeable assets and quote tokens.
2. **Marketplace / fixed-price listings** — seller lists an asset; buyer
   fills. The marketplace calls `check_limits` on the risk-management
   contract before accepting listings/fills and returns typed errors when the
   risk manager or wiring is missing (see issue #88).
3. **Escrow** — the buyer locks funds for the agreed amount before committing
   to the trade (deposit accounting only).
4. **Settlement** — the actual exchange happens in `trade-settlement`, which
   executes the bilateral transfer and emits a `SettlementCompleted` event.
5. **Treasury / fees** — `fee-commission` accrues and distributes protocol
   and maker/taker fees; distribution is permissionless because recipients
   and the treasury are fixed by governance.
6. **Release / refund** — on success or cancellation, the escrow record is
   marked released or refunded. Release/refund must not move tokens while the
   settlement path is authoritative for transfers; the escrow record is the
   source of truth for the deposit's disposition.

`escrow` may continue to exist as a standalone deposit-accounting contract
for the marketplace flow, with this scope limitation documented in
[docs/architecture.md](../architecture.md). It must not become a second,
divergent settlement implementation.

## Consequences

**Positive**

- One contract owns atomic transfers: auditable, testable, and the only place
  `token::Client::transfer` is called in the trade path.
- Retry and batching live in one place (`settle_trade` / `settle_batch_netted`),
  not in two.
- The marketplace + `check_limits` + `settle_trade` chain is a coherent
  end-to-end flow for fixed-price trades.

**Negative / trade-offs**

- The migration is not free: callers that currently use `escrow` to move
  funds must switch to `trade-settlement` (or accept the interim scope).
- Until the migration completes, `escrow`-mediated trades do **not** go
  through `trade-settlement`; this is documented in
  [docs/architecture.md](../architecture.md) and in issue #62.
- Fees are not yet attached to settlements (issue #108 / backlog); once the
  canonical path is unambiguous, wiring `fee-commission` becomes a one-way,
  auditable change rather than a guess.

## Migration plan (staged)

1. **Enforce the canonical path contract.** Callers may only invoke transfers
   through `trade-settlement`. Any remaining `escrow`-as-transfer code path is
   removed or clearly marked `// deprecated — use trade-settlement`.
2. **Document the temporary exception.** `escrow` continues to record deposits
   and releases/refunds for the marketplace flow; the exception and its
   migration date are recorded in the architecture doc and issue #62.
3. **Wire fees to the settlement completion.** Once the flow is unambiguous,
   add a settlement-time hook that accrues fees (issue #108 / related).
4. **Consolidate.** When every caller uses `trade-settlement` for asset
   movement, deprecate `escrow`'s token-moving surface entirely or retire it
   in favor of a settlement-batched escrow ledger.

## Alternatives considered

1. **Escrow-first.** Escrow holds funds and releases them; settlement executes
   the exchange and notifies escrow. Rejected: requires escrow to understand
   settlement retry/rollback semantics, and the central risk is duplicated
   atomicity logic — centralizing in `trade-settlement` avoids that.
2. **Two parallel paths forever.** Rejected: doubles the audit surface and
   invites divergence.
3. **Merge both into one contract.** Rejected: custody and execution have
   different lifetimes and authorization models; keeping them separate as
   contracts with a single authority for transfers is smaller and safer.

## Related

- ADR-003 (settlement and escrow are separate contracts)
- [docs/architecture.md](../architecture.md) — "Open integration gaps"
- Issues #62, #91, #94, #99, #108
