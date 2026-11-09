#![cfg(test)]

use super::*;
use risk_management::{RiskManager, RiskManagerClient};
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

    // Deploy and wire a risk manager: create_listing/fill_listing consult it
    // for order-size limits on every call.
    let rm_address = env.register(RiskManager, ());
    let rm = RiskManagerClient::new(&env, &rm_address);
    rm.initialize(&admin, &1_000_000);
    client.set_risk_manager(&rm_address);

    Fixture {
        seller: Address::generate(&env),
        asset: Address::generate(&env),
        quote: Address::generate(&env),
        client,
        env,
    }
}

fn setup_no_risk_manager() -> Fixture {
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
    let id = f
        .client
        .create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let listing = f.client.get(&id);
    assert_eq!(listing.amount, 10);
    assert_eq!(listing.price, 100);
    assert_eq!(listing.status, Status::Open);
}

#[test]
fn create_with_invalid_amount_fails() {
    let f = setup();
    let res = f
        .client
        .try_create_listing(&f.seller, &f.asset, &f.quote, &0, &100);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn create_with_invalid_price_fails() {
    let f = setup();
    let res = f
        .client
        .try_create_listing(&f.seller, &f.asset, &f.quote, &10, &0);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn initialize_rejects_unauthorized_admin() {
    let env = Env::default();
    // No mock_all_auths here: initialize requires the admin's explicit signature.
    let contract_id = env.register(Marketplace, ());
    let client = MarketplaceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let res = client.try_initialize(&admin);
    // Without authorized signer on `admin`, the host auth mechanism rejects the call.
    assert!(res.is_err());
}

#[test]
fn fill_listing_marks_filled() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let id = f
        .client
        .create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let filled = f.client.fill_listing(&id, &buyer);
    assert_eq!(filled.status, Status::Filled);
    assert_eq!(f.client.get(&id).status, Status::Filled);
}

#[test]
fn fill_twice_fails() {
    let f = setup();
    let buyer = Address::generate(&f.env);
    let id = f
        .client
        .create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    f.client.fill_listing(&id, &buyer);
    let res = f.client.try_fill_listing(&id, &buyer);
    assert_eq!(res, Err(Ok(Error::ListingNotOpen)));
}

#[test]
fn fill_by_seller_self_fill_fails() {
    let f = setup();
    let id = f
        .client
        .create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    let res = f.client.try_fill_listing(&id, &f.seller);
    assert_eq!(res, Err(Ok(Error::SelfFill)));
}

#[test]
fn create_without_risk_manager_fails_cleanly() {
    let f = setup_no_risk_manager();
    let res = f
        .client
        .try_create_listing(&f.seller, &f.asset, &f.quote, &10, &100);
    assert_eq!(res, Err(Ok(Error::RiskManagerNotConfigured)));
}
