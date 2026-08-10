#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env};

struct Fixture {
    env: Env,
    client: EscrowContractClient<'static>,
    contract_id: Address,
    buyer: Address,
    seller: Address,
    token: Address,
    token_admin: token::StellarAssetClient<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    // The escrow contract (non-root) invokes the token contract, which in turn
    // requires auth from the buyer/seller: mock those non-root auths too.
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(EscrowContract, ());
    let client = EscrowContractClient::new(&env, &contract_id);

    let token_admin_addr = Address::generate(&env);
    let token_sac = env.register_stellar_asset_contract_v2(token_admin_addr);
    let token = token_sac.address();
    let token_admin = token::StellarAssetClient::new(&env, &token);

    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);

    Fixture {
        env,
        client,
        contract_id,
        buyer,
        seller,
        token,
        token_admin,
    }
}

#[test]
fn fund_pulls_tokens_into_escrow() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &500);
    let token_client = token::Client::new(&f.env, &f.token);

    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);

    let e = f.client.get(&1);
    assert_eq!(e.amount, 500);
    assert_eq!(e.state, State::Funded);

    // The deposit actually moved: buyer paid, the contract holds the funds.
    assert_eq!(token_client.balance(&f.buyer), 0);
    assert_eq!(token_client.balance(&f.contract_id), 500);
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
    f.token_admin.mint(&f.buyer, &1000);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);
    let res = f.client.try_fund(&1, &f.buyer, &f.seller, &f.token, &500);
    assert_eq!(res, Err(Ok(Error::EscrowExists)));
}

#[test]
fn fund_without_balance_fails_atomically() {
    let f = setup();
    // The buyer has no tokens: the token transfer traps.
    let res = f.client.try_fund(&1, &f.buyer, &f.seller, &f.token, &500);
    assert!(res.is_err());

    // Nothing was recorded: the failed transfer rolled the state write back.
    assert_eq!(f.client.try_get(&1), Err(Ok(Error::EscrowNotFound)));
}

#[test]
fn release_pays_seller() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &500);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);

    let e = f.client.release(&1);
    assert_eq!(e.state, State::Released);

    // The funds left the contract and landed with the seller.
    let token_client = token::Client::new(&f.env, &f.token);
    assert_eq!(token_client.balance(&f.seller), 500);
    assert_eq!(token_client.balance(&f.buyer), 0);
    assert_eq!(token_client.balance(&f.contract_id), 0);
}

#[test]
fn refund_returns_to_buyer() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &500);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &500);

    let e = f.client.refund(&1);
    assert_eq!(e.state, State::Refunded);

    // The funds came back to the buyer; the contract holds nothing.
    let token_client = token::Client::new(&f.env, &f.token);
    assert_eq!(token_client.balance(&f.buyer), 500);
    assert_eq!(token_client.balance(&f.seller), 0);
    assert_eq!(token_client.balance(&f.contract_id), 0);
}

#[test]
fn release_twice_fails() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &500);
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
