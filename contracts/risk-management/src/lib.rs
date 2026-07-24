#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    MarketPaused,
    MaxOrderSize,
    Pauser,
    CumulativeExposure(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RiskError {
    MarketPaused = 1,
    MaxOrderSizeExceeded = 2,
    CumulativeExposureExceeded = 3,
    Unauthorized = 4,
}

const EVT_MKT_PAUSED: Symbol = symbol_short!("mkt_paus");
const EVT_MKT_UNPAUS: Symbol = symbol_short!("mkt_unp");
const EVT_LIM_UPDATED: Symbol = symbol_short!("lim_upd");

#[contract]
pub struct RiskManager;

#[contractimpl]
impl RiskManager {
    pub fn initialize(env: Env, pauser: Address, max_order_size: i128) -> Result<(), RiskError> {
        if env.storage().instance().has(&DataKey::Pauser) {
            return Err(RiskError::Unauthorized);
        }
        pauser.require_auth();
        env.storage().instance().set(&DataKey::Pauser, &pauser);
        env.storage().instance().set(&DataKey::MarketPaused, &false);
        env.storage()
            .instance()
            .set(&DataKey::MaxOrderSize, &max_order_size);
        Ok(())
    }

    pub fn pause_market(env: Env, caller: Address) -> Result<(), RiskError> {
        caller.require_auth();
        let pauser: Address = env.storage().instance().get(&DataKey::Pauser).unwrap();
        if caller != pauser {
            return Err(RiskError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::MarketPaused, &true);
        env.events().publish((EVT_MKT_PAUSED,), pauser);
        Ok(())
    }

    pub fn unpause_market(env: Env, caller: Address) -> Result<(), RiskError> {
        caller.require_auth();
        let pauser: Address = env.storage().instance().get(&DataKey::Pauser).unwrap();
        if caller != pauser {
            return Err(RiskError::Unauthorized);
        }
        env.storage().instance().set(&DataKey::MarketPaused, &false);
        env.events().publish((EVT_MKT_UNPAUS,), pauser);
        Ok(())
    }

    pub fn set_limit(env: Env, caller: Address, max_order_size: i128) -> Result<(), RiskError> {
        caller.require_auth();
        let pauser: Address = env.storage().instance().get(&DataKey::Pauser).unwrap();
        if caller != pauser {
            return Err(RiskError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::MaxOrderSize, &max_order_size);
        env.events()
            .publish((EVT_LIM_UPDATED,), (pauser, max_order_size));
        Ok(())
    }

    pub fn check_limits(env: Env, order_size: i128, trader: Address) -> Result<(), RiskError> {
        let market_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::MarketPaused)
            .unwrap_or(false);
        if market_paused {
            return Err(RiskError::MarketPaused);
        }
        let max_order_size: i128 = env
            .storage()
            .instance()
            .get(&DataKey::MaxOrderSize)
            .unwrap();
        if order_size > max_order_size {
            return Err(RiskError::MaxOrderSizeExceeded);
        }
        let exposure: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::CumulativeExposure(trader.clone()))
            .unwrap_or(0);
        if exposure.checked_add(order_size).is_none() {
            return Err(RiskError::CumulativeExposureExceeded);
        }
        Ok(())
    }
}
