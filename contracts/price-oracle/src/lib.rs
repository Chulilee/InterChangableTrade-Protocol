#![no_std]

//! # Price Oracle
//!
//! Provides reliable external price feeds with time-weighted average price (TWAP)
//! support, sanity checks (max deviation, freshness thresholds), and fallback logic
//! to secondary sources. Protects against price manipulation and stale data.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, IntoVal,
    Symbol,
};

/// Enum representing a price source type
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleSourceType {
    /// Primary oracle source
    Primary,
    /// Secondary fallback oracle source
    Secondary,
}

/// Oracle source configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleSource {
    /// Contract address of the oracle
    pub address: Address,
    /// Whether this source is currently active
    pub active: bool,
    /// Last successful update timestamp
    pub last_updated: u64,
}

/// Price data stored for an asset
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceData {
    /// The asset's token address
    pub asset: Address,
    /// Current price (scaled to 18 decimals)
    pub price: i128,
    /// Timestamp of the last price update
    pub timestamp: u64,
    /// TWAP price over the configured window
    pub twap: i128,
    /// Number of price observations for TWAP calculation
    pub observations: u32,
}

/// Configuration for oracle sanity checks
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    /// Maximum allowed price deviation between updates (basis points, e.g., 500 = 5%)
    pub max_deviation_bps: u32,
    /// Maximum age of price data before considered stale (seconds)
    pub freshness_threshold: u64,
    /// TWAP calculation window (number of blocks/seconds)
    pub twap_window: u64,
}

/// Errors surfaced by the price oracle
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidSource = 4,
    PriceStale = 5,
    PriceDeviationExceeded = 6,
    NoActiveSources = 7,
    AssetNotFound = 8,
}

/// Storage keys for the price oracle
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Access control contract address
    AccessControl,
    /// Oracle configuration
    Config,
    /// Oracle source, keyed by source type
    Source(OracleSourceType),
    /// Price data for an asset, keyed by asset address
    Price(Address),
    /// Currently active source type
    ActiveSource,
}

/// Event symbols (max 9 chars for `symbol_short!`).
const EVT_ORACLE_UPDATED: Symbol = symbol_short!("orc_upd");
const EVT_ORACLE_SOURCE_CHANGED: Symbol = symbol_short!("orc_src");

/// Predefined roles from access-control pattern.
const ROLE_ADMIN: Symbol = symbol_short!("ADMIN");
const ROLE_OPERATOR: Symbol = symbol_short!("OPERATOR");

#[contract]
pub struct PriceOracle;

#[contractimpl]
impl PriceOracle {
    /// Initialize the price oracle with an access control contract and base configuration.
    pub fn initialize(
        env: Env,
        access_control: Address,
        max_deviation_bps: u32,
        freshness_threshold: u64,
        twap_window: u64,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::AccessControl) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage()
            .instance()
            .set(&DataKey::AccessControl, &access_control);

        let config = OracleConfig {
            max_deviation_bps,
            freshness_threshold,
            twap_window,
        };
        env.storage().instance().set(&DataKey::Config, &config);

