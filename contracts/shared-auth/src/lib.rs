#![no_std]

//! Shared access-control helpers (issue #61).
//!
//! Every privileged entry point in the InterChangableTrade-Protocol contracts
//! authorizes the caller through the `access-control` contract instead of a
//! locally stored admin address. This crate is the single implementation of
//! that check, so each contract does not grow its own drifting copy:
//!
//! ```ignore
//! let access_control: Address = env.storage().instance().get(&DataKey::AccessControl).unwrap();
//! shared_auth::require_admin(&env, &access_control, &caller)?;
//! ```
//!
//! `access-control` hosts the roles (`ADMIN`, `OPERATOR`, `PAUSER`, `GOVERNOR`)
//! and decides whether `account` holds one; this crate only asks it. It never
//! signs or submits transactions — it performs read-only `has_role` calls.

use soroban_sdk::{contracterror, symbol_short, Address, Env, IntoVal, Symbol};

/// Well-known role identifiers, shared with `access-control`.
pub const ROLE_ADMIN: Symbol = symbol_short!("ADMIN");
pub const ROLE_OPERATOR: Symbol = symbol_short!("OPERATOR");
pub const ROLE_PAUSER: Symbol = symbol_short!("PAUSER");
pub const ROLE_GOVERNOR: Symbol = symbol_short!("GOVERNOR");

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AuthError {
    /// The caller does not hold the required role.
    Unauthorized = 1,
}

/// Ask the `access-control` contract whether `account` holds `role`.
///
/// Performs a read-only invocation of `has_role`; it does not authenticate
/// anyone. Use [`require_role`] for a checking entry point; use this only
/// when you need the boolean itself.
pub fn has_role(env: &Env, access_control: &Address, role: &Symbol, account: &Address) -> bool {
    env.invoke_contract(
        access_control,
        &symbol_short!("has_role"),
        (role.clone(), account.clone()).into_val(env),
    )
}

/// Require `caller` to hold `role` in the `access-control` contract.
///
/// `caller.require_auth()` is always called; the role check runs afterwards.
pub fn require_role(
    env: &Env,
    access_control: &Address,
    caller: &Address,
    role: &Symbol,
) -> Result<(), AuthError> {
    caller.require_auth();
    if !has_role(env, access_control, role, caller) {
        return Err(AuthError::Unauthorized);
    }
    Ok(())
}

/// Require `caller` to hold the `ADMIN` role. The helper every contract can
/// use for admin-only operations.
pub fn require_admin(
    env: &Env,
    access_control: &Address,
    caller: &Address,
) -> Result<(), AuthError> {
    require_role(env, access_control, caller, &ROLE_ADMIN)
}

/// Require `caller` to hold the `OPERATOR` role.
pub fn require_operator(
    env: &Env,
    access_control: &Address,
    caller: &Address,
) -> Result<(), AuthError> {
    require_role(env, access_control, caller, &ROLE_OPERATOR)
}

/// Require `caller` to hold the `PAUSER` role.
pub fn require_pauser(
    env: &Env,
    access_control: &Address,
    caller: &Address,
) -> Result<(), AuthError> {
    require_role(env, access_control, caller, &ROLE_PAUSER)
}

/// Require `caller` to hold the `GOVERNOR` role.
pub fn require_governor(
    env: &Env,
    access_control: &Address,
    caller: &Address,
) -> Result<(), AuthError> {
    require_role(env, access_control, caller, &ROLE_GOVERNOR)
}
