#![no_std]

//! # Liquidity Incentives (LP Rewards)
//!
//! Rewards liquidity providers for supplying depth. Rewards accrue over time,
//! are proportional to the liquidity a position contributes, and are claimable
//! at any point.
//!
//! ## Reward model
//!
//! Accrual uses the standard *reward-per-liquidity accumulator* (the pattern
//! popularised by Synthetix `StakingRewards`). Each pool emits `reward_rate`
//! reward tokens per second, shared pro-rata across all active liquidity:
//!
//! ```text
//! reward_per_token += dt * reward_rate * PRECISION / total_liquidity
//! earned(position)  = position.liquidity
//!                   * (reward_per_token - position.reward_per_token_paid)
//!                   / PRECISION
//!                   + position.rewards
//! ```
//!
//! The accumulator is advanced on *every* state change (deposit, withdraw,
//! claim, rate change), so rewards are time-weighted to the second and rapid
//! deposit/withdraw sequences prorate exactly: a position only ever earns over
//! the interval its liquidity was actually staked.
//!
//! ## Assets
//!
//! Each pool has a `staking_token` (the LP/depth token providers deposit) and a
//! `reward_token` (what they earn). Deposits pull `staking_token` into the
//! contract; withdrawals return it; claims pay out `reward_token`. Reward
//! tokens must be funded into the contract (via [`LiquidityIncentives::fund_pool`])
//! before they can be claimed.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol,
};

#[cfg(test)]
mod test;

/// Fixed-point scalar for the reward-per-liquidity accumulator (`1e18`).
pub const PRECISION: i128 = 1_000_000_000_000_000_000;

/// Storage keys for the liquidity incentives module.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin (governance) address authorized to manage pools.
    Admin,
    /// Auto-incrementing id for the next pool.
    NextPoolId,
    /// Auto-incrementing id for the next position.
    NextPositionId,
    /// A reward pool keyed by its id.
    Pool(u64),
    /// An LP position keyed by its id.
    Position(u64),
}

/// A reward pool: a `staking_token` whose depositors earn `reward_token`
/// emitted at `reward_rate` per second until `period_finish`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pool {
    /// Unique pool identifier.
    pub id: u64,
    /// Token that liquidity providers deposit (the LP / depth token).
    pub staking_token: Address,
    /// Token paid out as rewards.
    pub reward_token: Address,
    /// Reward tokens emitted per second while the period is active.
    pub reward_rate: i128,
    /// Ledger timestamp (seconds) at which reward emission stops.
    pub period_finish: u64,
    /// Accumulated reward per unit of liquidity, scaled by [`PRECISION`].
    pub reward_per_token_stored: i128,
    /// Last ledger timestamp at which the accumulator was advanced.
    pub last_update_time: u64,
    /// Total active liquidity currently staked in the pool.
    pub total_liquidity: i128,
}

/// A liquidity position owned by a provider within a pool.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    /// Unique position identifier.
    pub id: u64,
    /// Pool this position belongs to.
    pub pool_id: u64,
    /// Owner of the position.
    pub owner: Address,
    /// Liquidity currently contributed by this position.
    pub liquidity: i128,
    /// Lower tick of the price range this liquidity backs (metadata for
    /// range-based tracking; does not affect accrual).
    pub tick_lower: i32,
    /// Upper tick of the price range this liquidity backs.
    pub tick_upper: i32,
    /// Accumulator value already accounted for this position.
    pub reward_per_token_paid: i128,
    /// Settled-but-unclaimed rewards.
    pub rewards: i128,
}

/// Errors surfaced by the liquidity incentives module.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    PoolNotFound = 3,
    PositionNotFound = 4,
    /// A supplied amount was zero or negative.
    InvalidAmount = 5,
    /// Withdrawal amount exceeds the position's liquidity.
    InsufficientLiquidity = 6,
    /// Caller does not own the position.
    Unauthorized = 7,
    /// A reward-rate or duration parameter was invalid.
    InvalidParameters = 8,
    /// Nothing has accrued to claim.
    NothingToClaim = 9,
}

const EVT_LIQ_DEPOSITED: Symbol = symbol_short!("liqdep");
const EVT_LIQ_WITHDRAWN: Symbol = symbol_short!("liqwth");
const EVT_REWARDS_CLAIMED: Symbol = symbol_short!("rwclaim");
const EVT_POOL_CREATED: Symbol = symbol_short!("poolnew");
const EVT_POOL_FUNDED: Symbol = symbol_short!("poolfund");
const EVT_RATE_SET: Symbol = symbol_short!("rateset");

#[contract]
pub struct LiquidityIncentives;

