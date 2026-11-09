#![cfg(test)]
use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env, TryIntoVal,
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

    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 1);
    assert!(!result.more_available);
    let trade = result.trades.get(0).unwrap();
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

    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 1);
    assert!(!result.more_available);
    assert_eq!(result.trades.get(0).unwrap().quantity, 3);

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

    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 2);
    assert!(!result.more_available);
    assert_eq!(result.trades.get(0).unwrap().quantity, 5);
    assert_eq!(result.trades.get(1).unwrap().quantity, 5);

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

    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 1);
    assert!(!result.more_available);
    // Better-priced sell (99) matches first even though placed later.
    assert_eq!(result.trades.get(0).unwrap().maker_order_id, sell_id1);
    assert_eq!(result.trades.get(0).unwrap().price, 99);

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
    let actual = f.client.match_orders(&f.asset, &f.quote);
    assert!(!actual.more_available);

    assert_eq!(simulation.trades.len(), actual.trades.len());
    assert_eq!(
        simulation.trades.get(0).unwrap().quantity,
        actual.trades.get(0).unwrap().quantity
    );
    assert_eq!(
        simulation.trades.get(0).unwrap().price,
        actual.trades.get(0).unwrap().price
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
    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 0);
    assert!(!result.more_available);
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

#[test]
fn test_trade_ids_unique_across_match_calls() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller1 = Address::generate(&f.env);
    let seller2 = Address::generate(&f.env);

    // First match: one trade with ID 0
    let _sell_id1 = f
        .client
        .place_order(&seller1, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);
    f.env.ledger().set_timestamp(2000);
    let _buy_id1 = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let result1 = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result1.trades.len(), 1);
    assert!(!result1.more_available);
    assert_eq!(result1.trades.get(0).unwrap().id, 0);

    // Second match: trade ID should be 1, not 0
    let _sell_id2 = f
        .client
        .place_order(&seller2, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);
    f.env.ledger().set_timestamp(3000);
    let _buy_id2 = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let result2 = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result2.trades.len(), 1);
    assert!(!result2.more_available);
    assert_eq!(result2.trades.get(0).unwrap().id, 1);
}

#[test]
fn test_trade_ids_unique_across_transactions() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller = Address::generate(&f.env);

    // First transaction: place and match
    let _sell_id = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);
    f.env.ledger().set_timestamp(2000);
    let _buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let result1 = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result1.trades.len(), 1);
    assert!(!result1.more_available);
    assert_eq!(result1.trades.get(0).unwrap().id, 0);

    // Second transaction: place and match again
    let _sell_id2 = f
        .client
        .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);
    f.env.ledger().set_timestamp(3000);
    let _buy_id2 = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &5);
    let result2 = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result2.trades.len(), 1);
    assert!(!result2.more_available);
    assert_eq!(result2.trades.get(0).unwrap().id, 1);
}

#[test]
fn test_trade_ids_unique_multi_fill() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let seller1 = Address::generate(&f.env);
    let seller2 = Address::generate(&f.env);

    // Two sell orders resting
    let _sell_id1 = f
        .client
        .place_order(&seller1, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);
    f.env.ledger().set_timestamp(1500);
    let _sell_id2 = f
        .client
        .place_order(&seller2, &f.asset, &f.quote, &OrderSide::Sell, &100, &5);

    // One buy order matches both sells
    f.env.ledger().set_timestamp(2000);
    let _buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &10);
    let result = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(result.trades.len(), 2);
    assert!(!result.more_available);
    assert_eq!(result.trades.get(0).unwrap().id, 0);
    assert_eq!(result.trades.get(1).unwrap().id, 1);
}

#[test]
fn test_trade_executed_event_contains_counterparties_and_market() {
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

    f.client.match_orders(&f.asset, &f.quote);

    let events = f.env.events().all();
    let trade_event = events
        .iter()
        .find(|e| {
            let topics = e.1.clone();
            if topics.len() != 2 {
                return false;
            }
            let topic0: soroban_sdk::Symbol = topics
                .get(0)
                .unwrap()
                .try_into_val(&f.env)
                .expect("topic 0 is not a symbol");
            topic0 == symbol_short!("tradeexec")
        })
        .expect("EVT_TRADE_EXECUTED event not found");

    let expected: (u64, u64, Address, Address, (Address, Address), i128, i128) = (
        sell_id,
        buy_id,
        seller.clone(),
        buyer.clone(),
        (f.asset.clone(), f.quote.clone()),
        100_i128,
        5_i128,
    );
    let emitted: (u64, u64, Address, Address, (Address, Address), i128, i128) =
        trade_event.2.try_into_val(&f.env).unwrap();
    assert_eq!(emitted, expected);
}

#[test]
fn test_match_orders_chunked_convergence_respects_cap() {
    let f = setup();
    let buyer = Address::generate(&f.env);

    // 15 resting sell orders at the same price, 1 unit each (time priority).
    for i in 0..15u64 {
        let seller = Address::generate(&f.env);
        f.env.ledger().set_timestamp(1000 + i);
        f.client
            .place_order(&seller, &f.asset, &f.quote, &OrderSide::Sell, &100, &1);
    }

    // One crossing buy for 15 units.
    f.env.ledger().set_timestamp(2000);
    let buy_id = f
        .client
        .place_order(&buyer, &f.asset, &f.quote, &OrderSide::Buy, &100, &15);

    // First chunk: bounded by MAX_TRADES_PER_CALL.
    let first = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(first.trades.len() as u32, MAX_TRADES_PER_CALL);
    assert!(first.more_available);

    // Second chunk: completes the remainder.
    let second = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(second.trades.len(), 5);
    assert!(!second.more_available);

    // Third call: nothing left to match.
    let third = f.client.match_orders(&f.asset, &f.quote);
    assert_eq!(third.trades.len(), 0);
    assert!(!third.more_available);

    let buy_order = f.client.get_order(&f.asset, &f.quote, &buy_id);
    assert_eq!(buy_order.status, OrderStatus::Filled);
    assert_eq!(buy_order.filled, 15);
}
