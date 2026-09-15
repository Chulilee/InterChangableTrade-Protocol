#![no_std]

//! # Matching Engine
//!
//! Deterministic price-time matching engine that consumes orders from the orderbook
//! and performs price-time priority matching. Supports partial fills, multi-fill matching,
//! price-time priority, and atomic settlement hooks.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

/// Storage keys for the matching engine.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin authorized to administer the matching engine.
    Admin,
    /// Auto-incrementing id for the next order.
    NextOrderId,
    /// Order book data structure - maps market (asset+quote) to order book state
    OrderBook((Address, Address)),
}

/// Order side - buy or sell
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// Order status lifecycle
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Cancelled,
}

/// An order in the order book
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Order {
    pub id: u64,
    pub trader: Address,
    pub asset: Address,
    pub quote: Address,
    pub side: OrderSide,
    pub price: i128,    // Price per unit of asset (in quote token)
    pub quantity: i128, // Total quantity requested
    pub filled: i128,   // Quantity already filled
    pub timestamp: u64, // Timestamp when order was placed (for time priority)
    pub status: OrderStatus,
}

/// A executed trade
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub id: u64,
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub asset: Address,
    pub quote: Address,
    pub price: i128,
    pub quantity: i128,
    pub timestamp: u64,
}

/// Price level in the order book - contains all orders at a specific price
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceLevel {
    pub price: i128,
    pub total_quantity: i128,
    pub order_ids: Vec<u64>, // Orders in time priority (FIFO)
}

/// Order book for a specific market (asset + quote)
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketOrderBook {
    pub bids: Vec<PriceLevel>,  // Buy orders sorted by price descending
    pub asks: Vec<PriceLevel>,  // Sell orders sorted by price ascending
    pub all_orders: Vec<Order>, // All active orders in the market
}

/// Result of a matching simulation containing all trades that would be executed
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchResult {
    pub trades: Vec<Trade>,
    pub remaining_quantity: i128,
    pub success: bool,
}

/// Errors surfaced by the matching engine.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    OrderNotFound = 3,
    OrderNotOpen = 4,
    InvalidAmount = 5,
    InvalidPrice = 6,
    NotAuthorized = 7,
    MarketNotFound = 8,
    SettlementFailed = 9,
}

// Event symbols
const EVT_TRADE_EXECUTED: Symbol = symbol_short!("tradeexec");
const EVT_ORDER_PARTIAL: Symbol = symbol_short!("ordpart");
const EVT_ORDER_FILLED: Symbol = symbol_short!("ordfill");
const EVT_ORDER_PLACED: Symbol = symbol_short!("ordplace");
const EVT_ORDER_CANCEL: Symbol = symbol_short!("ordcancel");

#[contract]
pub struct MatchingEngine;

