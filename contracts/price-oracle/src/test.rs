
#![cfg(test)]
#![allow(non_snake_case)]

use soroban_sdk::{
    testutils::Env, Address, Symbol};
use super::{PriceOracle, PriceOracleClient, Error, OracleSourceType, DataKey};

#[test]
fn test_initialize_and_set_source() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle_address = env.register_contract(None, PriceOracle);
    let client = PriceOracleClient::new(&env, &oracle_address);

    // Initialize with 5% max deviation, 3600s freshness threshold, 86400s TWAP window
    client.initialize(&admin, &500, &3600, &86400);

    // Set primary source
    let primary_address = Address::generate(&env);
    client.set_oracle_source(&OracleSourceType::Primary, &primary_address);
    
    // Verify active source is primary
    let active_source: OracleSourceType = env.storage().instance().get(&DataKey::ActiveSource).unwrap();
    assert_eq!(active_source, OracleSourceType::Primary);
}

#[test]
fn test_update_and_get_price() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle_address = env.register_contract(None, PriceOracle);
    let client = PriceOracleClient::new(&env, &oracle_address);
    client.initialize(&admin, &500, &3600, &86400);

    let asset = Address::generate(&env);
    let primary = Address::generate(&env);
    client.set_oracle_source(&OracleSourceType::Primary, &primary);

    // Update price with admin as operator (temporarily - in production, admin is required for testing
    client.update_price(&asset, &1000_000000000000000); // $1000 with 18 decimals
    
    // Get price
    let price = client.get_price(&asset);
    assert_eq!(price, 1000_000000000000000);
}

#[test]
fn test_price_deviation_fallback() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle_address = env.register_contract(None, PriceOracle);
    let client = PriceOracleClient::new(&env, &oracle_address);
    client.initialize(&admin, &500, &3600, &86400); // 5% max deviation

    let asset = Address::generate(&env);
    let primary = Address::generate(&env);
    let secondary = Address::generate(&env);
    client.set_oracle_source(&OracleSourceType::Primary, &primary);
    client.set_oracle_source(&OracleSourceType::Secondary, &secondary);

    // Initial price update
    client.update_price(&asset, &1000_000000000000000);
    
    // Try to update with 10% deviation which should trigger fallback
    let result = std::panic::catch_unwind(|| {
        client.update_price(&asset, &1110_000000000000000); // 11% increase
    });
    
    // The update should fail
    assert!(result.is_err());
    
    // Verify we switched to secondary source
    let active_source: OracleSourceType = env.storage().instance().get(&DataKey::ActiveSource).unwrap();
    assert_eq!(active_source, OracleSourceType::Secondary);
}

#[test]
fn test_stale_price_triggers_fallback() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let oracle_address = env.register_contract(None, PriceOracle);
    let client = PriceOracleClient::new(&env, &oracle_address);
    client.initialize(&admin, &500, &3600, &86400); // 1 hour freshness

    let asset = Address::generate(&env);
    let primary = Address::generate(&env);
    let secondary = Address::generate(&env);
    client.set_oracle_source(&OracleSourceType::Primary, &primary);
    client.set_oracle_source(&OracleSourceType::Secondary, &secondary);

    // Update price at timestamp 0
    client.update_price(&asset, &1000_000000000000000);
    
    // Simulate time passing - 2 hours later
    env.ledger().with_mut(|ledger| {
        ledger.timestamp += 7200;
    });
    
    // Trying to get price should trigger fallback
    let result = std::panic::catch_unwind(|| {
        client.get_price(&asset);
    });
    
    assert!(result.is_err());
    
    // Verify we switched to secondary source
    let active_source: OracleSourceType = env.storage().instance().get(&DataKey::ActiveSource).unwrap();
    assert_eq!(active_source, OracleSourceType::Secondary);
}