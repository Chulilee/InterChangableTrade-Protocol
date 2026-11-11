#![no_std]

use shared_auth::require_pauser;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    MarketPaused,
    MaxOrderSize,
    /// Address of the access-control contract that holds the PAUSER role.
    AccessControl,
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
    NotInitialized = 5,
}

const EVT_MKT_PAUSED: Symbol = symbol_short!("paused");
const EVT_MKT_UNPAUS: Symbol = symbol_short!("unpaused");
const EVT_LIM_UPDATED: Symbol = symbol_short!("limit_set");

#[contract]
pub struct RiskManager;

#[contractimpl]
impl RiskManager {
    pub fn initialize(
        env: Env,
        admin: Address,
        access_control: Address,
        max_order_size: i128,
    ) -> Result<(), RiskError> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::AccessControl) {
            return Err(RiskError::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::AccessControl, &access_control);
        env.storage().instance().set(&DataKey::MarketPaused, &false);
        env.storage()
            .instance()
            .set(&DataKey::MaxOrderSize, &max_order_size);
        Ok(())
    }

    pub fn pause_market(env: Env, caller: Address) -> Result<(), RiskError> {
        let access_control = Self::access_control(&env)?;
        require_pauser(&env, &access_control, &caller).map_err(|_| RiskError::Unauthorized)?;
        env.storage().instance().set(&DataKey::MarketPaused, &true);
        env.events().publish((EVT_MKT_PAUSED,), caller);
        Ok(())
    }

    pub fn unpause_market(env: Env, caller: Address) -> Result<(), RiskError> {
        let access_control = Self::access_control(&env)?;
        require_pauser(&env, &access_control, &caller).map_err(|_| RiskError::Unauthorized)?;
        env.storage().instance().set(&DataKey::MarketPaused, &false);
        env.events().publish((EVT_MKT_UNPAUS,), caller);
        Ok(())
    }

    pub fn set_limit(env: Env, caller: Address, max_order_size: i128) -> Result<(), RiskError> {
        let access_control = Self::access_control(&env)?;
        require_pauser(&env, &access_control, &caller).map_err(|_| RiskError::Unauthorized)?;
        env.storage()
            .instance()
            .set(&DataKey::MaxOrderSize, &max_order_size);
        env.events()
            .publish((EVT_LIM_UPDATED,), (caller, max_order_size));
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
            .ok_or(RiskError::NotInitialized)?;
        if order_size > max_order_size {
            return Err(RiskError::MaxOrderSizeExceeded);
        }
        let exposure: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::CumulativeExposure(trader.clone()))
            .unwrap_or(0);
        let new_exposure = exposure
            .checked_add(order_size)
            .ok_or(RiskError::CumulativeExposureExceeded)?;
        // Clamp cumulative exposure to the configured max_order_size per trader;
        // this is the cap the `CumulativeExposureExceeded` error exists for.
        if new_exposure > max_order_size {
            return Err(RiskError::CumulativeExposureExceeded);
        }
        env.storage()
            .persistent()
            .set(&DataKey::CumulativeExposure(trader.clone()), &new_exposure);
        Ok(())
    }

    /// Get the current cumulative exposure for a trader.
    pub fn get_exposure(env: Env, trader: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::CumulativeExposure(trader))
            .unwrap_or(0)
    }

    fn access_control(env: &Env) -> Result<Address, RiskError> {
        env.storage()
            .instance()
            .get(&DataKey::AccessControl)
            .ok_or(RiskError::NotInitialized)
    }
}

#[cfg(test)]
mod test;
