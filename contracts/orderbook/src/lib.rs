#![no_std]
#![deny(warnings)]

//! # Orderbook Contract (Deprecated)
//!
//! **Deprecated:** `orderbook` is a legacy flat order-list scaffold and is no
//! longer maintained or wired into production trading flows. Use the
//! `matching-engine` contract (`contracts/matching-engine`) instead.
//!
//! See [ADR-004 (`docs/adr/ADR-004-why-matching-engine-replaced-orderbook.md`)](../../docs/adr/ADR-004-why-matching-engine-replaced-orderbook.md)
//! for the architectural decision and
//! [`docs/orderbook-migration.md`](../../docs/orderbook-migration.md) for the
//! complete entry-point, data-key, type, and event migration guide.
//! (Note: settlement-path deduplication between `escrow` and `trade-settlement`
//! is tracked separately under issue #62 and ADR-006.)
//!
//! ## Migration Summary (`orderbook` -> `matching-engine`)
//!
//! | Legacy `orderbook` | Canonical `matching-engine` | Key Differences |
//! | --- | --- | --- |
//! | *(no initializer)* | `initialize(env, admin) -> Result<(), Error>` | One-time admin initialization required before placing orders. |
//! | `place_order(env, owner, side, price: u64, quantity: u64) -> u64` | `place_order(env, trader, asset, quote, side, price: i128, quantity: i128) -> Result<u64, Error>` | Market-scoped by `(asset, quote)`; uses signed `i128` amounts/prices; validates positive price/quantity. |
//! | `cancel_order(env, owner, order_id) -> Result<(), Error>` | `cancel_order(env, asset, quote, order_id, caller) -> Result<(), Error>` | Requires `(asset, quote)` market key; removes remaining quantity from the resting `PriceLevel`. |
//! | `update_order(env, owner, order_id, new_price, new_quantity)` | `cancel_order(...)` + `place_order(...)` | Cancel-and-replace preserves price-time (FIFO) priority at each `PriceLevel`. |
//! | *(no matching entry point)* | `match_orders(env, asset, quote) -> Result<MatchResult, Error>` | Deterministic price-time priority matching with partial fills and bounded work (`MAX_TRADES_PER_CALL`). |
//! | *(no simulation entry point)* | `simulate_match(env, asset, quote, incoming_order) -> Result<MatchResult, Error>` | Read-only pre-flight simulation returning expected fills and remaining quantity. |
//! | `get_order_by_id(env, order_id) -> Option<Order>` | `get_order(env, asset, quote, order_id) -> Result<Order, Error>` | Scoped by `(asset, quote)` market; returns typed `Error::OrderNotFound` / `Error::MarketNotFound`. |
//! | `ORDERS` / `NEXT_ID` (`Symbol` keys) | `DataKey::OrderBook((asset, quote))`, `DataKey::NextOrderId`, `DataKey::NextTradeId` | Typed `#[contracttype] DataKey` enum with per-market `MarketOrderBook` (`bids`, `asks`, `all_orders`). |

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum OrderSide {
    Buy,
    Sell,
}

// OrderStatus represents the status of an order in the order book.
#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum OrderStatus {
    New,
    Filled,
    Cancelled,
}

// Order represents an order in the order book.
#[derive(Clone)]
#[contracttype]
pub struct Order {
    pub id: u64,
    pub owner: Address,
    pub side: OrderSide,
    pub price: u64,
    pub quantity: u64,
    pub filled: u64,
    pub status: OrderStatus,
    pub timestamp: u64,
}

const ORDERS: Symbol = symbol_short!("ORDERS");
const NEXT_ID: Symbol = symbol_short!("NEXT_ID");

/// Errors surfaced by the orderbook.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFoundOrNotOwner = 1,
    InvalidAmount = 2,
}

// OrderbookContract is a simple order book contract that allows users to place, cancel, and update orders. It also provides functionality to retrieve orders by ID or by owner, as well as to get the best bid and ask prices in the order book.
#[contract]
pub struct OrderbookContract;