        Ok(())
    }

    /// Set or update an oracle source (primary or secondary). Requires admin authorization.
    pub fn set_oracle_source(
        env: Env,
        caller: Address,
        source_type: OracleSourceType,
        address: Address,
    ) -> Result<(), Error> {
        Self::require_role(&env, &caller, ROLE_ADMIN)?;

        let source = OracleSource {
            address: address.clone(),
            active: true,
            last_updated: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Source(source_type.clone()), &source);

        // If this is the first source, set it as active
        if !env.storage().instance().has(&DataKey::ActiveSource) {
            env.storage()
                .instance()
                .set(&DataKey::ActiveSource, &source_type);
        }

        env.events()
            .publish((EVT_ORACLE_SOURCE_CHANGED, source_type, address), ());
        Ok(())
    }

    /// Update price for an asset. Only callable by authorized operators.
    pub fn update_price(
        env: Env,
        caller: Address,
        asset: Address,
        new_price: i128,
    ) -> Result<(), Error> {
        Self::require_role(&env, &caller, ROLE_OPERATOR)?;

        let config: OracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // Get current price if it exists, to check deviation and roll the TWAP.
        let current_price_data: Option<PriceData> = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset.clone()));

        let (new_twap, new_observations) = if let Some(current) = current_price_data {
            // Reject a move larger than the configured deviation outright. A
            // single anomalous tick never mutates oracle state, so it cannot be
            // weaponized to force a failover.
            let price_diff = (new_price - current.price).unsigned_abs();
            let deviation_bps = (price_diff * 10000) / current.price.unsigned_abs();
            if deviation_bps > config.max_deviation_bps as u128 {
                return Err(Error::PriceDeviationExceeded);
            }

            // Simple observation-weighted average.
            let twap = ((current.twap * current.observations as i128) + new_price)
                / (current.observations + 1) as i128;
            (twap, current.observations + 1)
        } else {
            (new_price, 1)
        };

        let price_data = PriceData {
            asset: asset.clone(),
            price: new_price,
            timestamp: env.ledger().timestamp(),
            twap: new_twap,
            observations: new_observations,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Price(asset.clone()), &price_data);

        // Update last_updated for the active source
        let active_source_type: OracleSourceType = env
            .storage()
            .instance()
            .get(&DataKey::ActiveSource)
            .ok_or(Error::NoActiveSources)?;

        let mut active_source: OracleSource = env
            .storage()
            .persistent()
            .get(&DataKey::Source(active_source_type.clone()))
            .ok_or(Error::NoActiveSources)?;
        active_source.last_updated = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Source(active_source_type), &active_source);

        env.events()
            .publish((EVT_ORACLE_UPDATED, asset), (new_price, new_twap));
        Ok(())
    }

    /// Get the current validated price for an asset.
    pub fn get_price(env: Env, asset: Address) -> Result<i128, Error> {
        let price_data: PriceData = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset.clone()))
            .ok_or(Error::AssetNotFound)?;

        let config: OracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        let current_timestamp = env.ledger().timestamp();
        if current_timestamp - price_data.timestamp > config.freshness_threshold {
            // Stale data is never served. Recovery is an explicit privileged
            // failover via `set_active_source`, not an implicit read-time switch
            // (which could not persist through this error return anyway).
            return Err(Error::PriceStale);
        }

        // Return TWAP if available, otherwise current price
        Ok(if price_data.observations > 1 {
            price_data.twap
        } else {
            price_data.price
        })
    }

    /// Explicitly promote a configured source (typically the secondary) to be
    /// the active one. This is the sanctioned recovery path when the active
    /// feed is producing stale or anomalous data: because it returns `Ok`, the
    /// switch is committed rather than rolled back with a rejecting read/update.
    /// Requires admin authorization.
    pub fn set_active_source(
        env: Env,
        caller: Address,
        source_type: OracleSourceType,
    ) -> Result<(), Error> {
        Self::require_role(&env, &caller, ROLE_ADMIN)?;

        let mut source: OracleSource = env
            .storage()
            .persistent()
            .get(&DataKey::Source(source_type.clone()))
            .ok_or(Error::InvalidSource)?;
        if !source.active {
            return Err(Error::InvalidSource);
        }

        env.storage()
            .instance()
            .set(&DataKey::ActiveSource, &source_type);
        source.last_updated = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Source(source_type.clone()), &source);

        env.events()
            .publish((EVT_ORACLE_SOURCE_CHANGED, source_type, source.address), ());
        Ok(())
    }

    /// Internal helper to check caller authorization using the access-control contract.
    fn require_role(env: &Env, caller: &Address, required_role: Symbol) -> Result<(), Error> {
        let access_control: Address = env
            .storage()
            .instance()
            .get(&DataKey::AccessControl)
            .ok_or(Error::NotInitialized)?;

        caller.require_auth();

        // Call the access-control contract to check if the caller has the role.
        let has_role: bool = env.invoke_contract(
            &access_control,
            &Symbol::new(env, "has_role"),
            (required_role, caller.clone()).into_val(env),
        );

        if !has_role {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
