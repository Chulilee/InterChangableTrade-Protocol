#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Role(Symbol, Address),
    RoleAdmin(Symbol),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    RoleNotFound = 4,
    RoleAlreadyExists = 5,
}

const EVT_ROLE_GRANTED: Symbol = symbol_short!("role_grntd");
const EVT_ROLE_REVOKED: Symbol = symbol_short!("role_rvkd");

const ROLE_ADMIN: Symbol = symbol_short!("ADMIN");
const ROLE_OPERATOR: Symbol = symbol_short!("OPERATOR");
const ROLE_PAUSER: Symbol = symbol_short!("PAUSER");
const ROLE_GOVERNOR: Symbol = symbol_short!("GOVERNOR");

#[contract]
pub struct AccessControl;

#[contractimpl]
impl AccessControl {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Self::grant_role_internal(&env, ROLE_ADMIN, admin.clone());
        Self::set_role_admin_internal(&env, ROLE_ADMIN, ROLE_ADMIN);
        Self::set_role_admin_internal(&env, ROLE_OPERATOR, ROLE_ADMIN);
        Self::set_role_admin_internal(&env, ROLE_PAUSER, ROLE_ADMIN);
        Self::set_role_admin_internal(&env, ROLE_GOVERNOR, ROLE_ADMIN);
        Ok(())
    }

    pub fn grant_role(env: Env, role: Symbol, account: Address) -> Result<(), Error> {
        let role_admin = Self::get_role_admin_internal(&env, role)?;
        Self::require_role(&env, role_admin)?;

        Self::grant_role_internal(&env, role, account);
        Ok(())
    }

    pub fn revoke_role(env: Env, role: Symbol, account: Address) -> Result<(), Error> {
        let role_admin = Self::get_role_admin_internal(&env, role)?;
        Self::require_role(&env, role_admin)?;

        Self::revoke_role_internal(&env, role, account);
        Ok(())
    }

    pub fn renounce_role(env: Env, role: Symbol, account: Address) -> Result<(), Error> {
        account.require_auth();
        Self::revoke_role_internal(&env, role, account);
        Ok(())
    }

    pub fn has_role(env: Env, role: Symbol, account: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Role(role, account))
    }

    pub fn get_role_admin(env: Env, role: Symbol) -> Result<Symbol, Error> {
        Self::get_role_admin_internal(&env, role)
    }

    pub fn set_role_admin(env: Env, role: Symbol, admin_role: Symbol) -> Result<(), Error> {
        Self::require_role(&env, ROLE_ADMIN)?;
        Self::set_role_admin_internal(&env, role, admin_role);
        Ok(())
    }

    fn admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    fn grant_role_internal(env: &Env, role: Symbol, account: Address) {
        let key = DataKey::Role(role, account.clone());
        if env.storage().persistent().has(&key) {
            return;
        }
        env.storage().persistent().set(&key, &());
        env.events()
            .publish((EVT_ROLE_GRANTED, role, account), ());
    }

    fn revoke_role_internal(env: &Env, role: Symbol, account: Address) {
        let key = DataKey::Role(role, account.clone());
        if !env.storage().persistent().has(&key) {
            return;
        }
        env.storage().persistent().remove(&key);
        env.events()
            .publish((EVT_ROLE_REVOKED, role, account), ());
    }

    fn require_role(env: &Env, role: Symbol) -> Result<(), Error> {
        let caller = env.invoker();
        if !Self::has_role(env.clone(), role, caller.clone()) {
            return Err(Error::Unauthorized);
        }
        caller.require_auth();
        Ok(())
    }

    fn get_role_admin_internal(env: &Env, role: Symbol) -> Result<Symbol, Error> {
        env.storage()
            .instance()
            .get(&DataKey::RoleAdmin(role))
            .ok_or(Error::RoleNotFound)
    }

    fn set_role_admin_internal(env: &Env, role: Symbol, admin_role: Symbol) {
        env.storage()
            .instance()
            .set(&DataKey::RoleAdmin(role), &admin_role);
    }
}

#[cfg(test)]
mod test;
