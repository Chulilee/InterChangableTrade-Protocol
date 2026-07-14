#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

fn setup() -> (Env, AssetRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AssetRegistry, ());
    let client = AssetRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn initialize_is_idempotent_guarded() {
    let (_env, client, admin) = setup();
    let res = client.try_initialize(&admin);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn register_and_get() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    client.register(&token, &symbol_short!("USDC"));

    let asset = client.get(&token);
    assert_eq!(asset.symbol, symbol_short!("USDC"));
    assert!(asset.enabled);
    assert_eq!(client.list().len(), 1);
}

#[test]
fn register_duplicate_fails() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    client.register(&token, &symbol_short!("USDC"));
    let res = client.try_register(&token, &symbol_short!("USDC"));
    assert_eq!(res, Err(Ok(Error::AssetExists)));
}

#[test]
fn remove_asset() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    client.register(&token, &symbol_short!("USDC"));
    client.remove(&token);
    assert_eq!(client.list().len(), 0);
    assert_eq!(client.try_get(&token), Err(Ok(Error::AssetNotFound)));
}

#[test]
fn remove_missing_fails() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    assert_eq!(client.try_remove(&token), Err(Ok(Error::AssetNotFound)));
}
