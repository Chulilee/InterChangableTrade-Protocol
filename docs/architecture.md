# Architecture

InterChangableTrade-Protocol is a Soroban (Rust) workspace of four
independent-but-composable smart contracts. Each contract lives under
`contracts/<name>/` and can be built and deployed on its own.

## Contracts

| Contract           | Responsibility                                                        |
|--------------------|-----------------------------------------------------------------------|
| `asset-registry`   | Admin-curated list of tokenized assets that are eligible to trade.    |
| `marketplace`      | Fixed-price listings: sellers offer assets, buyers fill, sellers cancel. |
| `escrow`           | Holds a buyer's deposit per trade; releases to seller or refunds buyer. |
| `trade-settlement` | Records a trade's lifecycle and drives it to settled/cancelled.       |

## Typical flow

```
        register asset            create listing            fund escrow
Admin  ───────────────▶ Registry   Seller ─────▶ Marketplace   Buyer ─────▶ Escrow
                                        │                            │
                                   fill listing                 open trade
                                        ▼                            ▼
                                     Buyer ───────────────────▶ Settlement
                                                 settle / cancel
                                                      │
                                       release ◀──────┴──────▶ refund
                                          (Escrow moves funds)
```

1. **Registry** — an admin registers the assets and quote tokens that may be traded.
2. **Marketplace** — a seller lists an asset at a fixed price; a buyer fills it.
3. **Escrow** — the buyer funds an escrow for the agreed amount.
4. **Settlement** — a trade record tracks the exchange; on success the escrow is
   released to the seller, otherwise it is refunded to the buyer.

## Design notes

- Every contract is `#![no_std]` and depends only on `soroban-sdk`.
- State is keyed with a per-contract `DataKey` enum; listings/trades/escrows use
  auto-incrementing `u64` ids.
- Mutating entry points call `require_auth()` on the relevant party, and every
  state transition publishes an event so off-chain indexers can follow along.
- Errors are returned via `#[contracterror]` enums rather than panics, so callers
  can use the generated `try_*` client methods.

These are MVP scaffolds: the token transfers themselves (registry ↔ escrow ↔
settlement wiring) are intentionally left as the next implementation step.