#[contractimpl]
impl OrderbookContract {
    pub fn place_order(
        env: Env,
        owner: Address,
        side: OrderSide,
        price: u64,
        quantity: u64,
    ) -> u64 {
        owner.require_auth();

        let mut orders = Self::get_orders(env.clone());
        let next_id = Self::get_next_id(env.clone());

        let order = Order {
            id: next_id,
            owner,
            side,
            price,
            quantity,
            filled: 0,
            status: OrderStatus::New,
            timestamp: env.ledger().timestamp(),
        };

        orders.push_back(order.clone());
        env.storage().persistent().set(&ORDERS, &orders);
        env.storage().persistent().set(&NEXT_ID, &(next_id + 1));

        env.events().publish(
            (symbol_short!("order"), symbol_short!("placed")),
            (
                order.id,
                order.owner,
                order.side,
                order.price,
                order.quantity,
            ),
        );

        next_id
    }

    fn get_orders(env: Env) -> Vec<Order> {
        env.storage()
            .persistent()
            .get(&ORDERS)
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn get_next_id(env: Env) -> u64 {
        env.storage().persistent().get(&NEXT_ID).unwrap_or(0)
    }

    pub fn get_order_by_id(env: Env, order_id: u64) -> Option<Order> {
        let orders = Self::get_orders(env);
        orders.iter().find(|order| order.id == order_id)
    }

    pub fn list_orders_by_owner(env: Env, owner: Address) -> Vec<Order> {
        let orders = Self::get_orders(env.clone());
        let mut owner_orders = Vec::new(&env);
        for order in orders.iter() {
            if order.owner == owner {
                owner_orders.push_back(order);
            }
        }
        owner_orders
    }

    pub fn cancel_order(env: Env, owner: Address, order_id: u64) -> Result<(), Error> {
        owner.require_auth();

        let mut orders = Self::get_orders(env.clone());

        for i in 0..orders.len() {
            if let Some(mut order) = orders.get(i) {
                if order.id == order_id && order.owner == owner {
                    order.status = OrderStatus::Cancelled;
                    orders.set(i, order.clone());
                    env.storage().persistent().set(&ORDERS, &orders);
                    env.events().publish(
                        (symbol_short!("order"), symbol_short!("cancelled")),
                        (order.id, order.owner),
                    );
                    return Ok(());
                }
            }
        }

        Err(Error::NotFoundOrNotOwner)
    }

    pub fn get_best_bid_ask(env: Env) -> (Option<u64>, Option<u64>) {
        let orders = Self::get_orders(env);
        let mut best_bid: Option<u64> = None;
        let mut best_ask: Option<u64> = None;

        for order in orders.iter() {
            if order.status == OrderStatus::New {
                match order.side {
                    OrderSide::Buy => {
                        if let Some(bid) = best_bid {
                            if order.price > bid {
                                best_bid = Some(order.price);
                            }
                        } else {
                            best_bid = Some(order.price);
                        }
                    }
                    OrderSide::Sell => {
                        if let Some(ask) = best_ask {
                            if order.price < ask {
                                best_ask = Some(order.price);
                            }
                        } else {
                            best_ask = Some(order.price);
                        }
                    }
                }
            }
        }

        (best_bid, best_ask)
    }

    pub fn update_order(
        env: Env,
        owner: Address,
        order_id: u64,
        new_price: u64,
        new_quantity: u64,
    ) -> Result<(), Error> {
        owner.require_auth();

        let mut orders = Self::get_orders(env.clone());

        for i in 0..orders.len() {
            if let Some(mut order) = orders.get(i) {
                if order.id == order_id && order.owner == owner {
                    order.price = new_price;
                    order.quantity = new_quantity;
                    orders.set(i, order.clone());
                    env.storage().persistent().set(&ORDERS, &orders);
                    env.events().publish(
                        (symbol_short!("order"), symbol_short!("updated")),
                        (order.id, order.price, order.quantity),
                    );
                    return Ok(());
                }
            }
        }

        Err(Error::NotFoundOrNotOwner)
    }
}

#[cfg(test)]
mod test;
