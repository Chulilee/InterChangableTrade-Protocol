#![no_std]

//! # Margining & Liquidation
//!
//! Implements margin accounts, collateral tracking, maintenance margins, and
//! automated liquidation for leveraged positions to support margin trading features.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Storage keys for margining and liquidation.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Auto-incrementing id for the next position.
    NextPositionId,
    /// A margin account keyed by user address.
    MarginAccount(Address),
    /// A position keyed by its id.
    Position(u64),
    /// Protocol configuration (initial margin, maintenance margin, liquidation incentive).
    Config,
}

/// Margin account structure tracking collateral balances and user positions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarginAccount {
    /// User address that owns this account.
    pub owner: Address,
    /// Mapping of asset addresses to collateral balances.
    pub collateral_balances: soroban_sdk::Map<Address, i128>,
    /// List of position ids associated with this account.
    pub positions: soroban_sdk::Vec<u64>,
}

/// Position structure representing a leveraged trading position.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    /// Unique position identifier.
    pub id: u64,
    /// Owner of the position.
    pub owner: Address,
    /// Asset being traded (base asset).
    pub asset: Address,
    /// Quote asset used for margin calculations.
    pub quote: Address,
    /// Size of the position in base asset.
    pub size: i128,
    /// Average entry price in quote asset per base asset.
    pub entry_price: i128,
    /// Current mark price used for margin calculations.
    pub mark_price: i128,
    /// Whether the position is long (true) or short (false).
    pub is_long: bool,
    /// Whether the position is currently active.
    pub is_active: bool,
}

/// Protocol configuration parameters.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// Initial margin requirement (e.g., 0.1 = 10% for 10x leverage).
    pub initial_margin_ratio: i128,
    /// Maintenance margin requirement (e.g., 0.05 = 5%).
    pub maintenance_margin_ratio: i128,
    /// Liquidation incentive percentage (e.g., 0.05 = 5% reward for liquidators).
    pub liquidation_incentive: i128,
    /// Address of the price oracle used to fetch mark prices.
    pub price_oracle: Address,
}

/// Errors surfaced by the margining and liquidation contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    MarginAccountNotFound = 3,
    PositionNotFound = 4,
    InsufficientCollateral = 5,
    InsufficientMargin = 6,
    PositionNotActive = 7,
    Unauthorized = 8,
    InvalidAmount = 9,
    PositionStillHealthy = 10,
}

/// Event symbols.
const EVT_COLLATERAL_DEPOSITED: Symbol = symbol_short!("colldep");
const EVT_COLLATERAL_WITHDRAWN: Symbol = symbol_short!("collwth");
const EVT_POSITION_OPENED: Symbol = symbol_short!("posopen");
const EVT_POSITION_CLOSED: Symbol = symbol_short!("posclsd");
const EVT_LIQUIDATION_TRIGGERED: Symbol = symbol_short!("liq");

#[contract]
pub struct MarginingLiquidation;

#[contractimpl]
impl MarginingLiquidation {
    /// Initialize the contract with protocol configuration. Callable once.
    pub fn initialize(
        env: Env,
        initial_margin_ratio: i128,
        maintenance_margin_ratio: i128,
        liquidation_incentive: i128,
        price_oracle: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Config) {
            return Err(Error::AlreadyInitialized);
        }
        if initial_margin_ratio <= 0
            || maintenance_margin_ratio <= 0
            || liquidation_incentive < 0
            || maintenance_margin_ratio >= initial_margin_ratio
        {
            return Err(Error::InvalidAmount);
        }

