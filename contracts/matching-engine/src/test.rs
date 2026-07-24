#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

#[test]
fn test_initialize() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let matching_engine = MatchingEngine;

    let result = matching_engine.initialize(env.clone(), admin.clone());
    assert!(result.is_ok());

    // Can't initialize twice
    let result = matching_engine.initialize(env.clone(), admin);
    assert_eq!(result.err(), Some(Error::AlreadyInitialized));
}

#[test]
fn test_place_order() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let trader = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place a buy order
    let order_id = matching_engine
        .place_order(
            env.clone(),
            trader.clone(),
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100, // price
            10,  // quantity
        )
        .unwrap();

    assert_eq!(order_id, 0);

    // Retrieve the order
    let order = matching_engine
        .get_order(env, asset, quote, order_id)
        .unwrap();
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
    let env = Env::default();
    let admin = Address::generate(&env);
    let trader = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Invalid quantity (0)
    let result = matching_engine.place_order(
        env.clone(),
        trader.clone(),
        asset.clone(),
        quote.clone(),
        OrderSide::Buy,
        100,
        0,
    );
    assert_eq!(result.err(), Some(Error::InvalidAmount));

    // Invalid price (0)
    let result =
        matching_engine.place_order(env.clone(), trader, asset, quote, OrderSide::Buy, 0, 10);
    assert_eq!(result.err(), Some(Error::InvalidPrice));
}

#[test]
fn test_single_match() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place sell order first (timestamp 1000)
    let sell_id = matching_engine
        .place_order(
            env.clone(),
            seller.clone(),
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            5,
        )
        .unwrap();

    // Update timestamp for buy order
    env.ledger().set_timestamp(2000);

    // Place buy order that matches
    let buy_id = matching_engine
        .place_order(
            env.clone(),
            buyer.clone(),
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100,
            5,
        )
        .unwrap();

    // Execute matching
    let trades = matching_engine
        .match_orders(env.clone(), asset.clone(), quote.clone())
        .unwrap();

    assert_eq!(trades.len(), 1);
    let trade = &trades[0];
    assert_eq!(trade.quantity, 5);
    assert_eq!(trade.price, 100);
    assert_eq!(trade.buyer, buyer);
    assert_eq!(trade.seller, seller);
    assert_eq!(trade.maker_order_id, sell_id); // Sell order was first (maker)
    assert_eq!(trade.taker_order_id, buy_id); // Buy order was second (taker)

    // Both orders should be filled
    let sell_order = matching_engine
        .get_order(env.clone(), asset.clone(), quote.clone(), sell_id)
        .unwrap();
    let buy_order = matching_engine
        .get_order(env, asset, quote, buy_id)
        .unwrap();

    assert_eq!(sell_order.status, OrderStatus::Filled);
    assert_eq!(sell_order.filled, 5);
    assert_eq!(buy_order.status, OrderStatus::Filled);
    assert_eq!(buy_order.filled, 5);
}

#[test]
fn test_partial_fill() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place large sell order
    let sell_id = matching_engine
        .place_order(
            env.clone(),
            seller.clone(),
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            10, // Selling 10 units
        )
        .unwrap();

    env.ledger().set_timestamp(2000);

    // Place smaller buy order that can only partially fill
    let buy_id = matching_engine
        .place_order(
            env.clone(),
            buyer.clone(),
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100,
            3, // Buying only 3 units
        )
        .unwrap();

    // Execute matching
    let trades = matching_engine
        .match_orders(env.clone(), asset.clone(), quote.clone())
        .unwrap();

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, 3);

    // Buy order should be fully filled
    let buy_order = matching_engine
        .get_order(env.clone(), asset.clone(), quote.clone(), buy_id)
        .unwrap();
    assert_eq!(buy_order.status, OrderStatus::Filled);
    assert_eq!(buy_order.filled, 3);

    // Sell order should be partially filled
    let sell_order = matching_engine
        .get_order(env, asset, quote, sell_id)
        .unwrap();
    assert_eq!(sell_order.status, OrderStatus::PartiallyFilled);
    assert_eq!(sell_order.filled, 3);
    assert_eq!(sell_order.quantity - sell_order.filled, 7); // 7 remaining
}

