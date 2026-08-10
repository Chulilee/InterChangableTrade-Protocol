#![cfg(test)]

use super::{DataKey, Error, OracleSourceType, PriceOracle, PriceOracleClient};
use access_control::{AccessControl, AccessControlClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env,
};

struct Fixture {
    env: Env,
    client: PriceOracleClient<'static>,
    admin: Address,
    operator: Address,
}

/// Deploy access-control + price-oracle, wire them together, and grant the
/// operator role so price updates are authorized.
fn setup(max_deviation_bps: u32, freshness: u64, twap_window: u64) -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(10_000);

    let admin = Address::generate(&env);

    // Access control, initialized with `admin` holding ROLE_ADMIN.
    let ac_address = env.register(AccessControl, ());
    let ac = AccessControlClient::new(&env, &ac_address);
    ac.initialize(&admin);

    // Grant the operator role to a dedicated operator account.
    let operator = Address::generate(&env);
    ac.grant_role(&symbol_short!("OPERATOR"), &operator, &admin);

    // Price oracle pointed at the access-control contract.
    let oracle_address = env.register(PriceOracle, ());
    let client = PriceOracleClient::new(&env, &oracle_address);
    client.initialize(&ac_address, &max_deviation_bps, &freshness, &twap_window);

    Fixture {
        env,
        client,
        admin,
        operator,
    }
}

#[test]
fn initialize_twice_fails() {
    let f = setup(500, 3600, 86400);
    let other = Address::generate(&f.env);
    let res = f.client.try_initialize(&other, &500, &3600, &86400);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn set_source_makes_first_source_active() {
    let f = setup(500, 3600, 86400);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    let active: OracleSourceType = f
        .env
        .as_contract(&f.client.address, || {
            f.env.storage().instance().get(&DataKey::ActiveSource)
        })
        .unwrap();
    assert_eq!(active, OracleSourceType::Primary);
}

#[test]
fn non_admin_cannot_set_source() {
    let f = setup(500, 3600, 86400);
    let stranger = Address::generate(&f.env);
    let primary = Address::generate(&f.env);
    let res = f
        .client
        .try_set_oracle_source(&stranger, &OracleSourceType::Primary, &primary);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn update_and_get_price() {
    let f = setup(500, 3600, 86400);
    let asset = Address::generate(&f.env);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    f.client.update_price(&f.operator, &asset, &1_000);
    assert_eq!(f.client.get_price(&asset), 1_000);
}

#[test]
fn non_operator_cannot_update_price() {
    let f = setup(500, 3600, 86400);
    let asset = Address::generate(&f.env);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    let stranger = Address::generate(&f.env);
    let res = f.client.try_update_price(&stranger, &asset, &1_000);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn price_deviation_is_rejected_without_state_change() {
    let f = setup(500, 3600, 86400); // 5% max deviation
    let asset = Address::generate(&f.env);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    f.client.update_price(&f.operator, &asset, &1_000);

    // An 11% jump exceeds the 5% band and is rejected outright.
    let res = f.client.try_update_price(&f.operator, &asset, &1_110);
    assert_eq!(res, Err(Ok(Error::PriceDeviationExceeded)));

    // The rejected tick leaves the stored price and the active source
    // untouched: a single outlier can neither move the oracle nor force a
    // failover.
    assert_eq!(f.client.get_price(&asset), 1_000);
    let active: OracleSourceType = f
        .env
        .as_contract(&f.client.address, || {
            f.env.storage().instance().get(&DataKey::ActiveSource)
        })
        .unwrap();
    assert_eq!(active, OracleSourceType::Primary);
}

#[test]
fn stale_price_is_rejected() {
    let f = setup(500, 3600, 86400); // 1 hour freshness
    let asset = Address::generate(&f.env);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    f.client.update_price(&f.operator, &asset, &1_000);

    // Jump forward well past the freshness threshold: stale data is never
    // served.
    f.env.ledger().set_timestamp(10_000 + 7_200);
    let res = f.client.try_get_price(&asset);
    assert_eq!(res, Err(Ok(Error::PriceStale)));
}

#[test]
fn admin_can_failover_to_secondary() {
    let f = setup(500, 3600, 86400);
    let primary = Address::generate(&f.env);
    let secondary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Secondary, &secondary);

    // Primary is active by default; the admin promotes the secondary. This
    // returns Ok, so the switch commits.
    f.client
        .set_active_source(&f.admin, &OracleSourceType::Secondary);

    let active: OracleSourceType = f
        .env
        .as_contract(&f.client.address, || {
            f.env.storage().instance().get(&DataKey::ActiveSource)
        })
        .unwrap();
    assert_eq!(active, OracleSourceType::Secondary);
}

#[test]
fn failover_requires_admin() {
    let f = setup(500, 3600, 86400);
    let secondary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Secondary, &secondary);

    let stranger = Address::generate(&f.env);
    let res = f
        .client
        .try_set_active_source(&stranger, &OracleSourceType::Secondary);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn failover_to_unconfigured_source_fails() {
    let f = setup(500, 3600, 86400);
    let primary = Address::generate(&f.env);
    f.client
        .set_oracle_source(&f.admin, &OracleSourceType::Primary, &primary);

    // Secondary was never configured, so it cannot be promoted.
    let res = f
        .client
        .try_set_active_source(&f.admin, &OracleSourceType::Secondary);
    assert_eq!(res, Err(Ok(Error::InvalidSource)));
}