        let config = Config {
            initial_margin_ratio,
            maintenance_margin_ratio,
            liquidation_incentive,
            price_oracle,
        };
        env.storage().instance().set(&DataKey::Config, &config);
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &0u64);
        Ok(())
    }

    /// Deposit collateral into a user's margin account.
    pub fn deposit_collateral(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), Error> {
        user.require_auth();
        Self::ensure_init(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        // Get or create margin account
        let mut account = env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(user.clone()))
            .unwrap_or_else(|| MarginAccount {
                owner: user.clone(),
                collateral_balances: soroban_sdk::Map::new(&env),
                positions: soroban_sdk::Vec::new(&env),
            });

        // Update collateral balance
        let current_balance = account.collateral_balances.get(asset.clone()).unwrap_or(0);
        account
            .collateral_balances
            .set(asset.clone(), current_balance + amount);

        // Save updated account
        env.storage()
            .persistent()
            .set(&DataKey::MarginAccount(user.clone()), &account);

        // Emit event
        env.events()
            .publish((EVT_COLLATERAL_DEPOSITED, user, asset), amount);
        Ok(())
    }

    /// Withdraw collateral from a user's margin account.
    pub fn withdraw_collateral(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
    ) -> Result<(), Error> {
        user.require_auth();
        Self::ensure_init(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        let mut account = env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(user.clone()))
            .ok_or(Error::MarginAccountNotFound)?;

        // Check sufficient balance
        let current_balance = account.collateral_balances.get(asset.clone()).unwrap_or(0);
        if current_balance < amount {
            return Err(Error::InsufficientCollateral);
        }

        // Calculate margin requirements after withdrawal to ensure account remains healthy
        let new_balance = current_balance - amount;
        account.collateral_balances.set(asset.clone(), new_balance);

        // Check if account maintains sufficient margin after withdrawal
        if !Self::check_margin(env.clone(), user.clone()) {
            // Revert the balance change
            account.collateral_balances.set(asset, current_balance);
            env.storage()
                .persistent()
                .set(&DataKey::MarginAccount(user), &account);
            return Err(Error::InsufficientMargin);
        }

        // Save updated account
        env.storage()
            .persistent()
            .set(&DataKey::MarginAccount(user.clone()), &account);

        // Emit event
        env.events()
            .publish((EVT_COLLATERAL_WITHDRAWN, user, asset), amount);
        Ok(())
    }

    /// Open a new leveraged position.
    pub fn open_position(
        env: Env,
        owner: Address,
        asset: Address,
        quote: Address,
        size: i128,
        entry_price: i128,
        is_long: bool,
    ) -> Result<u64, Error> {
        owner.require_auth();
        let config = Self::ensure_init(&env)?;
        if size <= 0 || entry_price <= 0 {
            return Err(Error::InvalidAmount);
        }

        let mut account = env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(owner.clone()))
            .ok_or(Error::MarginAccountNotFound)?;

        // Calculate notional value and required initial margin
        let notional = size * entry_price;
        let required_margin = notional * config.initial_margin_ratio / 1_000_000_000_000_000_000; // Scale for fixed-point

        // Check if account has sufficient collateral to cover margin requirement
        let total_collateral_value = Self::calculate_total_collateral_value(env.clone(), &account);
        if total_collateral_value < required_margin {
            return Err(Error::InsufficientMargin);
        }

        // Create new position
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextPositionId)
            .unwrap();
        let position = Position {
            id,
            owner: owner.clone(),
            asset,
            quote,
            size,
            entry_price,
            mark_price: entry_price, // Initialize mark price to entry price
            is_long,
            is_active: true,
        };

        // Save position and update account
        env.storage()
            .persistent()
            .set(&DataKey::Position(id), &position);
        account.positions.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::MarginAccount(owner.clone()), &account);
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &(id + 1));

        // Emit event
        env.events()
            .publish((EVT_POSITION_OPENED, owner), position.clone());
        Ok(id)
    }

    /// Close an existing position.
    pub fn close_position(env: Env, owner: Address, position_id: u64) -> Result<(), Error> {
        owner.require_auth();
        Self::ensure_init(&env)?;

        let mut position = env
            .storage()
            .persistent()
            .get::<DataKey, Position>(&DataKey::Position(position_id))
            .ok_or(Error::PositionNotFound)?;

        if position.owner != owner {
            return Err(Error::Unauthorized);
        }
        if !position.is_active {
            return Err(Error::PositionNotActive);
        }

        // Mark position as inactive
        position.is_active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);

        // Remove position from account's position list
        let mut account = env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(owner.clone()))
            .ok_or(Error::MarginAccountNotFound)?;

        let mut new_positions = soroban_sdk::Vec::new(&env);
        for id in account.positions.iter() {
            if id != position_id {
                new_positions.push_back(id);
            }
        }
        account.positions = new_positions;
        env.storage()
            .persistent()
            .set(&DataKey::MarginAccount(owner.clone()), &account);

        // Emit event
        env.events()
            .publish((EVT_POSITION_CLOSED, owner, position_id), ());
        Ok(())
    }

    /// Check if an account maintains sufficient maintenance margin.
    pub fn check_margin(env: Env, user: Address) -> bool {
        let config = match env
            .storage()
            .instance()
            .get::<DataKey, Config>(&DataKey::Config)
        {
            Some(c) => c,
            None => return false,
        };

        let account = match env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(user))
        {
            Some(a) => a,
            None => return false,
        };

        // Calculate total collateral value and total position liabilities
        let total_collateral = Self::calculate_total_collateral_value(env.clone(), &account);
        let (total_position_value, total_unrealized_pnl) =
            Self::calculate_total_position_metrics(env.clone(), &account);

        // Account equity = collateral + unrealized P&L
        let account_equity = if total_unrealized_pnl >= 0 {
            total_collateral + total_unrealized_pnl
        } else {
            total_collateral - total_unrealized_pnl.abs()
        };

        // If no positions, account is always healthy
        if account.positions.is_empty() {
            return true;
        }

        // Calculate current margin ratio: equity / notional position value
        let margin_ratio = account_equity * 1_000_000_000_000_000_000 / total_position_value;

        // Return true if margin ratio is above maintenance requirement
        margin_ratio >= config.maintenance_margin_ratio
    }

    /// Trigger liquidation of an undercollateralized account.
    pub fn trigger_liquidation(
        env: Env,
        user: Address,
        liquidator: Address,
    ) -> Result<i128, Error> {
        liquidator.require_auth();
        let config = Self::ensure_init(&env)?;

        // First verify the account is indeed undercollateralized
        if Self::check_margin(env.clone(), user.clone()) {
            return Err(Error::PositionStillHealthy);
        }

        let account = env
            .storage()
            .persistent()
            .get::<DataKey, MarginAccount>(&DataKey::MarginAccount(user.clone()))
            .ok_or(Error::MarginAccountNotFound)?;

        // Calculate liquidator incentive
        let total_collateral_value = Self::calculate_total_collateral_value(env.clone(), &account);
        let incentive =
            total_collateral_value * config.liquidation_incentive / 1_000_000_000_000_000_000;

        // Close all active positions for the account
        for position_id in account.positions.iter() {
            if let Some(mut position) = env
                .storage()
                .persistent()
                .get::<DataKey, Position>(&DataKey::Position(position_id))
            {
                if position.is_active {
                    position.is_active = false;
                    env.storage()
                        .persistent()
                        .set(&DataKey::Position(position_id), &position);
                }
            }
        }

        // Reset the account's collateral balances (all collateral transferred to liquidator)
        let mut updated_account = account.clone();
        for (asset, _) in updated_account.collateral_balances.clone() {
            updated_account.collateral_balances.set(asset, 0);
        }
        updated_account.positions = soroban_sdk::Vec::new(&env);
        env.storage()
            .persistent()
            .set(&DataKey::MarginAccount(user.clone()), &updated_account);

        // Emit liquidation event
        env.events()
            .publish((EVT_LIQUIDATION_TRIGGERED, user, liquidator), incentive);

        Ok(incentive)
    }

    /// Fetch a margin account by user address.
    pub fn get_margin_account(env: Env, user: Address) -> Result<MarginAccount, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::MarginAccount(user))
            .ok_or(Error::MarginAccountNotFound)
    }

    /// Fetch a position by id.
    pub fn get_position(env: Env, id: u64) -> Result<Position, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Position(id))
            .ok_or(Error::PositionNotFound)
    }

    /// Ensure the contract is initialized.
    fn ensure_init(env: &Env) -> Result<Config, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Config)
            .ok_or(Error::NotInitialized)
    }

    /// Calculate total USD value of all collateral in an account.
    fn calculate_total_collateral_value(_env: Env, account: &MarginAccount) -> i128 {
        let mut total = 0i128;
        for (_asset, amount) in account.collateral_balances.clone() {
            // In a real implementation, this would fetch the price from the oracle
            // For this implementation, we'll assume all assets are quoted in the same unit
            // and simply sum the amounts (would be replaced with actual price lookup)
            total += amount;
        }
        total
    }

    /// Calculate total notional value and unrealized P&L of all active positions.
    fn calculate_total_position_metrics(env: Env, account: &MarginAccount) -> (i128, i128) {
        let mut total_notional = 0i128;
        let mut total_pnl = 0i128;

        for position_id in account.positions.iter() {
            if let Some(position) = env
                .storage()
                .persistent()
                .get::<DataKey, Position>(&DataKey::Position(position_id))
            {
                if position.is_active {
                    let notional = position.size * position.mark_price;
                    total_notional += notional;

                    // Calculate unrealized P&L
                    let price_diff = if position.is_long {
                        position.mark_price - position.entry_price
                    } else {
                        position.entry_price - position.mark_price
                    };
                    let pnl = position.size * price_diff;
                    total_pnl += pnl;
                }
            }
        }

        (total_notional, total_pnl)
    }
}

#[cfg(test)]
mod test;