#[test]
fn test_multi_match() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller1 = Address::generate(&env);
    let seller2 = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place two sell orders at the same price
    let sell_id1 = matching_engine
        .place_order(
            env.clone(),
            seller1,
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            5,
        )
        .unwrap();

    env.ledger().set_timestamp(1500);
    let sell_id2 = matching_engine
        .place_order(
            env.clone(),
            seller2,
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            5,
        )
        .unwrap();

    env.ledger().set_timestamp(2000);
    // Place large buy order that fills both
    let buy_id = matching_engine
        .place_order(
            env.clone(),
            buyer,
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100,
            10,
        )
        .unwrap();

    // Execute matching
    let trades = matching_engine
        .match_orders(env.clone(), asset.clone(), quote.clone())
        .unwrap();

    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].quantity, 5); // Fills first sell order completely
    assert_eq!(trades[1].quantity, 5); // Fills second sell order completely

    // All orders should be filled
    let order1 = matching_engine
        .get_order(env.clone(), asset.clone(), quote.clone(), sell_id1)
        .unwrap();
    let order2 = matching_engine
        .get_order(env.clone(), asset.clone(), quote.clone(), sell_id2)
        .unwrap();
    let buy_order = matching_engine
        .get_order(env, asset, quote, buy_id)
        .unwrap();

    assert_eq!(order1.status, OrderStatus::Filled);
    assert_eq!(order2.status, OrderStatus::Filled);
    assert_eq!(buy_order.status, OrderStatus::Filled);
}

#[test]
fn test_price_time_priority() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller1 = Address::generate(&env); // Will sell at 99 (better price)
    let seller2 = Address::generate(&env); // Will sell at 100 (worse price)
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place sell order with worse price first
    let sell_id2 = matching_engine
        .place_order(
            env.clone(),
            seller2,
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            5,
        )
        .unwrap();

    // Then place sell order with better price
    env.ledger().set_timestamp(1500);
    let sell_id1 = matching_engine
        .place_order(
            env.clone(),
            seller1,
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            99, // Better price (lower for sells)
            5,
        )
        .unwrap();

    // Place buy order
    env.ledger().set_timestamp(2000);
    let buy_id = matching_engine
        .place_order(
            env.clone(),
            buyer,
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100,
            5,
        )
        .unwrap();

    // Execute matching
    let trades = matching_engine
        .match_orders(env.clone(), asset.clone(), quote.clone())
        .unwrap();

    assert_eq!(trades.len(), 1);
    // The better priced order (99) should match first, even though it was placed later
    assert_eq!(trades[0].maker_order_id, sell_id1);
    assert_eq!(trades[0].price, 99);

    let better_order = matching_engine
        .get_order(env, asset, quote, sell_id1)
        .unwrap();
    assert_eq!(better_order.status, OrderStatus::Filled);
}

#[test]
fn test_simulation_matches_actual() {
    let env = Env::default();
    env.ledger().set_timestamp(1000);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let asset = Address::generate(&env);
    let quote = Address::generate(&env);
    let matching_engine = MatchingEngine;

    matching_engine.initialize(env.clone(), admin).unwrap();

    // Place sell order in the actual order book
    let sell_id = matching_engine
        .place_order(
            env.clone(),
            seller,
            asset.clone(),
            quote.clone(),
            OrderSide::Sell,
            100,
            5,
        )
        .unwrap();

    // Create an incoming order to simulate
    env.ledger().set_timestamp(2000);
    let incoming_order = Order {
        id: 999,
        trader: buyer,
        asset: asset.clone(),
        quote: quote.clone(),
        side: OrderSide::Buy,
        price: 100,
        quantity: 5,
        filled: 0,
        timestamp: 2000,
        status: OrderStatus::Open,
    };

    // Run simulation
    let simulation = matching_engine
        .simulate_match(env.clone(), asset.clone(), quote.clone(), incoming_order)
        .unwrap();

    assert_eq!(simulation.trades.len(), 1);
    assert_eq!(simulation.remaining_quantity, 0);

    // Now place the order for real and execute matching
    let buy_id = matching_engine
        .place_order(
            env.clone(),
            buyer,
            asset.clone(),
            quote.clone(),
            OrderSide::Buy,
            100,
            5,
        )
        .unwrap();

    let actual_trades = matching_engine
        .match_orders(env.clone(), asset.clone(), quote.clone())
        .unwrap();

    // Simulation should match actual results
    assert_eq!(simulation.trades.len(), actual_trades.len());
    assert_eq!(simulation.trades[0].quantity, actual_trades[0].quantity);
    assert_eq!(simulation.trades[0].price, actual_trades[0].price);
}
