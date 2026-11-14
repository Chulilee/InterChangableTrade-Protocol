#![cfg(test)]

use super::*;
use access_control::{AccessControl, AccessControlClient};
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: RiskManagerClient<'static>,
    access_control: Address,
    pauser: Address,
    trader: Address,
}

// Set up a test fixture with a default environment, a registered RiskManager contract, and initialized state.
fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy the access-control contract the risk manager authorizes against,
    // grant the PAUSER role to the pauser account.
    let ac_id = env.register(AccessControl, ());
    let ac = AccessControlClient::new(&env, &ac_id);
    let admin = Address::generate(&env);
    ac.initialize(&admin);
    let pauser = Address::generate(&env);
    ac.grant_role(&symbol_short!("PAUSER"), &pauser, &admin);

    let contract_id = env.register(RiskManager, ());
    let client = RiskManagerClient::new(&env, &contract_id);
    let trader = Address::generate(&env);
    client.initialize(&admin, &ac_id, &1_000);
    Fixture {
        env,
        client,
        access_control: ac_id,
        pauser,
        trader,
    }
}

#[test]
fn initialize_sets_defaults() {
    let f = setup();
    assert!(!f.client.is_market_paused());
    assert_eq!(f.client.get_max_order_size(), 1_000);
    assert_eq!(f.client.get_pauser(), f.access_control);
    assert_eq!(f.client.get_access_control(), f.access_control);
    // Market starts unpaused and limits pass under the configured cap
    // (panics if the check fails).
    f.client.check_limits(&500, &f.trader);
}

#[test]
fn getters_reflect_pause_and_unpause() {
    let f = setup();
    assert!(!f.client.is_market_paused());

    f.client.pause_market(&f.pauser);
    assert!(f.client.is_market_paused());

    f.client.unpause_market(&f.pauser);
    assert!(!f.client.is_market_paused());
}

#[test]
fn get_max_order_size_reflects_set_limit() {
    let f = setup();
    assert_eq!(f.client.get_max_order_size(), 1_000);

    f.client.set_limit(&f.pauser, &2_500);
    assert_eq!(f.client.get_max_order_size(), 2_500);
}

#[test]
fn getters_on_uninitialized_contract() {
    let env = Env::default();
    let contract_id = env.register(RiskManager, ());
    let client = RiskManagerClient::new(&env, &contract_id);

    assert!(!client.is_market_paused());
    assert_eq!(client.get_max_order_size(), 0);
    assert_eq!(client.try_get_pauser(), Err(Ok(RiskError::NotInitialized)));
    assert_eq!(
        client.try_get_access_control(),
        Err(Ok(RiskError::NotInitialized))
    );
}

#[test]
fn order_over_max_size_fails() {
    let f = setup();
    let res = f.client.try_check_limits(&1_001, &f.trader);
    assert_eq!(res, Err(Ok(RiskError::MaxOrderSizeExceeded)));
}

#[test]
fn order_at_max_size_passes() {
    let f = setup();
    f.client.check_limits(&1_000, &f.trader);
}

#[test]
fn pause_blocks_orders() {
    let f = setup();
    f.client.pause_market(&f.pauser);
    let res = f.client.try_check_limits(&10, &f.trader);
    assert_eq!(res, Err(Ok(RiskError::MarketPaused)));
}

#[test]
fn unpause_restores_orders() {
    let f = setup();
    f.client.pause_market(&f.pauser);
    f.client.unpause_market(&f.pauser);
    f.client.check_limits(&10, &f.trader);
}

#[test]
fn pause_by_non_pauser_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_pause_market(&stranger);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}

#[test]
fn set_limit_changes_cap() {
    let f = setup();
    f.client.set_limit(&f.pauser, &50);
    let res = f.client.try_check_limits(&60, &f.trader);
    assert_eq!(res, Err(Ok(RiskError::MaxOrderSizeExceeded)));
}

#[test]
fn set_limit_by_non_pauser_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_set_limit(&stranger, &50);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}

#[test]
fn exposure_accumulates_across_orders() {
    let f = setup();
    f.client.check_limits(&300, &f.trader);
    assert_eq!(f.client.get_exposure(&f.trader), 300);
    f.client.check_limits(&200, &f.trader);
    assert_eq!(f.client.get_exposure(&f.trader), 500);
    f.client.check_limits(&100, &f.trader);
    assert_eq!(f.client.get_exposure(&f.trader), 600);
}

#[test]
fn exposure_limit_breached_fails() {
    let f = setup();
    f.client.check_limits(&600, &f.trader);
    let res = f.client.try_check_limits(&500, &f.trader);
    assert_eq!(res, Err(Ok(RiskError::CumulativeExposureExceeded)));
}

#[test]
fn exposure_is_per_trader() {
    let f = setup();
    let trader2 = Address::generate(&f.env);
    f.client.check_limits(&400, &f.trader);
    f.client.check_limits(&200, &trader2);
    assert_eq!(f.client.get_exposure(&f.trader), 400);
    assert_eq!(f.client.get_exposure(&trader2), 200);
}

#[test]
fn get_exposure_returns_zero_for_new_trader() {
    let f = setup();
    let new_trader = Address::generate(&f.env);
    assert_eq!(f.client.get_exposure(&new_trader), 0);
}
