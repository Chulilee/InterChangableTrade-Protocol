
#![cfg(test)]

extern crate std;

use crate::{OrderbookContract, OrderbookContractClient};
use soroban_sdk::{
    testutils::{
        Address as _,
        Events as _,
    },
    Address,
    Env,
    IntoVal,
    Symbol,
};

#[test]
fn test_place_order() {
    let env = Env::default();
    let contract_id = env.register_contract(None, OrderbookContract);
    let client = OrderbookContractClient::new(&env, &contract_id);

    let owner = Address::random(&env);
    let side = crate::OrderSide::Buy;
    let price = 100;
    let quantity = 10;

    let order_id = client.place_order(&owner, &side, &price, &quantity);

    let order = client.get_order_by_id(&order_id).unwrap();
    assert_eq!(order.id, order_id);
    assert_eq!(order.owner, owner);
    assert_eq!(order.side, side);
    assert_eq!(order.price, price);
    assert_eq!(order.quantity, quantity);
    assert_eq!(order.filled, 0);
    assert_eq!(order.status, crate::OrderStatus::New);

    let event = env.events().all().last().unwrap();
    assert_eq!(
        event.topics.last().unwrap(),
        (
            Symbol::new(&env, "placed"),
            order.id,
            order.owner,
            order.side,
            order.price,
            order.quantity
        )
            .into_val(&env)
    );
}

#[test]
fn test_update_order() {
    let env = Env::default();
    let contract_id = env.register_contract(None, OrderbookContract);
    let client = OrderbookContractClient::new(&env, &contract_id);

    let owner = Address::random(&env);
    let side = crate::OrderSide::Buy;
    let price = 100;
    let quantity = 10;

    let order_id = client.place_order(&owner, &side, &price, &quantity);

    let new_price = 110;
    let new_quantity = 12;

    client.update_order(&owner, &order_id, &new_price, &new_quantity);

    let order = client.get_order_by_id(&order_id).unwrap();
    assert_eq!(order.price, new_price);
    assert_eq!(order.quantity, new_quantity);

    let event = env.events().all().last().unwrap();
    assert_eq!(
        event.topics.last().unwrap(),
        (
            Symbol::new(&env, "updated"),
            order.id,
            order.price,
            order.quantity
        )
            .into_val(&env)
    );
}

#[test]
fn test_cancel_order() {
    let env = Env::default();
    let contract_id = env.register_contract(None, OrderbookContract);
    let client = OrderbookContractClient::new(&env, &contract_id);

    let owner = Address::random(&env);
    let side = crate::OrderSide::Buy;
    let price = 100;
    let quantity = 10;

    let order_id = client.place_order(&owner, &side, &price, &quantity);

    client.cancel_order(&owner, &order_id);

    let order = client.get_order_by_id(&order_id).unwrap();
    assert_eq!(order.status, crate::OrderStatus::Cancelled);

    let event = env.events().all().last().unwrap();
    assert_eq!(
        event.topics.last().unwrap(),
        (Symbol::new(&env, "cancelled"), order.id, order.owner).into_val(&env)
    );
}

#[test]
fn test_get_best_bid_ask() {
    let env = Env::default();
    let contract_id = env.register_contract(None, OrderbookContract);
    let client = OrderbookContractClient::new(&env, &contract_id);

    let owner1 = Address::random(&env);
    let owner2 = Address::random(&env);

    client.place_order(&owner1, &crate::OrderSide::Buy, &100, &10);
    client.place_order(&owner2, &crate::OrderSide::Buy, &105, &5);

    client.place_order(&owner1, &crate::OrderSide::Sell, &110, &10);
    client.place_order(&owner2, &crate::OrderSide::Sell, &108, &5);

    let (best_bid, best_ask) = client.get_best_bid_ask();
    assert_eq!(best_bid, Some(105));
    assert_eq!(best_ask, Some(108));
}

#[test]
fn test_list_orders_by_owner() {
    let env = Env::default();
    let contract_id = env.register_contract(None, OrderbookContract);
    let client = OrderbookContractClient::new(&env, &contract_id);

    let owner1 = Address::random(&env);
    let owner2 = Address::random(&env);

    client.place_order(&owner1, &crate::OrderSide::Buy, &100, &10);
    client.place_order(&owner2, &crate::OrderSide::Buy, &105, &5);
    client.place_order(&owner1, &crate::OrderSide::Sell, &110, &10);

    let owner1_orders = client.list_orders_by_owner(&owner1);
    assert_eq!(owner1_orders.len(), 2);
}