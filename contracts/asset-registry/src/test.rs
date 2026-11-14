#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _},
    Address, Env, IntoVal,
};

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

#[test]
fn set_asset_enabled_round_trip_and_list_enabled() {
    let (env, client, _admin) = setup();
    let usdc = Address::generate(&env);
    let xlm = Address::generate(&env);
    client.register(&usdc, &symbol_short!("USDC"));
    client.register(&xlm, &symbol_short!("XLM"));

    assert_eq!(client.list().len(), 2);
    assert_eq!(client.list_enabled().len(), 2);

    client.set_asset_enabled(&usdc, &false);
    assert!(!client.get(&usdc).enabled);
    assert!(client.get(&xlm).enabled);

    let enabled = client.list_enabled();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled.get_unchecked(0), xlm);

    client.set_asset_enabled(&usdc, &true);
    assert!(client.get(&usdc).enabled);
    assert_eq!(client.list_enabled().len(), 2);
}

#[test]
fn set_asset_enabled_emits_event() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    client.register(&token, &symbol_short!("USDC"));

    client.set_asset_enabled(&token, &false);
    let events = env.events().all();
    let last = events.last().unwrap();
    assert_eq!(last.2, false.into_val(&env));
}

#[test]
fn set_asset_enabled_unknown_asset_fails() {
    let (env, client, _admin) = setup();
    let unknown = Address::generate(&env);
    let res = client.try_set_asset_enabled(&unknown, &false);
    assert_eq!(res, Err(Ok(Error::AssetNotFound)));
}

#[test]
fn set_asset_enabled_requires_admin_auth() {
    let (env, client, _admin) = setup();
    let token = Address::generate(&env);
    client.register(&token, &symbol_short!("USDC"));

    env.mock_auths(&[]);
    let res = client.try_set_asset_enabled(&token, &false);
    assert!(res.is_err());
    assert!(client.get(&token).enabled);
}
