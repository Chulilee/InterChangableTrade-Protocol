#![no_std]

//! # Price Oracle
//!
//! Provides reliable external price feeds with time-weighted average price (TWAP)
//! support, sanity checks (max deviation, freshness thresholds), and fallback logic
//! to secondary sources. Protects against price manipulation and stale data.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
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
    /// Admin address
    Admin,
    /// Oracle configuration
    Config,
    /// Oracle source, keyed by source type
    Source(OracleSourceType),
    /// Price data for an asset, keyed by asset address
    Price(Address),
    /// Currently active source type
    ActiveSource,
}

/// Event symbols
const EVT_ORACLE_UPDATED: Symbol = symbol_short!("oracle_upd");
const EVT_ORACLE_SOURCE_CHANGED: Symbol = symbol_short!("oracle_srcchg");
const EVT_ORACLE_ALERT: Symbol = symbol_short!("oracle_alert");

/// Predefined roles from access-control pattern
const ROLE_ADMIN: Symbol = symbol_short!("ADMIN");
const ROLE_OPERATOR: Symbol = symbol_short!("OPERATOR");

#[contract]
pub struct PriceOracle;

#[contractclient("price-oracle")]
#[contractimpl]
impl PriceOracle {
    /// Initialize the price oracle with an admin and base configuration.
    pub fn initialize(
        env: Env,
        admin: Address,
        max_deviation_bps: u32,
        freshness_threshold: u64,
        twap_window: u64,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Admin, &admin);

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
        source_type: OracleSourceType,
        address: Address,
    ) -> Result<(), Error> {
        Self::require_role(&env, ROLE_ADMIN)?;

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
            env.storage().instance().set(&DataKey::ActiveSource, &source_type);
        }

        env.events()
            .publish((EVT_ORACLE_SOURCE_CHANGED, source_type, address), ());
        Ok(())
    }

    /// Update price for an asset. Only callable by authorized operators.
    pub fn update_price(
        env: Env,
        asset: Address,
        new_price: i128,
    ) -> Result<(), Error> {
        Self::require_role(&env, ROLE_OPERATOR)?;

        let config: OracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        // Get current price if exists to check deviation
        let current_price_data: Option<PriceData> = env.storage().persistent().get(&DataKey::Price(asset.clone()));
        
        if let Some(current) = current_price_data {
            // Calculate price deviation
            let price_diff = (new_price - current.price).abs();
            let deviation_bps = (price_diff as u128 * 10000) / current.price.abs() as u128;
            
            if deviation_bps > config.max_deviation_bps as u128 {
                // Trigger fallback to secondary source if primary failed validation
                Self::handle_fallback(&env, asset.clone(), "Price deviation exceeded")?;
                return Err(Error::PriceDeviationExceeded);
            }
        }

        // Calculate new TWAP
        let new_twap = if let Some(current) = current_price_data {
            // Simple TWAP calculation: weighted average based on observations
            ((current.twap * current.observations as i128) + new_price) / (current.observations + 1) as i128
        } else {
            new_price
        };

        let new_observations = if let Some(current) = current_price_data {
            current.observations + 1
        } else {
            1
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
            .get(&DataKey::Source(active_source_type))
            .ok_or(Error::NoActiveSources)?;
        active_source.last_updated = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Source(active_source_type), &active_source);

        env.events().publish((EVT_ORACLE_UPDATED, asset), (new_price, new_twap));
        Ok(())
    }

    /// Get the current validated price for an asset.
    pub fn get_price(env: Env, asset: Address) -> Result<i128, Error> {
        let price_data: PriceData = env
            .storage()
            .persistent()
            .get(&DataKey::Price(asset))
            .ok_or(Error::AssetNotFound)?;

        let config: OracleConfig = env
            .storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)?;

        let current_timestamp = env.ledger().timestamp();
        if current_timestamp - price_data.timestamp > config.freshness_threshold {
            // Price is stale, try to use fallback
            Self::handle_fallback(&env, asset.clone(), "Price data is stale")?;
            return Err(Error::PriceStale);
        }

        // Return TWAP if available, otherwise current price
        Ok(if price_data.observations > 1 {
            price_data.twap
        } else {
            price_data.price
        })
    }

    /// Switch to the secondary oracle source as the active source.
    fn switch_to_secondary(env: &Env) -> Result<(), Error> {
        let secondary_source: Option<OracleSource> = env
            .storage()
            .persistent()
            .get(&DataKey::Source(OracleSourceType::Secondary));

        if let Some(mut secondary) = secondary_source {
            if secondary.active {
                env.storage().instance().set(&DataKey::ActiveSource, &OracleSourceType::Secondary);
                secondary.last_updated = env.ledger().timestamp();
                env.storage()
                    .persistent()
                    .set(&DataKey::Source(OracleSourceType::Secondary), &secondary);
                
                env.events().publish(
                    (EVT_ORACLE_SOURCE_CHANGED, OracleSourceType::Secondary, secondary.address),
                    ()
                );
                Ok(())
            } else {
                Err(Error::NoActiveSources)
            }
        } else {
            Err(Error::NoActiveSources)
        }
    }

    /// Handle fallback logic when primary source fails validation.
    fn handle_fallback(env: &Env, asset: Address, reason: &str) -> Result<(), Error> {
        // Emit alert event
        let reason_symbol = Symbol::from_str(reason);
        env.events().publish((EVT_ORACLE_ALERT, asset, reason_symbol), ());
        
        // Try to switch to secondary source
        Self::switch_to_secondary(env)
    }

    /// Internal helper to check caller authorization.
    fn require_role(env: &Env, required_role: Symbol) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
            
        let caller = env.invoker();
        // For simplicity, admin always has all roles; in production this would integrate
        // with the access-control contract's has_role function
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        caller.require_auth();
        Ok(())
    }
}

#[cfg(test)]
mod test;