#[contractimpl]
impl LiquidityIncentives {
    /// Initialize the module with an admin (governance) address. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextPoolId, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &0u64);
        Ok(())
    }

    /// Create a reward pool. Requires admin authorization.
    ///
    /// Emits `reward_rate` reward tokens per second for `duration_secs` seconds
    /// starting now. Reward tokens must be funded separately via [`Self::fund_pool`].
    pub fn create_pool(
        env: Env,
        staking_token: Address,
        reward_token: Address,
        reward_rate: i128,
        duration_secs: u64,
    ) -> Result<u64, Error> {
        Self::require_admin(&env)?;
        if reward_rate < 0 {
            return Err(Error::InvalidParameters);
        }

        let now = env.ledger().timestamp();
        let id: u64 = env.storage().instance().get(&DataKey::NextPoolId).unwrap();
        let pool = Pool {
            id,
            staking_token,
            reward_token,
            reward_rate,
            period_finish: now.saturating_add(duration_secs),
            reward_per_token_stored: 0,
            last_update_time: now,
            total_liquidity: 0,
        };
        env.storage().persistent().set(&DataKey::Pool(id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::NextPoolId, &(id + 1));

        env.events().publish((EVT_POOL_CREATED, id), pool);
        Ok(id)
    }

    /// Fund a pool's reward reserve by transferring `amount` reward tokens from
    /// the admin into the contract. Requires admin authorization.
    pub fn fund_pool(env: Env, pool_id: u64, amount: i128) -> Result<(), Error> {
        let admin = Self::require_admin(&env)?;
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let pool = Self::load_pool(&env, pool_id)?;
        token::Client::new(&env, &pool.reward_token).transfer(
            &admin,
            &env.current_contract_address(),
            &amount,
        );
        env.events().publish((EVT_POOL_FUNDED, pool_id), amount);
        Ok(())
    }

    /// Update a pool's reward rate and extend its emission period. Requires
    /// admin authorization. Settles accrual up to now before changing the rate
    /// so past emissions are unaffected.
    pub fn set_reward_rate(
        env: Env,
        pool_id: u64,
        reward_rate: i128,
        duration_secs: u64,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        if reward_rate < 0 {
            return Err(Error::InvalidParameters);
        }
        let mut pool = Self::load_pool(&env, pool_id)?;
        Self::sync_pool(&env, &mut pool);
        pool.reward_rate = reward_rate;
        pool.period_finish = env.ledger().timestamp().saturating_add(duration_secs);
        env.storage()
            .persistent()
            .set(&DataKey::Pool(pool_id), &pool);
        env.events()
            .publish((EVT_RATE_SET, pool_id), (reward_rate, pool.period_finish));
        Ok(())
    }

    /// Deposit liquidity into a pool, opening a new position over the given tick
    /// range. Transfers `amount` staking tokens from `owner` into the contract
    /// and returns the new position id.
    pub fn deposit_liquidity(
        env: Env,
        owner: Address,
        pool_id: u64,
        amount: i128,
        tick_lower: i32,
        tick_upper: i32,
    ) -> Result<u64, Error> {
        owner.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if tick_lower >= tick_upper {
            return Err(Error::InvalidParameters);
        }
        let mut pool = Self::load_pool(&env, pool_id)?;

        // Advance the accumulator before changing total liquidity.
        Self::sync_pool(&env, &mut pool);

        // Pull staking tokens into the contract.
        token::Client::new(&env, &pool.staking_token).transfer(
            &owner,
            &env.current_contract_address(),
            &amount,
        );

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextPositionId)
            .unwrap();
        let position = Position {
            id,
            pool_id,
            owner: owner.clone(),
            liquidity: amount,
            tick_lower,
            tick_upper,
            reward_per_token_paid: pool.reward_per_token_stored,
            rewards: 0,
        };
        pool.total_liquidity += amount;

        env.storage()
            .persistent()
            .set(&DataKey::Position(id), &position);
        env.storage()
            .persistent()
            .set(&DataKey::Pool(pool_id), &pool);
        env.storage()
            .instance()
            .set(&DataKey::NextPositionId, &(id + 1));

        env.events()
            .publish((EVT_LIQ_DEPOSITED, owner, pool_id), (id, amount));
        Ok(id)
    }

    /// Withdraw liquidity from a position. Settles accrued rewards into the
    /// position (still claimable) and returns `amount` staking tokens to the
    /// owner. A full withdrawal leaves the position open with zero liquidity so
    /// any unclaimed rewards can still be claimed.
    pub fn withdraw_liquidity(
        env: Env,
        owner: Address,
        position_id: u64,
        amount: i128,
    ) -> Result<(), Error> {
        owner.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let mut position = Self::load_position(&env, position_id)?;
        if position.owner != owner {
            return Err(Error::Unauthorized);
        }
        if amount > position.liquidity {
            return Err(Error::InsufficientLiquidity);
        }
        let mut pool = Self::load_pool(&env, position.pool_id)?;

        // Settle accrual for this position before reducing its liquidity.
        Self::sync_pool(&env, &mut pool);
        Self::settle_position(&mut position, &pool);

        position.liquidity -= amount;
        pool.total_liquidity -= amount;

        // Return staking tokens to the owner.
        token::Client::new(&env, &pool.staking_token).transfer(
            &env.current_contract_address(),
            &owner,
            &amount,
        );

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);
        env.storage()
            .persistent()
            .set(&DataKey::Pool(position.pool_id), &pool);

        env.events()
            .publish((EVT_LIQ_WITHDRAWN, owner, position_id), amount);
        Ok(())
    }

    /// Claim all accrued rewards for a position. Transfers the reward tokens to
    /// the owner and resets the position's accrued counter to zero.
    pub fn claim_rewards(env: Env, owner: Address, position_id: u64) -> Result<i128, Error> {
        owner.require_auth();
        let mut position = Self::load_position(&env, position_id)?;
        if position.owner != owner {
            return Err(Error::Unauthorized);
        }
        let mut pool = Self::load_pool(&env, position.pool_id)?;

        Self::sync_pool(&env, &mut pool);
        Self::settle_position(&mut position, &pool);

        let amount = position.rewards;
        if amount <= 0 {
            return Err(Error::NothingToClaim);
        }
        position.rewards = 0;

        token::Client::new(&env, &pool.reward_token).transfer(
            &env.current_contract_address(),
            &owner,
            &amount,
        );

        env.storage()
            .persistent()
            .set(&DataKey::Position(position_id), &position);
        env.storage()
            .persistent()
            .set(&DataKey::Pool(position.pool_id), &pool);

        env.events()
            .publish((EVT_REWARDS_CLAIMED, owner, position_id), amount);
        Ok(amount)
    }

    // ---- Views ------------------------------------------------------------

    /// View the rewards accrued to a position as of the current ledger time,
    /// without mutating state.
    pub fn view_accrued_rewards(env: Env, position_id: u64) -> Result<i128, Error> {
        let position = Self::load_position(&env, position_id)?;
        let pool = Self::load_pool(&env, position.pool_id)?;
        let rpt = Self::reward_per_token(&pool, env.ledger().timestamp());
        Ok(Self::earned(&position, rpt))
    }

    /// Fetch a pool by id.
    pub fn get_pool(env: Env, pool_id: u64) -> Result<Pool, Error> {
        Self::load_pool(&env, pool_id)
    }

    /// Fetch a position by id.
    pub fn get_position(env: Env, position_id: u64) -> Result<Position, Error> {
        Self::load_position(&env, position_id)
    }

    // ---- Internal helpers -------------------------------------------------

    fn require_admin(env: &Env) -> Result<Address, Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        Ok(admin)
    }

    fn load_pool(env: &Env, pool_id: u64) -> Result<Pool, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Pool(pool_id))
            .ok_or(Error::PoolNotFound)
    }

    fn load_position(env: &Env, position_id: u64) -> Result<Position, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Position(position_id))
            .ok_or(Error::PositionNotFound)
    }

    /// The last ledger time at which rewards were still being emitted.
    fn last_applicable_time(pool: &Pool, now: u64) -> u64 {
        if now < pool.period_finish {
            now
        } else {
            pool.period_finish
        }
    }

    /// Reward-per-liquidity accumulator value at `now` (does not mutate).
    fn reward_per_token(pool: &Pool, now: u64) -> i128 {
        if pool.total_liquidity == 0 {
            return pool.reward_per_token_stored;
        }
        let applicable = Self::last_applicable_time(pool, now);
        if applicable <= pool.last_update_time {
            return pool.reward_per_token_stored;
        }
        let dt = (applicable - pool.last_update_time) as i128;
        pool.reward_per_token_stored + dt * pool.reward_rate * PRECISION / pool.total_liquidity
    }

    /// Rewards earned by a position at accumulator value `rpt`.
    fn earned(position: &Position, rpt: i128) -> i128 {
        position.rewards
            + position.liquidity * (rpt - position.reward_per_token_paid) / PRECISION
    }

    /// Advance a pool's stored accumulator to the current ledger time.
    fn sync_pool(env: &Env, pool: &mut Pool) {
        let now = env.ledger().timestamp();
        pool.reward_per_token_stored = Self::reward_per_token(pool, now);
        pool.last_update_time = Self::last_applicable_time(pool, now);
    }

    /// Settle a position's accrued rewards against the pool's *already-synced*
    /// accumulator. Call [`Self::sync_pool`] first.
    fn settle_position(position: &mut Position, pool: &Pool) {
        position.rewards = Self::earned(position, pool.reward_per_token_stored);
        position.reward_per_token_paid = pool.reward_per_token_stored;
    }
}
