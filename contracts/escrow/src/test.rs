#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    client: EscrowContractClient<'static>,
    buyer: Address,
    seller: Address,
    token: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);
    Fixture {
        buyer: Address::generate(&env),
        seller: Address::generate(&env),
        token: Address::generate(&env),
        client,
    }
}

#[test]
fn fund_and_get() {
    let f = setup();
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    let e = f.client.get(&1);
    assert_eq!(e.amount, 500);
    assert_eq!(e.state, State::Funded);
}

#[test]
fn fund_invalid_amount_fails() {
    let f = setup();
    let res = f.client.try_fund(&1, &f.buyer, &f.seller, &f.token, &0);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn fund_duplicate_fails() {
    let f = setup();
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    let res = f.client.try_fund(&1, &f.buyer, &f.seller, &f.token, &500);
    assert_eq!(res, Err(Ok(Error::EscrowExists)));
}

#[test]
fn release_to_seller() {
    let f = setup();
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    let e = f.client.release(&1);
    assert_eq!(e.state, State::Released);
}

#[test]
fn refund_to_buyer() {
    let f = setup();
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    let e = f.client.refund(&1);
    assert_eq!(e.state, State::Refunded);
}

#[test]
fn release_twice_fails() {
    let f = setup();
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    f.client.release(&1);
    let res = f.client.try_release(&1);
    assert_eq!(res, Err(Ok(Error::NotFunded)));
}

#[test]
fn get_missing_fails() {
    let f = setup();
    assert_eq!(f.client.try_get(&42), Err(Ok(Error::EscrowNotFound)));
}
