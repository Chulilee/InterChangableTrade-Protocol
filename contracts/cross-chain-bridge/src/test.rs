#![cfg(test)]
extern crate std;
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
    token,
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

    client.initialize(&admin, &validators, &required_attestations).unwrap();

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
fn test_initialize_twice_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();
    let res = client.try_initialize(&admin, &validators, &required_attestations);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_lock_asset() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let transaction: CrossChainTransaction =
        env.storage().instance().get(&DataKey::Transaction(tx_id)).unwrap();
    assert_eq!(transaction.id, tx_id);
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
fn test_lock_asset_when_paused_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();
    client.pause_bridge().unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let res = client.try_lock_asset(&user, &asset, &amount, &to_chain, &to_address);
    assert_eq!(res, Err(Ok(Error::BridgePaused)));
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

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let wrapped_asset_id = env.register_stellar_asset_contract(asset.clone());
    client.register_asset(&asset, &wrapped_asset_id).unwrap();

    client.attest_transaction(&validator, &tx_id).unwrap();

    let transaction: CrossChainTransaction =
        env.storage().instance().get(&DataKey::Transaction(tx_id)).unwrap();
    assert_eq!(transaction.status, TransactionStatus::Completed);

    let token_client = token::Client::new(&env, &wrapped_asset_id);
    assert_eq!(token_client.balance(&user), amount);
}

#[test]
fn test_attest_transaction_invalid_validator_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let invalid_validator = Address::random(&env);
    let res = client.try_attest_transaction(&invalid_validator, &tx_id);
    assert_eq!(res, Err(Ok(Error::NotActiveValidator)));
}

#[test]
fn test_attest_transaction_duplicate_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let wrapped_asset_id = env.register_stellar_asset_contract(asset.clone());
    client.register_asset(&asset, &wrapped_asset_id).unwrap();

    client.attest_transaction(&validator, &tx_id).unwrap();

    let res = client.try_attest_transaction(&validator, &tx_id);
    assert_eq!(res, Err(Ok(Error::DuplicateAttestation)));
}

#[test]
fn test_add_validator() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let new_validator = Address::random(&env);
    client.add_validator(&new_validator).unwrap();

    let state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 2);
}

#[test]
fn test_add_duplicate_validator_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let res = client.try_add_validator(&validator);
    assert_eq!(res, Err(Ok(Error::ValidatorAlreadyExists)));
}

#[test]
fn test_remove_validator() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    client.remove_validator(&validator).unwrap();

    let state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 0);
}

#[test]
fn test_remove_nonexistent_validator_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let nonexistent = Address::random(&env);
    let res = client.try_remove_validator(&nonexistent);
    assert_eq!(res, Err(Ok(Error::ValidatorNotFound)));
}

#[test]
fn test_emergency_withdraw() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    client.emergency_withdraw(&user, &tx_id).unwrap();

    let transaction: CrossChainTransaction =
        env.storage().instance().get(&DataKey::Transaction(tx_id)).unwrap();
    assert_eq!(transaction.status, TransactionStatus::Cancelled);
}

#[test]
fn test_emergency_withdraw_not_owner_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::random(&env));
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let stranger = Address::random(&env);
    let res = client.try_emergency_withdraw(&stranger, &tx_id);
    assert_eq!(res, Err(Ok(Error::NotTransactionOwner)));
}

#[test]
fn test_emergency_withdraw_invalid_state_fails() {
    let env = Env::default();
    let client = create_bridge_contract(&env);

    let admin = Address::random(&env);
    let validator = Address::random(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(validator.clone());
    let required_attestations = 1;

    client.initialize(&admin, &validators, &required_attestations).unwrap();

    let user = Address::random(&env);
    let asset = Address::random(&env);
    let amount = 100;
    let to_chain = String::from_str(&env, "ETH");
    let to_address = String::from_str(&env, "0x1234");

    let tx_id = client.lock_asset(&user, &asset, &amount, &to_chain, &to_address).unwrap();

    let wrapped_asset_id = env.register_stellar_asset_contract(asset.clone());
    client.register_asset(&asset, &wrapped_asset_id).unwrap();

    client.attest_transaction(&validator, &tx_id).unwrap();

    let res = client.try_emergency_withdraw(&user, &tx_id);
    assert_eq!(res, Err(Ok(Error::InvalidTransactionState)));
}