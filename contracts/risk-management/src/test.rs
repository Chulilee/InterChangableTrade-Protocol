#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: RiskManagerClient<'static>,
    pauser: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RiskManager, ());
    let client = RiskManagerClient::new(&env, &contract_id);
    let pauser = Address::generate(&env);
    client.initialize(&pauser, &1_000);
    Fixture {
        env,
        client,
        pauser,
    }
}

#[test]
fn initialize_twice_fails() {
    let f = setup();
    let other = Address::generate(&f.env);
    let res = f.client.try_initialize(&other, &2_000);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}

#[test]
fn check_limits_passes_within_size() {
    let f = setup();
    let trader = Address::generate(&f.env);
    // Under the max order size and not paused: allowed.
    f.client.check_limits(&500, &trader);
}

#[test]
fn check_limits_rejects_oversized_order() {
    let f = setup();
    let trader = Address::generate(&f.env);
    let res = f.client.try_check_limits(&1_001, &trader);
    assert_eq!(res, Err(Ok(RiskError::MaxOrderSizeExceeded)));
}

#[test]
fn pause_blocks_all_orders() {
    let f = setup();
    let trader = Address::generate(&f.env);

    f.client.pause_market(&f.pauser);
    // Even a within-size order is rejected while the market is paused.
    let res = f.client.try_check_limits(&500, &trader);
    assert_eq!(res, Err(Ok(RiskError::MarketPaused)));
}

#[test]
fn unpause_restores_trading() {
    let f = setup();
    let trader = Address::generate(&f.env);

    f.client.pause_market(&f.pauser);
    f.client.unpause_market(&f.pauser);
    // Trading resumes after unpause.
    f.client.check_limits(&500, &trader);
}

#[test]
fn only_pauser_can_pause() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_pause_market(&stranger);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}

#[test]
fn only_pauser_can_unpause() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_unpause_market(&stranger);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}

#[test]
fn set_limit_changes_threshold() {
    let f = setup();
    let trader = Address::generate(&f.env);

    // Originally 2_000 would be rejected (max is 1_000)...
    let res = f.client.try_check_limits(&2_000, &trader);
    assert_eq!(res, Err(Ok(RiskError::MaxOrderSizeExceeded)));

    // ...raise the limit, and the same order is now allowed.
    f.client.set_limit(&f.pauser, &5_000);
    f.client.check_limits(&2_000, &trader);
}

#[test]
fn only_pauser_can_set_limit() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_set_limit(&stranger, &9_999);
    assert_eq!(res, Err(Ok(RiskError::Unauthorized)));
}
