# Migrating from Legacy `orderbook` to `matching-engine`

> **Status:** `contracts/orderbook` is **deprecated** and receives no new
> features or production wiring. All new trading integrations, indexers, and
> settlement hooks must target `contracts/matching-engine`.

## Background

The `orderbook` crate was the initial prototype scaffold: a single flat,
unscoped `Vec<Order>` stored under `symbol_short!("ORDERS")` with `u64` prices
and quantities and no matching logic.

`matching-engine` replaced it as the canonical order book for the protocol
(see [ADR-004](adr/ADR-004-why-matching-engine-replaced-orderbook.md)), adding:

- **Per-market isolation** keyed by `(asset: Address, quote: Address)`.
- **Price-time priority (FIFO)** across sorted bid (`descending`) and ask
  (`ascending`) `PriceLevel` queues.
- **Partial and multi-fill matching** bounded per call (`MAX_TRADES_PER_CALL = 10`)
  with resumable `MatchResult.more_available`.
- **Read-only match simulation** (`simulate_match`) for pre-trade price/fill
  estimation.

*(Note: deduplicating the token-custody settlement path between `escrow` and
`trade-settlement` is a separate track covered by issue #62 and
[ADR-006](adr/ADR-006-canonical-settlement-path.md).)*

---

## Entry-Point Migration Mapping

| Operation | Legacy `orderbook` (`OrderbookContract`) | Canonical `matching-engine` (`MatchingEngine`) | Migration Notes |
| --- | --- | --- | --- |
| **Initialize** | *(none)* | `initialize(env, admin: Address) -> Result<(), Error>` | Must be called once before any orders are placed; requires `admin.require_auth()`. |
| **Place order** | `place_order(env, owner: Address, side: OrderSide, price: u64, quantity: u64) -> u64` | `place_order(env, trader: Address, asset: Address, quote: Address, side: OrderSide, price: i128, quantity: i128) -> Result<u64, Error>` | Pass `asset` and `quote` token addresses; convert `price` and `quantity` to `i128` (`> 0`); handle `Result<u64, Error>`. |
| **Cancel order** | `cancel_order(env, owner: Address, order_id: u64) -> Result<(), Error>` | `cancel_order(env, asset: Address, quote: Address, order_id: u64, caller: Address) -> Result<(), Error>` | Pass the `(asset, quote)` market pair and `caller` (`trader`). Removes unfilled quantity from its resting `PriceLevel`. |
| **Update order** | `update_order(env, owner: Address, order_id: u64, new_price: u64, new_quantity: u64) -> Result<(), Error>` | `cancel_order(...)` followed by `place_order(...)` | In-place mutation is intentionally omitted in `matching-engine` so price/size changes cannot jump the time-priority queue. |
| **Match orders** | *(not supported)* | `match_orders(env, asset: Address, quote: Address) -> Result<MatchResult, Error>` | Executes up to `MAX_TRADES_PER_CALL` crossable trades per call; re-invoke while `MatchResult.more_available` is `true`. |
| **Simulate match** | *(not supported)* | `simulate_match(env, asset: Address, quote: Address, incoming_order: Order) -> Result<MatchResult, Error>` | Evaluates fills and `remaining_quantity` against a cloned book without mutating state. |
| **Get order by ID** | `get_order_by_id(env, order_id: u64) -> Option<Order>` | `get_order(env, asset: Address, quote: Address, order_id: u64) -> Result<Order, Error>` | Scoped to `(asset, quote)`; returns `Err(Error::OrderNotFound)` or `Err(Error::MarketNotFound)` instead of `None`. |
| **List owner orders / best bid-ask** | `list_orders_by_owner(env, owner)` / `get_best_bid_ask(env)` | Inspect `MarketOrderBook.bids` / `asks` / `all_orders` or index events off-chain | Best bid is `bids.get(0)` (highest buy price); best ask is `asks.get(0)` (lowest sell price). |

---

## Data Types & Storage Keys Mapping

| Concept | Legacy `orderbook` | Canonical `matching-engine` |
| --- | --- | --- |
| **Order actor field** | `order.owner: Address` | `order.trader: Address` |
| **Market scope** | Unscoped (single global list) | `order.asset: Address`, `order.quote: Address` |
| **Price & quantity types** | `u64` (`price`, `quantity`, `filled`) | `i128` (`price`, `quantity`, `filled`) |
| **Order status enum** | `OrderStatus::{New, Filled, Cancelled}` | `OrderStatus::{Open, PartiallyFilled, Filled, Cancelled}` |
| **Storage keys** | Raw symbols: `ORDERS`, `NEXT_ID` (persistent) | `DataKey::Admin` (instance), `DataKey::NextOrderId` (instance), `DataKey::NextTradeId` (instance), `DataKey::OrderBook((Address, Address))` (persistent) |
| **Event topics** | `("order", "placed")`, `("order", "cancelled")`, `("order", "updated")` | `("placed", order_id)`, `("cancelled", order_id)`, `("partial", order_id)`, `("filled", order_id)`, `("executed", trade_id)` |

---

## Feature Flag & Retirement Plan

`contracts/orderbook/Cargo.toml` defines a `legacy-orderbook` Cargo feature
(enabled by default so workspace CI and historical benchmarks continue to
compile and run). Downstream workspace tooling can opt out with
`default-features = false` prior to the crate's eventual removal once all
consumers have migrated to `matching-engine`.
