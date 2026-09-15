#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: RiskManagerClient<'static>,
    pauser: Address,
    trader: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RiskManager, ());
    let client = RiskManagerClient::new(&env, &contract_id);
    let pauser = Address::generate(&env);
    let trader = Address::generate(&env);
    client.initialize(&pauser, &1_000);
    Fixture {
        env,
        client,
        pauser,
        trader,
    }
}

#[test]
fn initialize_sets_defaults() {
    let f = setup();
    // Market starts unpaused and limits pass under the configured cap
    // (panics if the check fails).
    f.client.check_limits(&500, &f.trader);
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
