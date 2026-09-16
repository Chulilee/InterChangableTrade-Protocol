# Architecture

InterChangableTrade-Protocol is a Soroban (Rust) workspace of smart
contracts. Each contract lives under `contracts/<name>/` and can be built and deployed on its own.

## Contracts

| Contract                | Responsibility                                                        |
|-------------------------|-----------------------------------------------------------------------|
| `access-control`        | Role-based access control (ADMIN, OPERATOR, PAUSER, GOVERNOR roles).  |
| `asset-registry`        | Admin-curated list of tokenized assets that are eligible to trade.    |
| `marketplace`           | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `escrow`                | Holds a buyer's deposit per trade; releases to seller or refunds buyer. |
| `trade-settlement`      | Atomic bilateral settlement with retry, batching, and a netting engine. |
| `fee-commission`        | Calculates, collects, and distributes protocol and maker/taker fees.  |
| `liquidity-incentives`  | Rewards liquidity providers over time via a reward-per-liquidity accumulator. |
| `orderbook`             | Minimal flat order list (legacy scaffold; see "Trading surfaces").    |
| `matching-engine`       | Price-time priority order book with partial fills and match simulation. |
| `price-oracle`          | Price feeds with deviation/freshness checks and secondary-source fallback. |
| `margining-liquidation` | Margin accounts, collateral, and liquidation of leveraged positions.  |
| `risk-management`       | Market pause switch and order-size limits; called by the marketplace. |
| `governance`            | Proposals, voting, timelock, and execution into a parameter store.    |

## Typical flow

```
        register asset            create listing            fund escrow
Admin  ───────────────────────▶ Registry   Seller ─────────▶ Marketplace   Buyer ─────────▶ Escrow
                                        │                            │
                                   fill listing                 open trade
                                        ▼                            ▼
                                     Buyer ───────────────────────▶ Settlement
                                                 settle / cancel
                                                      │
                                       release ◀──────┴──────▶ refund
                                          (Escrow moves funds)
```

1. **Registry** — an admin registers the assets and quote tokens that may be traded.
2. **Marketplace** — a seller lists an asset at a fixed price; a buyer fills it
   (the marketplace consults `risk-management` before accepting orders).
3. **Escrow** — the buyer funds an escrow for the agreed amount.
4. **Settlement** — a trade record tracks the exchange; on success the escrow is
   released to the seller, otherwise it is refunded to the buyer.

## Order-book flow

The `matching-engine` maintains a per-market (asset, quote) order book with
price-time priority, partial fills, and match simulation. Executed matches are
emitted as `tradeexec` events; on-chain settlement of matched trades is not yet
wired to `trade-settlement` (see "Open integration gaps").

`orderbook` is an earlier, simpler scaffold (a flat, unscoped order list) kept
for reference; `matching-engine` is the canonical order book going forward.

## Design notes

- Every contract is `#![no_std]` and depends only on `soroban-sdk`
  (`orderbook` predates this convention and is the exception).
- State is keyed with a per-contract `DataKey` enum; listings/trades/escrows use
  auto-incrementing `u64` ids.
- Mutating entry points call `require_auth()` on the relevant party, and every
  state transition publishes an event so off-chain indexers can follow along.
- Errors are returned via `#[contracterror]` enums rather than panics, so callers
  can use the generated `try_*` client methods.
- Units: fees use basis points (`10_000 bps = 100%`); margin ratios and reward
  accumulators use 18-decimal fixed point (`1e18`).

## Open integration gaps

The contracts above are individually functional but not yet wired
end-to-end. Known gaps, in rough priority order:

1. **Settlement paths are duplicated.** `trade-settlement` moves tokens itself
   (bilateral transfers, optionally netted per batch) and does not use
   `escrow`, which tracks deposits but never moves tokens. One of the two
   mechanisms should become the single settlement path.
2. **Matched orders are not settled.** The matching engine's settlement hook is
   a placeholder; orders can be marked `Filled` without any tokens moving.
3. **Fees are not attached to trades.** Nothing calls `fee-commission` when a
   trade settles.
4. **Authorization is fragmented.** Only `price-oracle` consults
   `access-control`; every other contract keeps its own local admin address,
   and the PAUSER/GOVERNOR roles are unused.
5. **Governance executes into a vacuum.** Proposals write
   `Parameter(Symbol) -> i128` entries that no contract reads.
6. **Margining does not use the oracle.** `margining-liquidation` stores a
   `price_oracle` address but computes collateral value and P&L from raw
   amounts and stored mark prices.