#![cfg(test)]
use super::*;
use soroban_sdk::{contract, contracterror, contractimpl, testutils::Address as _, Address, Env};

/// Mock price oracle exposing the same `get_price(asset)` interface as the
/// production price-oracle contract. Prices are scaled to 18 decimals.
#[contract]
pub struct MockPriceOracle;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MockOracleError {
    AssetNotFound = 1,
}

#[contractimpl]
impl MockPriceOracle {
    /// Set the 18-decimal price for `asset`.
    pub fn set_price(env: Env, asset: Address, price: i128) {
        env.storage().persistent().set(&asset, &price);
    }

    /// Mirror the production `get_price` interface.
    pub fn get_price(env: Env, asset: Address) -> Result<i128, MockOracleError> {
        env.storage()
            .persistent()
            .get(&asset)
            .ok_or(MockOracleError::AssetNotFound)
    }
}

/// 1.0 expressed with the 18-decimal scaling used by the price oracle.
const PRICE_ONE: i128 = 1_000_000_000_000_000_000;

struct Fixture {
    env: Env,
    client: MarginingLiquidationClient<'static>,
    price_oracle: Address,
}

impl Fixture {
    fn set_price(&self, asset: &Address, price: i128) {
        MockPriceOracleClient::new(&self.env, &self.price_oracle).set_price(asset, &price);
    }
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MarginingLiquidation, ());
    let client = MarginingLiquidationClient::new(&env, &contract_id);
    let price_oracle = env.register(MockPriceOracle, ());
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
    f.set_price(&asset, &PRICE_ONE);

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
    f.set_price(&quote, &PRICE_ONE);

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
    f.set_price(&quote, &PRICE_ONE);

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
    f.set_price(&quote, &PRICE_ONE);

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
    f.set_price(&quote, &PRICE_ONE);

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
    f.set_price(&quote, &PRICE_ONE);

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
    assert_eq!(incentive, 5); // 5% of the 100 units of oracle-priced collateral

    // Position is closed and collateral is drained.
    let position = f.client.get_position(&position_id);
    assert!(!position.is_active);
    let account = f.client.get_margin_account(&user);
    assert_eq!(account.collateral_balances.get(quote), Some(0));
}

#[test]
fn test_healthy_account_cannot_be_liquidated() {
    let f = setup();
    let user = Address::generate(&f.env);
    let liquidator = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);
    f.set_price(&quote, &PRICE_ONE);

    f.client.deposit_collateral(&user, &quote, &200);
    f.client
        .open_position(&user, &asset, &quote, &100, &10, &true);

    let res = f.client.try_trigger_liquidation(&user, &liquidator);
    assert_eq!(res, Err(Ok(Error::PositionStillHealthy)));
}

#[test]
fn test_collateral_value_uses_oracle_prices() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset_a = Address::generate(&f.env);
    let asset_b = Address::generate(&f.env);

    // asset_a is worth 2.0, asset_b is worth 0.5 (18-decimal prices).
    f.set_price(&asset_a, &2_000_000_000_000_000_000);
    f.set_price(&asset_b, &500_000_000_000_000_000);

    f.client.deposit_collateral(&user, &asset_a, &10);
    f.client.deposit_collateral(&user, &asset_b, &10);

    // 10 * 2.0 + 10 * 0.5 = 25, not the 1:1 sum of 20.
    assert_eq!(f.client.get_collateral_value(&user), 25);
}

#[test]
fn test_collateral_value_fails_when_oracle_unavailable() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(MarginingLiquidation, ());
    let client = MarginingLiquidationClient::new(&env, &contract_id);
    let missing_oracle = Address::generate(&env);
    client.initialize(
        &100_000_000_000_000_000,
        &50_000_000_000_000_000,
        &50_000_000_000_000_000,
        &missing_oracle,
    );

    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    client.deposit_collateral(&user, &asset, &100);

    let res = client.try_get_collateral_value(&user);
    assert_eq!(res, Err(Ok(Error::PriceUnavailable)));
}

#[test]
fn test_collateral_value_fails_when_price_missing() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);

    // No price was registered for this asset.
    f.client.deposit_collateral(&user, &asset, &100);

    let res = f.client.try_get_collateral_value(&user);
    assert_eq!(res, Err(Ok(Error::PriceUnavailable)));
}

#[test]
fn test_oracle_price_drives_margin_health() {
    let f = setup();
    let user = Address::generate(&f.env);
    let asset = Address::generate(&f.env);
    let quote = Address::generate(&f.env);
    f.set_price(&quote, &PRICE_ONE);

    f.client.deposit_collateral(&user, &quote, &200);
    f.client
        .open_position(&user, &asset, &quote, &100, &10, &true);
    assert!(f.client.check_margin(&user));

    // Collateral is worth 80 at a price of 0.4 against a 1000 notional, so the
    // margin ratio is 8% and still above the 5% maintenance threshold.
    f.set_price(&quote, &400_000_000_000_000_000);
    assert!(f.client.check_margin(&user));

    // At 0.04 the collateral is worth 8, so the margin ratio is 0.8% and the
    // account must be treated as unhealthy.
    f.set_price(&quote, &40_000_000_000_000_000);
    assert!(!f.client.check_margin(&user));
}