#[contractimpl]
impl MatchingEngine {
    /// Initialize the matching engine with an admin. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextOrderId, &0u64);
        Ok(())
    }

    /// Place a new order into the order book. Requires trader authorization.
    pub fn place_order(
        env: Env,
        trader: Address,
        asset: Address,
        quote: Address,
        side: OrderSide,
        price: i128,
        quantity: i128,
    ) -> Result<u64, Error> {
        Self::ensure_init(&env)?;
        trader.require_auth();

        if quantity <= 0 {
            return Err(Error::InvalidAmount);
        }
        if price <= 0 {
            return Err(Error::InvalidPrice);
        }

        // Get or create order book for this market
        let market_key = (asset.clone(), quote.clone());
        let mut order_book: MarketOrderBook = env
            .storage()
            .persistent()
            .get(&DataKey::OrderBook(market_key.clone()))
            .unwrap_or_else(|| MarketOrderBook {
                bids: Vec::new(&env),
                asks: Vec::new(&env),
                all_orders: Vec::new(&env),
            });

        // Create new order
        let order_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextOrderId)
            .unwrap_or(0);
        let timestamp = env.ledger().timestamp();

        let order = Order {
            id: order_id,
            trader: trader.clone(),
            asset: asset.clone(),
            quote: quote.clone(),
            side,
            price,
            quantity,
            filled: 0,
            timestamp,
            status: OrderStatus::Open,
        };

        // Add order to the appropriate price level
        Self::add_order_to_price_level(&mut order_book, order.clone())?;

        // Save the updated order book
        env.storage()
            .persistent()
            .set(&DataKey::OrderBook(market_key), &order_book);

        // Update next order id
        env.storage()
            .instance()
            .set(&DataKey::NextOrderId, &(order_id + 1));

        // Emit order placed event
        env.events()
            .publish((EVT_ORDER_PLACED, order_id), order.clone());

        Ok(order_id)
    }

    /// Cancel an open (or partially filled) order. Only the trader who placed the
    /// order may cancel it. The order is marked `Cancelled` and its remaining
    /// quantity is removed from the resting price level so it can no longer match.
    pub fn cancel_order(
        env: Env,
        asset: Address,
        quote: Address,
        order_id: u64,
        caller: Address,
    ) -> Result<(), Error> {
        Self::ensure_init(&env)?;
        caller.require_auth();

        let market_key = (asset.clone(), quote.clone());
        let mut order_book: MarketOrderBook = env
            .storage()
            .persistent()
            .get(&DataKey::OrderBook(market_key.clone()))
            .ok_or(Error::MarketNotFound)?;

        // Locate the order and verify ownership + state.
        let mut target: Option<Order> = None;
        let mut order_index: u32 = 0;
        for (i, o) in order_book.all_orders.iter().enumerate() {
            if o.id == order_id {
                target = Some(o);
                order_index = i as u32;
                break;
            }
        }
        let mut order = target.ok_or(Error::OrderNotFound)?;
        if order.trader != caller {
            return Err(Error::NotAuthorized);
        }
        if order.status != OrderStatus::Open && order.status != OrderStatus::PartiallyFilled {
            return Err(Error::OrderNotOpen);
        }

        // Remove the order's remaining quantity from its resting price level.
        let remaining = order.quantity - order.filled;
        let levels = if order.side == OrderSide::Buy {
            &mut order_book.bids
        } else {
            &mut order_book.asks
        };
        for i in 0..levels.len() {
            let mut level = levels.get(i).unwrap();
            if level.price == order.price {
                let mut new_ids = Vec::new(&env);
                for id in level.order_ids.iter() {
                    if id != order_id {
                        new_ids.push_back(id);
                    }
                }
                level.order_ids = new_ids;
                level.total_quantity -= remaining;
                if level.order_ids.is_empty() {
                    levels.remove(i);
                } else {
                    levels.set(i, level);
                }
                break;
            }
        }

        // Mark the order cancelled in the flat order list.
        order.status = OrderStatus::Cancelled;
        order_book.all_orders.set(order_index, order);

        env.storage()
            .persistent()
            .set(&DataKey::OrderBook(market_key), &order_book);

        env.events().publish((EVT_ORDER_CANCEL, order_id), caller);

        Ok(())
    }

    /// Batch match orders for a specific market. This is the main matching function
    /// that processes all new orders against the order book and executes trades.
    pub fn match_orders(env: Env, asset: Address, quote: Address) -> Result<Vec<Trade>, Error> {
        Self::ensure_init(&env)?;

        let market_key = (asset.clone(), quote.clone());
        let mut order_book: MarketOrderBook = env
            .storage()
            .persistent()
            .get(&DataKey::OrderBook(market_key.clone()))
            .ok_or(Error::MarketNotFound)?;

        let mut executed_trades = Vec::new(&env);

        // Perform the matching process
        Self::process_matching(&env, &mut order_book, &mut executed_trades)?;

        // Save the updated order book
        env.storage()
            .persistent()
            .set(&DataKey::OrderBook(market_key), &order_book);

        Ok(executed_trades)
    }

    /// Simulate matching without modifying any state. Returns the same results as match_orders
    /// but doesn't persist any changes to the order book.
    pub fn simulate_match(
        env: Env,
        asset: Address,
        quote: Address,
        incoming_order: Order,
    ) -> Result<MatchResult, Error> {
        Self::ensure_init(&env)?;

        let market_key = (asset.clone(), quote.clone());
        let order_book: MarketOrderBook = env
            .storage()
            .persistent()
            .get(&DataKey::OrderBook(market_key))
            .ok_or(Error::MarketNotFound)?;

        // Create a copy of the order book to simulate on
        let mut simulation_book = order_book.clone();
        let mut simulated_trades = Vec::new(&env);

        // Add the incoming order to the simulation book
        Self::add_order_to_price_level(&mut simulation_book, incoming_order.clone())?;

        // Process matching on the simulation book
        let mut remaining = incoming_order.quantity - incoming_order.filled;
        Self::process_matching(&env, &mut simulation_book, &mut simulated_trades)?;

        // Calculate remaining quantity after simulation
        for trade in simulated_trades.iter() {
            if trade.taker_order_id == incoming_order.id {
                remaining -= trade.quantity;
            }
        }

        Ok(MatchResult {
            trades: simulated_trades,
            remaining_quantity: remaining.max(0),
            success: true,
        })
    }

    /// Get an order by its ID
    pub fn get_order(
        env: Env,
        asset: Address,
        quote: Address,
        order_id: u64,
    ) -> Result<Order, Error> {
        let market_key = (asset, quote);
        let order_book: MarketOrderBook = env
            .storage()
            .persistent()
            .get(&DataKey::OrderBook(market_key))
            .ok_or(Error::MarketNotFound)?;

        for order in order_book.all_orders.iter() {
            if order.id == order_id {
                return Ok(order);
            }
        }

        Err(Error::OrderNotFound)
    }

    /// Ensure the contract is initialized
    fn ensure_init(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    /// Add an order to the appropriate price level in the order book
    fn add_order_to_price_level(
        order_book: &mut MarketOrderBook,
        order: Order,
    ) -> Result<(), Error> {
        let price_levels = if order.side == OrderSide::Buy {
            &mut order_book.bids
        } else {
            &mut order_book.asks
        };

        // Find if there's an existing price level for this price
        let mut found = false;
        for (level_index, level) in price_levels.iter().enumerate() {
            if level.price == order.price {
                // Get mutable access by removing and reinserting (simplified for Soroban Vec)
                let mut mut_level = level.clone();
                mut_level.order_ids.push_back(order.id);
                mut_level.total_quantity += order.quantity - order.filled;
                price_levels.set(level_index as u32, mut_level);
                found = true;
                break;
            }
        }

        // If no existing price level, create a new one and insert in the correct position
        if !found {
            let new_level = PriceLevel {
                price: order.price,
                total_quantity: order.quantity - order.filled,
                order_ids: {
                    let mut vec = Vec::new(order_book.all_orders.env());
                    vec.push_back(order.id);
                    vec
                },
            };

            // Insert in the correct position to maintain sorting without using sort_by
            if order.side == OrderSide::Buy {
                // Bids sorted descending - find first price lower than new price and insert before
                let mut insert_at = price_levels.len();
                for (i, level) in price_levels.iter().enumerate() {
                    if level.price < new_level.price {
                        insert_at = i as u32;
                        break;
                    }
                }
                price_levels.insert(insert_at, new_level);
            } else {
                // Asks sorted ascending - find first price higher than new price and insert before
                let mut insert_at = price_levels.len();
                for (i, level) in price_levels.iter().enumerate() {
                    if level.price > new_level.price {
                        insert_at = i as u32;
                        break;
                    }
                }
                price_levels.insert(insert_at, new_level);
            }
        }

        // Add to all_orders
        order_book.all_orders.push_back(order);

        Ok(())
    }

    /// Core matching logic that processes the order book and executes trades
    fn process_matching(
        env: &Env,
        order_book: &mut MarketOrderBook,
        executed_trades: &mut Vec<Trade>,
    ) -> Result<(), Error> {
        let mut trade_id: u64 = 0; // In production, this would be a persistent counter

        // Continue matching while there are both bids and asks that can match
        while !order_book.bids.is_empty() && !order_book.asks.is_empty() {
            // Get first (best) bid and ask price levels using Soroban's get() method
            let best_bid = order_book.bids.get(0).ok_or(Error::OrderNotFound)?;
            let best_ask = order_book.asks.get(0).ok_or(Error::OrderNotFound)?;

            // Check if prices cross (bid >= ask)
            if best_bid.price < best_ask.price {
                break; // No more matches possible
            }

            // Get the first order in each price level (time priority)
            let bid_order_id = best_bid.order_ids.first().unwrap();
            let ask_order_id = best_ask.order_ids.first().unwrap();

            // Find the actual order objects
            let mut bid_order = order_book
                .all_orders
                .iter()
                .find(|o| o.id == bid_order_id)
                .ok_or(Error::OrderNotFound)?
                .clone();

            let mut ask_order = order_book
                .all_orders
                .iter()
                .find(|o| o.id == ask_order_id)
                .ok_or(Error::OrderNotFound)?
                .clone();

            // Calculate matchable quantity
            let bid_remaining = bid_order.quantity - bid_order.filled;
            let ask_remaining = ask_order.quantity - ask_order.filled;
            let matched_quantity = bid_remaining.min(ask_remaining);

            // Use the maker's price (the order that was resting on the book)
            // In price-time priority, the maker's price is what determines execution
            let execution_price = if bid_order.timestamp < ask_order.timestamp {
                bid_order.price
            } else {
                ask_order.price
            };

            // Create and execute the trade
            let trade = Trade {
                id: trade_id,
                maker_order_id: if bid_order.timestamp < ask_order.timestamp {
                    bid_order.id
                } else {
                    ask_order.id
                },
                taker_order_id: if bid_order.timestamp < ask_order.timestamp {
                    ask_order.id
                } else {
                    bid_order.id
                },
                buyer: if bid_order.side == OrderSide::Buy {
                    bid_order.trader.clone()
                } else {
                    ask_order.trader.clone()
                },
                seller: if ask_order.side == OrderSide::Sell {
                    ask_order.trader.clone()
                } else {
                    bid_order.trader.clone()
                },
                asset: bid_order.asset.clone(),
                quote: bid_order.quote.clone(),
                price: execution_price,
                quantity: matched_quantity,
                timestamp: env.ledger().timestamp(),
            };

            // Publish the authoritative trade record for the settlement layer.
            Self::record_trade_execution(env, &trade);

            // Update order filled quantities
            bid_order.filled += matched_quantity;
            ask_order.filled += matched_quantity;

            // Update order statuses and emit events
            if bid_order.filled == bid_order.quantity {
                bid_order.status = OrderStatus::Filled;
                env.events()
                    .publish((EVT_ORDER_FILLED, bid_order.id), (bid_order.filled,));
            } else {
                bid_order.status = OrderStatus::PartiallyFilled;
                env.events().publish(
                    (EVT_ORDER_PARTIAL, bid_order.id),
                    (bid_order.filled, bid_order.quantity - bid_order.filled),
                );
            }

            if ask_order.filled == ask_order.quantity {
                ask_order.status = OrderStatus::Filled;
                env.events()
                    .publish((EVT_ORDER_FILLED, ask_order.id), (ask_order.filled,));
            } else {
                ask_order.status = OrderStatus::PartiallyFilled;
                env.events().publish(
                    (EVT_ORDER_PARTIAL, ask_order.id),
                    (ask_order.filled, ask_order.quantity - ask_order.filled),
                );
            }

            // Update the orders in all_orders - find their indices and use set()
            for (i, order) in order_book.all_orders.iter().enumerate() {
                if order.id == bid_order.id {
                    order_book.all_orders.set(i as u32, bid_order.clone());
                }
                if order.id == ask_order.id {
                    order_book.all_orders.set(i as u32, ask_order.clone());
                }
            }

            // Update price level quantities
            let mut mut_best_bid = best_bid.clone();
            mut_best_bid.total_quantity -= matched_quantity;
            order_book.bids.set(0, mut_best_bid);

            let mut mut_best_ask = best_ask.clone();
            mut_best_ask.total_quantity -= matched_quantity;
            order_book.asks.set(0, mut_best_ask);

            // Remove filled orders from price levels
            if bid_order.filled == bid_order.quantity {
                let mut bb = order_book.bids.get(0).unwrap();
                bb.order_ids.remove(0);
                order_book.bids.set(0, bb);
            }
            if ask_order.filled == ask_order.quantity {
                let mut ba = order_book.asks.get(0).unwrap();
                ba.order_ids.remove(0);
                order_book.asks.set(0, ba);
            }

            // Remove empty price levels
            let current_best_bid = order_book.bids.get(0).unwrap();
            if current_best_bid.order_ids.is_empty() {
                order_book.bids.remove(0);
            }
            let current_best_ask = order_book.asks.get(0).unwrap();
            if current_best_ask.order_ids.is_empty() {
                order_book.asks.remove(0);
            }

            // Add to executed trades
            executed_trades.push_back(trade);
            trade_id += 1;
        }

        Ok(())
    }
}

impl MatchingEngine {
    /// Records a matched trade and emits the authoritative `EVT_TRADE_EXECUTED`
    /// event that the settlement layer consumes to move funds.
    ///
    /// The matching engine deliberately does not move tokens itself: it matches
    /// orders and publishes an unambiguous trade record (price, quantity, both
    /// counterparties), while the `trade-settlement` contract in this workspace
    /// performs the atomic `token::Client` transfers that settle it. Keeping
    /// matching and settlement separate keeps the match logic stateless with
    /// respect to balances and lets settlement batch/net trades.
    fn record_trade_execution(env: &Env, trade: &Trade) {
        env.events().publish(
            (EVT_TRADE_EXECUTED, trade.id),
            (
                trade.maker_order_id,
                trade.taker_order_id,
                trade.price,
                trade.quantity,
            ),
        );
    }
}

#[cfg(test)]
mod test;
