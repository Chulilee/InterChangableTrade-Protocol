#![cfg(test)]
extern crate std;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

fn create_bridge_contract<'a>(env: &Env) -> CrossChainBridgeClient<'a> {
    let contract_id = env.register_contract(None, CrossChainBridge);
    CrossChainBridgeClient::new(env, &contract_id)
}


#[test]
fn test_initialize() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    validators.push_back(Address::random(&env));
    let required_attestations = 2;

    client.initialize(&admin, &validators, &required_attestations);

    let state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 2);
    assert_eq!(state.required_attestations, 2);
    assert_eq!(state.is_paused, false);
    assert_eq!(state.next_transaction_id, 0);

    let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    assert_eq!(stored_admin, admin);

    let stored_validators: Vec<Validator> = env.storage().instance().get(&DataKey::Validators).unwrap();
    assert_eq!(stored_validators.len(), 2);
}

#[test]
fn test_lock_asset() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations);

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    client.lock_asset(&user, &asset, &amount, &to_chain, &to_address);

    let transaction: CrossChainTransaction =
        env.storage().instance().get(&DataKey::Transaction(0)).unwrap();
    assert_eq!(transaction.id, 0);
    assert_eq!(transaction.user, user);
    assert_eq!(transaction.asset, asset);
    assert_eq!(transaction.amount, amount);
    assert_eq!(transaction.to_chain, to_chain);
    assert_eq!(transaction.to_address, to_address);
    assert_eq!(transaction.status, TransactionStatus::Pending);

    let state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.next_transaction_id, 1);
}

#[test]
fn test_attest_transaction() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations);

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    client.lock_asset(&user, &asset, &amount, &to_chain, &to_address);

    let wrapped_asset_id = env.register_stellar_asset_contract(asset.clone());
    client.register_asset(&asset, &wrapped_asset_id);

    client.attest_transaction(&validator, &0);

    let transaction: CrossChainTransaction =
        env.storage().instance().get(&DataKey::Transaction(0)).unwrap();
    assert_eq!(transaction.status, TransactionStatus::Completed);

    let token_client = token::Client::new(&env, &wrapped_asset_id);
    assert_eq!(token_client.balance(&user), amount);
}