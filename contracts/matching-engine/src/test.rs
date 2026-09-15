#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

struct Fixture {
    env: Env,
    client: MatchingEngineClient<'static>,
    asset: Address,
    quote: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(MatchingEngine, ());
    let client = MatchingEngineClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    Fixture {
        asset: Address::generate(&env),
        quote: Address::generate(&env),
        env,
        client,
    }
}

#[test]
fn test_initialize_twice_fails() {
    let f = setup();
    let admin = Address::generate(&f.env);
    let res = f.client.try_initialize(&admin);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_place_order() {
    let f = setup();
    let trader = Address::generate(&f.env);

    let order_id = f
        .client
        .place_order(&trader, &f.asset, &f.quote, &OrderSide::Buy, &100, &10);
    assert_eq!(order_id, 0);

    let order = f.client.get_order(&f.asset, &f.quote, &order_id);
    assert_eq!(order.id, 0);
    assert_eq!(order.trader, trader);
    assert_eq!(order.side, OrderSide::Buy);
    assert_eq!(order.price, 100);
    assert_eq!(order.quantity, 10);
    assert_eq!(order.filled, 0);
    assert_eq!(order.status, OrderStatus::Open);
}

#[test]
fn test_invalid_order_parameters() {
    let f = setup();
    let trader = Address::generate(&f.env);

    let res = f
        .client
        .try_place_order(&trader, &f.asset, &f.quote, &OrderSide::Buy, &100, &0);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));

    let res = f
        .client
        .try_place_order(&trader, &f.asset, &f.quote, &OrderSide::Buy, &0, &10);
    assert_eq!(res, Err(Ok(Error::InvalidPrice)));
}

#[test]
fn test_single_match() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller = Address::generate(&f.env);

    // Sell order rests first (maker).
    let sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    f.env.ledger().set_timestamp(2000);
    let buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);

    let trades = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(trades.len(), 1);
    let trade = trades.get(0).unwrap();
    assert_eq!(trade.quantity, 5);
    assert_eq!(trade.price, 100);
    assert_eq!(trade.buyer, buyer);
    assert_eq!(trade.seller, seller);
    assert_eq!(trade.maker_order_id, sell_id);
    assert_eq!(trade.taker_order_id, buy_id);

    let sell_order = f.client.get_order(&f.asset, &f.quote, &sell_id);
    let buy_order = f.client.get_order(&f.asset, &f.quote, &buy_id);
    assert_eq!(sell_order.status, OrderStatus::Filled);
    assert_eq!(sell_order.filled, 5);
    assert_eq!(buy_order.status, OrderStatus::Filled);
    assert_eq!(buy_order.filled, 5);
}

#[test]
fn test_partial_fill() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller = Address::generate(&f.env);

    let sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &10);

    f.env.ledger().set_timestamp(2000);
    let buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &3);

    let trades = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(trades.len(), 1);
    assert_eq!(trades.get(0).unwrap().quantity, 3);

    let buy_order = f.client.get_order(&f.asset, &f.quote, &buy_id);
    assert_eq!(buy_order.status, OrderStatus::Filled);
    assert_eq!(buy_order.filled, 3);

    let sell_order = f.client.get_order(&f.asset, &f.quote, &sell_id);
    assert_eq!(sell_order.status, OrderStatus::PartiallyFilled);
    assert_eq!(sell_order.filled, 3);
    assert_eq!(sell_order.quantity - sell_order.filled, 7);
}

#[test]
fn test_multi_match() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller1 = Address::generate(&f.env);
    let seller2 = Address::generate(&f.env);

    let sell_id1 = f
        .client
        .place_order(&seller1, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    f.env.ledger().set_timestamp(1500);
    let sell_id2 = f
        .client
        .place_order(&seller2, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    f.env.ledger().set_timestamp(2000);
    let buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &10);

    let trades = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(trades.len(), 2);
    assert_eq!(trades.get(0).unwrap().quantity, 5);
    assert_eq!(trades.get(1).unwrap().quantity, 5);

    let order1 = f.client.get_order(&f.asset, &f.quote, &sell_id1);
    let order2 = f.client.get_order(&f.asset, &f.quote, &sell_id2);
    let buy_order = f.client.get_order(&f.asset, &f.quote, &buy_id);
    assert_eq!(order1.status, OrderStatus::Filled);
    assert_eq!(order2.status, OrderStatus::Filled);
    assert_eq!(buy_order.status, OrderStatus::Filled);
}

#[test]
fn test_price_time_priority() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller1 = Address::generate(&f.env); // sells at 99 (better)
    let seller2 = Address::generate(&f.env); // sells at 100 (worse)

    let _sell_id2 = f
        .client
        .place_order(&seller2, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    f.env.ledger().set_timestamp(1500);
    let sell_id1 = f
        .client
        .place_order(&seller1, &f.asset, &f.quote, &OrderSide::Sell, &99, &5);

    f.env.ledger().set_timestamp(2000);
    let _buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);

    let trades = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(trades.len(), 1);
    // Better-priced sell (99) matches first even though placed later.
    assert_eq!(trades.get(0).unwrap().maker_order_id, sell_id1);
    assert_eq!(trades.get(0).unwrap().price, 99);

    let better_order = f.client.get_order(&f.asset, &f.quote, &sell_id1);
    assert_eq!(better_order.status, OrderStatus::Filled);
}

#[test]
fn test_simulation_matches_actual() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller = Address::generate(&f.env);

    let _sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    f.env.ledger().set_timestamp(2000);
    let incoming_order = Order {
        id: 999,
        trader: buyer.clone(),
        asset: f.asset.clone(),
        quote: f.quote.clone(),
        side: OrderSide::Buy,
        price: 100,
        quantity: 5,
        filled: 0,
        timestamp: 2000,
        status: OrderStatus::Open,
    };

    let simulation = f.client.simulate_match(&f.asset, &f.quote, &incoming_order);
    assert_eq!(simulation.trades.len(), 1);
    assert_eq!(simulation.remaining_quantity, 0);

    let _buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let actual_trades = f.client.match_orders(&f.asset, &f.quote);

    assert_eq!(simulation.trades.len(), actual_trades.len());
    assert_eq!(
        simulation.trades.get(0).unwrap().quantity,
        actual_trades.get(0).unwrap().quantity
    );
    assert_eq!(
        simulation.trades.get(0).unwrap().price,
        actual_trades.get(0).unwrap().price
    );
}

#[test]
fn test_cancel_order_removes_from_book() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let buyer = Address::generate(&f.env);

    let sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    // Cancel the resting sell order.
    f.client.cancel_order(&f.asset, &f.quote, &sell_id, &seller);
    let cancelled = f.client.get_order(&f.asset, &f.quote, &sell_id);
    assert_eq!(cancelled.status, OrderStatus::Cancelled);

    // A crossing buy order should now find nothing to match against.
    f.env.ledger().set_timestamp(2000);
    f.client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let trades = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(trades.len(), 0);
}

#[test]
fn test_cancel_order_wrong_caller_fails() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let stranger = Address::generate(&f.env);

    let sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    let res = f
        .client
        .try_cancel_order(&f.asset, &f.quote, &sell_id, &stranger);
    assert_eq!(res, Err(Ok(Error::NotAuthorized)));
}
