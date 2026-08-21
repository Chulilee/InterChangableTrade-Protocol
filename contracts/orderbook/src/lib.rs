#![no_std]
#![deny(warnings)]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec};

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

    pub fn cancel_order(env: Env, owner: Address, order_id: u64) {
        owner.require_auth();

        let mut orders = Self::get_orders(env.clone());
        let mut order_found = false;

        for i in 0..orders.len() {
            if let Some(mut order) = orders.get(i) {
                if order.id == order_id && order.owner == owner {
                    order.status = OrderStatus::Cancelled;
                    orders.set(i, order.clone());
                    order_found = true;

                    env.events().publish(
                        (symbol_short!("order"), symbol_short!("cancelled")),
                        (order.id, order.owner),
                    );
                    break;
                }
            }
        }

        if order_found {
            env.storage().persistent().set(&ORDERS, &orders);
        } else {
            panic!("Order not found or not owned by the caller");
        }
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
    ) {
        owner.require_auth();

        let mut orders = Self::get_orders(env.clone());
        let mut order_found = false;

        for i in 0..orders.len() {
            if let Some(mut order) = orders.get(i) {
                if order.id == order_id && order.owner == owner {
                    order.price = new_price;
                    order.quantity = new_quantity;
                    orders.set(i, order.clone());
                    order_found = true;

                    env.events().publish(
                        (symbol_short!("order"), symbol_short!("updated")),
                        (order.id, order.price, order.quantity),
                    );
                    break;
                }
            }
        }

        if order_found {
            env.storage().persistent().set(&ORDERS, &orders);
        } else {
            panic!("Order not found or not owned by the caller");
        }
    }
}

#[cfg(test)]
mod test;
