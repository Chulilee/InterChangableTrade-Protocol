#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: MarketplaceClient<'static>,
    seller: Address,
    asset: Address,
    quote: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Marketplace, ());
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    Fixture {
        seller: Address::generate(&env),
        asset: Address::generate(&env),
        quote: Address::generate(&env),
        client,
        env,
    }
}

#[test]
fn create_and_get_listing() {
    let f = setup();
    let id = f.client.create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let listing = f.client.get(&id);
    assert_eq!(listing.amount, 10);
    assert_eq!(listing.price, 100);
    assert_eq!(listing.status, Status::Open);
}

#[test]
fn create_with_invalid_amount_fails() {
    let f = setup();
    let res = f.client.try_create_listing(&f.seller, &f.asset, &f.quote, &0, &100);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn fill_listing_marks_filled() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let id = f.client.create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let filled = f.client.fill_listing(&id, &buyer);
    assert_eq!(filled.status, Status::Filled);
    assert_eq!(f.client.get(&id).status, Status::Filled);
}

#[test]
fn fill_twice_fails() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let id = f.client.create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    f.client.fill_listing(&id, &buyer);
    let res = f.client.try_fill_listing(&id, &buyer);
    assert_eq!(res, Err(Ok(Error::ListingNotOpen)));
}

#[test]
fn cancel_listing() {
    let f = setup();
    let id = f.client.create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    f.client.cancel_listing(&id, &f.seller);
    assert_eq!(f.client.get(&id).status, Status::Cancelled);
}

#[test]
fn cancel_by_non_seller_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let id = f.client.create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let res = f.client.try_cancel_listing(&id, &stranger);
    assert_eq!(res, Err(Ok(Error::NotSeller)));
}

#[test]
fn get_missing_listing_fails() {
    let f = setup();
    assert_eq!(f.client.try_get(&999), Err(Ok(Error::ListingNotFound)));
}
