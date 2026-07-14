#![no_std]

//! # Asset Registry
//!
//! Tracks tokenized assets that are eligible to trade within the
//! InterChangableTrade ecosystem. Each asset is registered by an admin and
//! maps to an on-chain Stellar asset (token contract) address.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

/// Storage keys for the registry.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin authorized to register/remove assets.
    Admin,
    /// Metadata for a registered asset, keyed by its token contract address.
    Asset(Address),
    /// The list of all registered asset addresses.
    Assets,
}

/// Metadata describing a registered asset.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asset {
    /// The token contract (Stellar asset) address.
    pub token: Address,
    /// Human-readable symbol, e.g. "USDC".
    pub symbol: Symbol,
    /// Whether the asset is currently tradeable.
    pub enabled: bool,
}

/// Errors surfaced by the registry.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    AssetExists = 3,
    AssetNotFound = 4,
}

const EVT_REGISTERED: Symbol = symbol_short!("registerd");
const EVT_REMOVED: Symbol = symbol_short!("removed");

#[contract]
pub struct AssetRegistry;

#[contractimpl]
impl AssetRegistry {
    /// Initialize the registry with an admin. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Assets, &Vec::<Address>::new(&env));
        Ok(())
    }

    /// Register a new tradeable asset. Requires admin authorization.
    pub fn register(env: Env, token: Address, symbol: Symbol) -> Result<(), Error> {
        let admin = Self::admin(&env)?;
        admin.require_auth();

        let key = DataKey::Asset(token.clone());
        if env.storage().persistent().has(&key) {
            return Err(Error::AssetExists);
        }

        let asset = Asset { token: token.clone(), symbol, enabled: true };
        env.storage().persistent().set(&key, &asset);

        let mut assets: Vec<Address> = env.storage().instance().get(&DataKey::Assets).unwrap_or(Vec::new(&env));
        assets.push_back(token.clone());
        env.storage().instance().set(&DataKey::Assets, &assets);

        env.events().publish((EVT_REGISTERED, token), asset);
        Ok(())
    }

    /// Remove an asset from the registry. Requires admin authorization.
    pub fn remove(env: Env, token: Address) -> Result<(), Error> {
        let admin = Self::admin(&env)?;
        admin.require_auth();

        let key = DataKey::Asset(token.clone());
        if !env.storage().persistent().has(&key) {
            return Err(Error::AssetNotFound);
        }
        env.storage().persistent().remove(&key);

        let assets: Vec<Address> = env.storage().instance().get(&DataKey::Assets).unwrap_or(Vec::new(&env));
        let mut remaining = Vec::new(&env);
        for a in assets.iter() {
            if a != token {
                remaining.push_back(a);
            }
        }
        env.storage().instance().set(&DataKey::Assets, &remaining);

        env.events().publish((EVT_REMOVED, token), ());
        Ok(())
    }

    /// Fetch a registered asset's metadata.
    pub fn get(env: Env, token: Address) -> Result<Asset, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Asset(token))
            .ok_or(Error::AssetNotFound)
    }

    /// List all registered asset addresses.
    pub fn list(env: Env) -> Vec<Address> {
        env.storage().instance().get(&DataKey::Assets).unwrap_or(Vec::new(&env))
    }

    fn admin(env: &Env) -> Result<Address, Error> {
        env.storage().instance().get(&DataKey::Admin).ok_or(Error::NotInitialized)
    }
}

#[cfg(test)]
mod test;
