#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: TradeSettlementClient<'static>,
    buyer: Address,
    seller: Address,
    asset: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&env, &contract_id);
    Fixture {
        buyer: Address::generate(&env),
        seller: Address::generate(&env),
        asset: Address::generate(&env),
        client,
        env,
    }
}

#[test]
fn open_and_get() {
    let f = setup();
    let id = f.client.open(&f.buyer, &f.seller, &f.asset, &10, &100);
    let trade = f.client.get(&id);
    assert_eq!(trade.phase, Phase::Pending);
    assert_eq!(trade.amount, 10);
}

#[test]
fn open_invalid_amount_fails() {
    let f = setup();
    let res = f.client.try_open(&f.buyer, &f.seller, &f.asset, &-1, &100);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn settle_by_seller() {
    let f = setup();
    let id = f.client.open(&f.buyer, &f.seller, &f.asset, &10, &100);
    let trade = f.client.settle(&id, &f.seller);
    assert_eq!(trade.phase, Phase::Settled);
}

#[test]
fn cancel_by_buyer() {
    let f = setup();
    let id = f.client.open(&f.buyer, &f.seller, &f.asset, &10, &100);
    let trade = f.client.cancel(&id, &f.buyer);
    assert_eq!(trade.phase, Phase::Cancelled);
}

#[test]
fn settle_by_stranger_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let id = f.client.open(&f.buyer, &f.seller, &f.asset, &10, &100);
    let res = f.client.try_settle(&id, &stranger);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn settle_twice_fails() {
    let f = setup();
    let id = f.client.open(&f.buyer, &f.seller, &f.asset, &10, &100);
    f.client.settle(&id, &f.seller);
    let res = f.client.try_settle(&id, &f.seller);
    assert_eq!(res, Err(Ok(Error::NotPending)));
}

#[test]
fn get_missing_fails() {
    let f = setup();
    assert_eq!(f.client.try_get(&7), Err(Ok(Error::TradeNotFound)));
}
