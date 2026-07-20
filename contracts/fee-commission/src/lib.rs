#![no_std]

//! # Fee & Commission System
//!
//! Calculates, collects, and distributes protocol and maker/taker trading fees.
//!
//! Fees are expressed in **basis points** (bps), where `10_000 bps = 100%`.
//! A trade of `notional` units incurs a fee of `notional * fee_bps / 10_000`.
//!
//! * `maker_fee_bps` / `taker_fee_bps` — charged to the maker/taker side of a trade.
//! * `protocol_fee_bps` — the share of every *collected* fee routed to the protocol
//!   treasury; the remainder is split among [`FeeRecipient`]s by their `share_bps`.
//!
//! Collected fees accumulate in a per-asset pool inside this contract and are paid
//! out to the treasury and recipients when [`FeeCommission::distribute_fee`] runs.
//!
//! Fee parameters are governed by an admin (governance) address and support:
//! * exemption / whitelist addresses that trade fee-free, and
//! * scheduled parameter changes that activate at a future ledger timestamp.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol,
    Vec,
};

#[cfg(test)]
mod test;

/// Basis-point denominator: `10_000 bps == 100%`.
pub const BPS_DENOMINATOR: i128 = 10_000;

/// Upper bound on a single side's trading fee (10%). Guards against fat-finger
/// governance updates.
pub const MAX_FEE_BPS: i128 = 1_000;

/// Storage keys for the fee module.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin (governance) address authorized to change fee parameters.
    Admin,
    /// Active fee configuration.
    Config,
    /// Ordered list of fee recipients (excluding the protocol treasury).
    Recipients,
    /// Exemption / whitelist flag for an address (fee-free when `true`).
    Exempt(Address),
    /// Accumulated, not-yet-distributed fee balance for an asset.
    FeePool(Address),
    /// A scheduled configuration change awaiting its activation timestamp.
    Pending,
}

/// Which side of a trade a party is on.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Side {
    /// Liquidity maker — charged `maker_fee_bps`.
    Maker,
    /// Liquidity taker — charged `taker_fee_bps`.
    Taker,
}

/// Configurable fee parameters.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeConfig {
    /// Fee charged to the maker side, in basis points.
    pub maker_fee_bps: i128,
    /// Fee charged to the taker side, in basis points.
    pub taker_fee_bps: i128,
    /// Share of every collected fee routed to the protocol treasury, in basis points.
    pub protocol_fee_bps: i128,
    /// Treasury address that receives the protocol's share of fees.
    pub protocol_treasury: Address,
}

/// A recipient of the non-protocol portion of distributed fees.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeRecipient {
    /// Address that receives the fee share.
    pub address: Address,
    /// Share of the post-protocol remainder, in basis points. Shares of all
    /// recipients must sum to exactly [`BPS_DENOMINATOR`].
    pub share_bps: i128,
}

/// A fee-parameter change scheduled to activate at a future ledger time.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingConfig {
    /// The configuration that becomes active at `activate_at`.
    pub config: FeeConfig,
    /// Ledger timestamp (seconds) at or after which the change may be applied.
    pub activate_at: u64,
}

/// Errors surfaced by the fee module.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// A fee or share parameter is outside its permitted range.
    InvalidFee = 3,
    /// A supplied amount was zero or negative.
    InvalidAmount = 4,
    /// Recipient `share_bps` values do not sum to `BPS_DENOMINATOR`.
    InvalidShares = 5,
    /// The asset's fee pool is empty; nothing to distribute.
    NothingToDistribute = 6,
    /// No scheduled configuration change is pending.
    NoPendingConfig = 7,
    /// The scheduled change's activation time has not yet been reached.
    NotYetActive = 8,
    /// Fee recipients have not been configured.
    NoRecipients = 9,
}

const EVT_FEE_COLLECTED: Symbol = symbol_short!("feecollct");
const EVT_FEE_DISTRIB: Symbol = symbol_short!("feedistr");
const EVT_CONFIG_SET: Symbol = symbol_short!("cfgset");
const EVT_SCHEDULED: Symbol = symbol_short!("cfgsched");
const EVT_EXEMPT_SET: Symbol = symbol_short!("exemptset");

#[contract]
pub struct FeeCommission;

