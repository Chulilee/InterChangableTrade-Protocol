# Architecture

InterChangableTrade-Protocol is a Soroban (Rust) workspace of 13
independent-but-composable smart contracts. Each contract lives under
`contracts/<name>/`, is `#![no_std]`, depends only on `soroban-sdk`, and can be
built and deployed on its own.

## Contracts

| Contract | Responsibility |
|----------|----------------|
| `access-control` | Role-based permissions (admin, operator, pauser, governor) with grant/revoke/renounce and per-role admin roles. |
| `asset-registry` | Admin-curated list of tokenized assets that are eligible to trade. |
| `escrow` | Custodies a buyer's deposit per trade and moves real tokens on release/refund. |
| `fee-commission` | Calculates, collects, and distributes protocol and maker/taker fees. |
| `governance` | On-chain proposal and voting lifecycle. |
| `liquidity-incentives` | Rewards liquidity providers over time via a reward-per-liquidity accumulator. |
| `margining-liquidation` | Margin accounts, collateral tracking, maintenance margins, and oracle-driven liquidation. |
| `marketplace` | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `matching-engine` | Price-time-priority order matching that emits authoritative trade records. |
| `orderbook` | On-chain order book storage with bid/ask price levels. |
| `price-oracle` | Publishes and serves mark prices consumed by margining and settlement. |
| `risk-management` | Market pause switch, per-order size limits, and cumulative exposure checks. |
| `trade-matching` | Tag/category/value-range listing matching with ranked suggestions and propose/accept workflow. |
| `trade-settlement` | Atomic, retryable settlement with position netting across a batch of trades. |

## How value moves

Two contracts move real tokens via the Soroban `token` interface; the rest are
coordination, bookkeeping, and risk layers around them.

```
                 register asset            create listing
   Admin  ─────────────────────▶ Registry   Seller ─────────▶ Marketplace
                                                                   │
                                                              fill listing
                                                                   ▼
   Buyer ──── fund (pulls tokens) ────▶ Escrow            Matching Engine
                                          │                        │
                                   release / refund         emits trade record
                                   (pushes tokens)                 │
                                          ▼                        ▼
                                       Seller / Buyer       Trade Settlement
                                                          (atomic token::transfer,
                                                           batch netting, retry)
```

1. **Registry** — an admin registers the assets and quote tokens that may be
   traded.
2. **Marketplace / Matching Engine** — a seller lists an asset at a fixed price
   and a buyer fills it, or the matching engine crosses resting orders by
   price-time priority and publishes a trade record.
3. **Escrow** — `fund` transfers the buyer's deposit into the contract's own
   balance; `release` pushes it to the seller and `refund` returns it to the
   buyer. Because a Soroban transaction is atomic, a failed token transfer rolls
   back the accompanying state transition — an escrow can never read `Released`
   without the funds having moved.
4. **Trade Settlement** — executes the base/quote token transfers for a trade
   atomically, nets obligations across a batch to minimize transfers, and
   supports retry after a transient failure (e.g. a buyer topping up balance).

## Margin & risk

- **Margining & Liquidation** tracks collateral and open positions. The
  configured **Price Oracle** pushes mark prices via `update_mark_price`; when a
  position's unrealized P&L pushes an account below the maintenance margin, any
  liquidator can `trigger_liquidation` and earn the configured incentive.
- **Risk Management** gates order flow: a pauser can halt the market, cap
  per-order size, and reject orders that would breach limits.

## Design notes

- State is keyed with a per-contract `DataKey` enum; listings, trades, escrows,
  and positions use auto-incrementing `u64` ids.
- Mutating entry points call `require_auth()` on the relevant party, and every
  state transition publishes an event so off-chain indexers can follow along.
- Errors are returned via `#[contracterror]` enums rather than panics, so callers
  can use the generated `try_*` client methods.
- Every contract ships with unit tests exercised in CI across the full matrix
  (`.github/workflows/contracts-check.yml`), including `cargo fmt`, `clippy -D
  warnings`, a WASM release build, and the test suite.
