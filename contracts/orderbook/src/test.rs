#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: OrderbookContractClient<'static>,
    owner: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(OrderbookContract, ());
    let client = OrderbookContractClient::new(&env, &contract_id);
    let owner = Address::generate(&env);
    Fixture { env, client, owner }
}

#[test]
fn place_order_assigns_incrementing_ids() {
    let f = setup();
    let id0 = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    let id1 = f.client.place_order(&f.owner, &OrderSide::Sell, &110, &5);
    assert_eq!(id0, 0);
    assert_eq!(id1, 1);
}

#[test]
fn place_order_stores_new_order() {
    let f = setup();
    let id = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    let order = f.client.get_order_by_id(&id).unwrap();
    assert_eq!(order.owner, f.owner);
    assert_eq!(order.price, 100);
    assert_eq!(order.quantity, 10);
    assert_eq!(order.filled, 0);
    assert!(order.status == OrderStatus::New);
    assert!(order.side == OrderSide::Buy);
}

#[test]
fn get_order_by_id_returns_none_for_missing() {
    let f = setup();
    assert!(f.client.get_order_by_id(&999).is_none());
}

#[test]
fn list_orders_by_owner_filters_by_owner() {
    let f = setup();
    let other = Address::generate(&f.env);
    f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    f.client.place_order(&other, &OrderSide::Sell, &120, &7);
    f.client.place_order(&f.owner, &OrderSide::Sell, &130, &3);

    let mine = f.client.list_orders_by_owner(&f.owner);
    assert_eq!(mine.len(), 2);
    for order in mine.iter() {
        assert_eq!(order.owner, f.owner);
    }

    let theirs = f.client.list_orders_by_owner(&other);
    assert_eq!(theirs.len(), 1);
}

#[test]
fn cancel_order_marks_cancelled() {
    let f = setup();
    let id = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    f.client.cancel_order(&f.owner, &id);
    let order = f.client.get_order_by_id(&id).unwrap();
    assert!(order.status == OrderStatus::Cancelled);
}

#[test]
fn cancel_missing_order_fails() {
    let f = setup();
    assert!(f.client.try_cancel_order(&f.owner, &999).is_err());
}

#[test]
fn cancel_by_non_owner_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let id = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    assert!(f.client.try_cancel_order(&stranger, &id).is_err());
}

#[test]
fn update_order_changes_price_and_quantity() {
    let f = setup();
    let id = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    f.client.update_order(&f.owner, &id, &150, &20);
    let order = f.client.get_order_by_id(&id).unwrap();
    assert_eq!(order.price, 150);
    assert_eq!(order.quantity, 20);
}

#[test]
fn update_by_non_owner_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let id = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &10);
    assert!(f
        .client
        .try_update_order(&stranger, &id, &150, &20)
        .is_err());
}

#[test]
fn best_bid_ask_picks_best_prices() {
    let f = setup();
    // Bids (buys): the best bid is the highest price.
    f.client.place_order(&f.owner, &OrderSide::Buy, &100, &1);
    f.client.place_order(&f.owner, &OrderSide::Buy, &105, &1);
    f.client.place_order(&f.owner, &OrderSide::Buy, &95, &1);
    // Asks (sells): the best ask is the lowest price.
    f.client.place_order(&f.owner, &OrderSide::Sell, &110, &1);
    f.client.place_order(&f.owner, &OrderSide::Sell, &108, &1);
    f.client.place_order(&f.owner, &OrderSide::Sell, &120, &1);

    let (best_bid, best_ask) = f.client.get_best_bid_ask();
    assert_eq!(best_bid, Some(105));
    assert_eq!(best_ask, Some(108));
}

#[test]
fn best_bid_ask_ignores_cancelled_orders() {
    let f = setup();
    let top = f.client.place_order(&f.owner, &OrderSide::Buy, &100, &1);
    f.client.place_order(&f.owner, &OrderSide::Buy, &90, &1);

    // With the 100 bid live it is the best; once cancelled the 90 bid wins.
    let (before, _) = f.client.get_best_bid_ask();
    assert_eq!(before, Some(100));

    f.client.cancel_order(&f.owner, &top);
    let (after, ask) = f.client.get_best_bid_ask();
    assert_eq!(after, Some(90));
    assert_eq!(ask, None);
}
