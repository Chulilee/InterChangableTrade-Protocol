#![cfg(test)]
use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: MarginingLiquidationClient<'static>,
    price_oracle: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MarginingLiquidation, ());
    let client = MarginingLiquidationClient::new(&env, &contract_id);
    let price_oracle = Address::generate(&env);
    client.initialize(
        &100_000_000_000_000_000, // 0.1 = 10% initial margin
        &50_000_000_000_000_000,  // 0.05 = 5% maintenance margin
        &50_000_000_000_000_000,  // 0.05 = 5% liquidation incentive
        &price_oracle,
    );
    Fixture {
        env,
        client,
        price_oracle,
    }
}

#[test]
fn test_initialize_twice_fails() {
    let f = setup();
    let res = f.client.try_initialize(
        &100_000_000_000_000_000,
        &50_000_000_000_000_000,
        &50_000_000_000_000_000,
        &f.price_oracle,
    );
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_deposit_and_withdraw_collateral() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &asset, &1000);
    let account = f.client.get_margin_account(&user);
    assert_eq!(
        account.collateral_balances.get(asset.clone()).unwrap(),
        1000
    );

    f.client.withdraw_collateral(&user, &asset, &400);
    let account = f.client.get_margin_account(&user);
    assert_eq!(account.collateral_balances.get(asset).unwrap(), 600);
}

#[test]
fn test_withdraw_more_than_balance_fails() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &asset, &500);
    let res = f.client.try_withdraw_collateral(&user, &asset, &600);
    assert_eq!(res, Err(Ok(Error::InsufficientCollateral)));
}

#[test]
fn test_open_and_close_position() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &quote, &200);

    let position_id = f
        .client
        .open_position(&user, &asset, &quote, &100, &10, &true);
    assert_eq!(position_id, 0);

    let position = f.client.get_position(&position_id);
    assert!(position.is_active);
    assert_eq!(position.size, 100);
    assert_eq!(position.entry_price, 10);

    f.client.close_position(&user, &position_id);
    let position = f.client.get_position(&position_id);
    assert!(!position.is_active);
}

#[test]
fn test_open_position_insufficient_margin_fails() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &quote, &50);
    let res = f
        .client
        .try_open_position(&user, &asset, &quote, &100, &10, &true);
    assert_eq!(res, Err(Ok(Error::InsufficientMargin)));
}

#[test]
fn test_margin_check_healthy_account() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &quote, &200);
    f.client
        .open_position(&user, &asset, &quote, &100, &10, &true);
    assert!(f.client.check_margin(&user));
}

#[test]
fn test_only_oracle_can_update_mark_price() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);
    let stranger = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &quote, &200);
    let position_id = f
        .client
        .open_position(&user, &asset, &quote, &100, &10, &true);

    let res = f.client.try_update_mark_price(&stranger, &position_id, &5);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_liquidation_triggered_when_undercollateralized() {
    let f = setup();
    let user = Address::generate(&f.env);
    let liquidator = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);

    // Deposit just enough to open the position (notional 1000, 10% margin = 100).
    f.client.deposit_collateral(&user, &quote, &100);
    let position_id = f
        .client
        .open_position(&user, &asset, &quote, &100, &10, &true);

    // Account is healthy at entry price.
    assert!(f.client.check_margin(&user));

    // Oracle pushes an adverse mark price: long position, price falls 10 -> 2.
    // Unrealized P&L = 100 * (2 - 10) = -800, wiping out equity below maintenance.
    f.client
        .update_mark_price(&f.price_oracle, &position_id, &2);
    assert!(!f.client.check_margin(&user));

    // Liquidator can now liquidate the undercollateralized account.
    let incentive = f.client.trigger_liquidation(&user, &liquidator);
    assert!(incentive >= 0);

    // Position is closed and collateral is drained.
    let position = f.client.get_position(&position_id);
    assert!(!position.is_active);
    let account = f.client.get_margin_account(&user);
    assert_eq!(account.collateral_balances.get(quote).unwrap_or(0), 0);
}

#[test]
fn test_healthy_account_cannot_be_liquidated() {
    let f = setup();
    let user = Address::generate(&f.env);
    let liquidator = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);

    f.client.deposit_collateral(&user, &quote, &200);
    f.client
        .open_position(&user, &asset, &quote, &100, &10, &true);

    let res = f.client.try_trigger_liquidation(&user, &liquidator);
    assert_eq!(res, Err(Ok(Error::PositionStillHealthy)));
}