#[contractimpl]
impl FeeCommission {
    /// Initialize the module with an admin and the starting fee configuration.
    /// Callable once.
    pub fn initialize(
        env: Env,
        admin: Address,
        maker_fee_bps: i128,
        taker_fee_bps: i128,
        protocol_fee_bps: i128,
        protocol_treasury: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(Error::AlreadyInitialized);
        }
        let config = FeeConfig {
            maker_fee_bps,
            taker_fee_bps,
            protocol_fee_bps,
            protocol_treasury,
        };
        Self::validate_config(&config)?;

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::Recipients, &Vec::<FeeRecipient>::new(&env));
        Ok(())
    }

    /// Calculate the fee owed for a trade of `amount` on the given `side`.
    ///
    /// Returns `0` if `payer` is exempt/whitelisted. This is a read-only view
    /// intended for off-chain quoting and on-chain preflight checks.
    pub fn calculate_fees(
        env: Env,
        payer: Address,
        amount: i128,
        side: Side,
    ) -> Result<i128, Error> {
        let config = Self::config(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if Self::is_exempt(env.clone(), payer) {
            return Ok(0);
        }
        let fee_bps = match side {
            Side::Maker => config.maker_fee_bps,
            Side::Taker => config.taker_fee_bps,
        };
        Ok(amount * fee_bps / BPS_DENOMINATOR)
    }

    /// Collect the trading fee for `amount` from `payer` on the given `side`.
    ///
    /// Transfers the computed fee in `asset` tokens from `payer` into the
    /// contract's per-asset fee pool and returns the fee charged. Exempt payers
    /// are charged nothing and no transfer occurs.
    pub fn collect_fee(
        env: Env,
        payer: Address,
        asset: Address,
        amount: i128,
        side: Side,
    ) -> Result<i128, Error> {
        payer.require_auth();
        let fee = Self::calculate_fees(env.clone(), payer.clone(), amount, side)?;
        if fee == 0 {
            return Ok(0);
        }

        // Pull the fee from the payer into this contract.
        let contract = env.current_contract_address();
        token::Client::new(&env, &asset).transfer(&payer, &contract, &fee);

        // Credit the per-asset pool.
        let pool_key = DataKey::FeePool(asset.clone());
        let pooled: i128 = env.storage().persistent().get(&pool_key).unwrap_or(0);
        env.storage().persistent().set(&pool_key, &(pooled + fee));

        env.events()
            .publish((EVT_FEE_COLLECTED, payer, asset), fee);
        Ok(fee)
    }

    /// Distribute the accumulated fee pool for `asset` to the protocol treasury
    /// and configured recipients, then zero the pool.
    ///
    /// The treasury receives `protocol_fee_bps` of the pool plus any rounding
    /// dust; each recipient receives its `share_bps` of the remainder.
    /// Permissionless: funds only ever move to pre-configured addresses.
    pub fn distribute_fee(env: Env, asset: Address) -> Result<i128, Error> {
        let config = Self::config(&env)?;
        let recipients: Vec<FeeRecipient> = env
            .storage()
            .instance()
            .get(&DataKey::Recipients)
            .unwrap_or(Vec::new(&env));
        if recipients.is_empty() {
            return Err(Error::NoRecipients);
        }

        let pool_key = DataKey::FeePool(asset.clone());
        let total: i128 = env.storage().persistent().get(&pool_key).unwrap_or(0);
        if total <= 0 {
            return Err(Error::NothingToDistribute);
        }

        let client = token::Client::new(&env, &asset);
        let contract = env.current_contract_address();

        // Protocol treasury cut.
        let protocol_amount = total * config.protocol_fee_bps / BPS_DENOMINATOR;
        let remainder = total - protocol_amount;

        // Split the remainder among recipients by share; track what we paid out
        // so any integer-division dust can be swept to the treasury.
        let mut distributed_to_recipients: i128 = 0;
        for r in recipients.iter() {
            let amount = remainder * r.share_bps / BPS_DENOMINATOR;
            if amount > 0 {
                client.transfer(&contract, &r.address, &amount);
                distributed_to_recipients += amount;
            }
        }

        // Treasury receives its cut plus any rounding dust.
        let treasury_amount = protocol_amount + (remainder - distributed_to_recipients);
        if treasury_amount > 0 {
            client.transfer(&contract, &config.protocol_treasury, &treasury_amount);
        }

        env.storage().persistent().set(&pool_key, &0i128);
        env.events().publish((EVT_FEE_DISTRIB, asset), total);
        Ok(total)
    }

    /// Update the active fee parameters immediately. Requires admin authorization.
    pub fn set_fees(
        env: Env,
        maker_fee_bps: i128,
        taker_fee_bps: i128,
        protocol_fee_bps: i128,
        protocol_treasury: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        let config = FeeConfig {
            maker_fee_bps,
            taker_fee_bps,
            protocol_fee_bps,
            protocol_treasury,
        };
        Self::validate_config(&config)?;
        env.storage().instance().set(&DataKey::Config, &config);
        env.events().publish((EVT_CONFIG_SET,), config);
        Ok(())
    }

    /// Set the list of fee recipients. Shares must sum to [`BPS_DENOMINATOR`].
    /// Requires admin authorization.
    pub fn set_recipients(env: Env, recipients: Vec<FeeRecipient>) -> Result<(), Error> {
        Self::require_admin(&env)?;
        Self::validate_shares(&recipients)?;
        env.storage()
            .instance()
            .set(&DataKey::Recipients, &recipients);
        Ok(())
    }

    /// Add or remove an address from the fee-exemption whitelist.
    /// Requires admin authorization.
    pub fn set_exempt(env: Env, address: Address, exempt: bool) -> Result<(), Error> {
        Self::require_admin(&env)?;
        let key = DataKey::Exempt(address.clone());
        if exempt {
            env.storage().persistent().set(&key, &true);
        } else {
            env.storage().persistent().remove(&key);
        }
        env.events().publish((EVT_EXEMPT_SET, address), exempt);
        Ok(())
    }

    /// Schedule a fee-parameter change to take effect at `activate_at` (a ledger
    /// timestamp in seconds). Requires admin authorization. Overwrites any
    /// previously scheduled change.
    pub fn schedule_fee_update(
        env: Env,
        maker_fee_bps: i128,
        taker_fee_bps: i128,
        protocol_fee_bps: i128,
        protocol_treasury: Address,
        activate_at: u64,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        let config = FeeConfig {
            maker_fee_bps,
            taker_fee_bps,
            protocol_fee_bps,
            protocol_treasury,
        };
        Self::validate_config(&config)?;
        let pending = PendingConfig {
            config,
            activate_at,
        };
        env.storage().instance().set(&DataKey::Pending, &pending);
        env.events()
            .publish((EVT_SCHEDULED,), (pending.config, activate_at));
        Ok(())
    }

    /// Apply a scheduled fee change once its activation time has been reached.
    /// Permissionless: the change and its timing were fixed by governance.
    pub fn apply_scheduled_update(env: Env) -> Result<(), Error> {
        let pending: PendingConfig = env
            .storage()
            .instance()
            .get(&DataKey::Pending)
            .ok_or(Error::NoPendingConfig)?;
        if env.ledger().timestamp() < pending.activate_at {
            return Err(Error::NotYetActive);
        }
        env.storage()
            .instance()
            .set(&DataKey::Config, &pending.config);
        env.storage().instance().remove(&DataKey::Pending);
        env.events().publish((EVT_CONFIG_SET,), pending.config);
        Ok(())
    }

    // ---- Views ------------------------------------------------------------

    /// Whether `address` is exempt/whitelisted from fees.
    pub fn is_exempt(env: Env, address: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Exempt(address))
            .unwrap_or(false)
    }

    /// Read the active fee configuration.
    pub fn get_config(env: Env) -> Result<FeeConfig, Error> {
        Self::config(&env)
    }

    /// Read the configured fee recipients.
    pub fn get_recipients(env: Env) -> Vec<FeeRecipient> {
        env.storage()
            .instance()
            .get(&DataKey::Recipients)
            .unwrap_or(Vec::new(&env))
    }

    /// Read the not-yet-distributed fee balance for `asset`.
    pub fn get_fee_pool(env: Env, asset: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::FeePool(asset))
            .unwrap_or(0)
    }

    /// Read the pending scheduled configuration, if any.
    pub fn get_pending(env: Env) -> Option<PendingConfig> {
        env.storage().instance().get(&DataKey::Pending)
    }

    // ---- Internal helpers -------------------------------------------------

    fn config(env: &Env) -> Result<FeeConfig, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)
    }

    fn require_admin(env: &Env) -> Result<Address, Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        Ok(admin)
    }

    fn validate_config(config: &FeeConfig) -> Result<(), Error> {
        if config.maker_fee_bps < 0
            || config.taker_fee_bps < 0
            || config.maker_fee_bps > MAX_FEE_BPS
            || config.taker_fee_bps > MAX_FEE_BPS
        {
            return Err(Error::InvalidFee);
        }
        if config.protocol_fee_bps < 0 || config.protocol_fee_bps > BPS_DENOMINATOR {
            return Err(Error::InvalidFee);
        }
        Ok(())
    }

    fn validate_shares(recipients: &Vec<FeeRecipient>) -> Result<(), Error> {
        // An empty recipient set is permitted (all fees accrue to the treasury
        // via distribution dust), but non-empty sets must sum to 100%.
        if recipients.is_empty() {
            return Ok(());
        }
        let mut sum: i128 = 0;
        for r in recipients.iter() {
            if r.share_bps < 0 {
                return Err(Error::InvalidShares);
            }
            sum += r.share_bps;
        }
        if sum != BPS_DENOMINATOR {
            return Err(Error::InvalidShares);
        }
        Ok(())
    }
}
