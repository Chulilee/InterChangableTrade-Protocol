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

#[test]
fn release_batch_success() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &600);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &100);
    f.client.fund(&2, &f.buyer, &f.seller, &f.token, &200);
    f.client.fund(&3, &f.buyer, &f.seller, &f.token, &300);

    let mut ids = Vec::new(&f.env);
    ids.push_back(1);
    ids.push_back(2);
    ids.push_back(3);

    let count = f.client.release_batch(&ids);
    assert_eq!(count, 3);

    assert_eq!(f.client.get(&1).state, State::Released);
    assert_eq!(f.client.get(&2).state, State::Released);
    assert_eq!(f.client.get(&3).state, State::Released);

    let token_client = token::Client::new(&f.env, &f.token);
    assert_eq!(token_client.balance(&f.seller), 600);
    assert_eq!(token_client.balance(&f.contract_id), 0);
}

#[test]
fn refund_batch_success() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &400);
    f.client.fund(&10, &f.buyer, &f.seller, &f.token, &150);
    f.client.fund(&11, &f.buyer, &f.seller, &f.token, &250);

    let mut ids = Vec::new(&f.env);
    ids.push_back(10);
    ids.push_back(11);

    let count = f.client.refund_batch(&ids);
    assert_eq!(count, 2);

    assert_eq!(f.client.get(&10).state, State::Refunded);
    assert_eq!(f.client.get(&11).state, State::Refunded);

    let token_client = token::Client::new(&f.env, &f.token);
    assert_eq!(token_client.balance(&f.buyer), 400);
    assert_eq!(token_client.balance(&f.contract_id), 0);
}

#[test]
fn batch_empty_fails() {
    let f = setup();
    let empty_ids = Vec::new(&f.env);
    assert_eq!(
        f.client.try_release_batch(&empty_ids),
        Err(Ok(Error::InvalidBatch))
    );
    assert_eq!(
        f.client.try_refund_batch(&empty_ids),
        Err(Ok(Error::InvalidBatch))
    );
}

#[test]
fn batch_cap_violation_fails() {
    let f = setup();
    let mut ids = Vec::new(&f.env);
    for i in 1..=21 {
        ids.push_back(i);
    }
    assert_eq!(
        f.client.try_release_batch(&ids),
        Err(Ok(Error::BatchTooLarge))
    );
    assert_eq!(
        f.client.try_refund_batch(&ids),
        Err(Ok(Error::BatchTooLarge))
    );
}

#[test]
fn release_batch_atomic_rollback_on_missing_escrow() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &300);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &100);
    // Note: trade 2 is intentionally NOT funded
    f.client.fund(&3, &f.buyer, &f.seller, &f.token, &200);

    let mut ids = Vec::new(&f.env);
    ids.push_back(1);
    ids.push_back(2);
    ids.push_back(3);

    let res = f.client.try_release_batch(&ids);
    assert_eq!(res, Err(Ok(Error::EscrowNotFound)));

    // Strict-all rollback: trade 1 was NOT released
    assert_eq!(f.client.get(&1).state, State::Funded);
    assert_eq!(f.client.get(&3).state, State::Funded);

    let token_client = token::Client::new(&f.env, &f.token);
    assert_eq!(token_client.balance(&f.seller), 0);
    assert_eq!(token_client.balance(&f.contract_id), 300);
}

#[test]
fn refund_batch_atomic_rollback_on_mixed_state() {
    let f = setup();
    f.token_admin.mint(&f.buyer, &300);
    f.client.fund(&1, &f.buyer, &f.seller, &f.token, &100);
    f.client.fund(&2, &f.buyer, &f.seller, &f.token, &200);

    // Release trade 1 first so it is not in Funded state
    f.client.release(&1);

    let mut ids = Vec::new(&f.env);
    ids.push_back(1);
    ids.push_back(2);

    let res = f.client.try_refund_batch(&ids);
    assert_eq!(res, Err(Ok(Error::NotFunded)));

    // Trade 2 remains Funded (not refunded)
    assert_eq!(f.client.get(&2).state, State::Funded);
}
