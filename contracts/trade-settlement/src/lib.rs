#![no_std]

//! # Trade Settlement & Clearing
//!
//! Handles atomic transfers of assets between parties after a trade is matched.
//! Supports individual atomic trade settlement, batched netted settlement,
//! failure recovery and retry semantics, and lifecycle event emission.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, token, Address, Env, Symbol, Vec,
};

pub mod netting;
pub use netting::NetObligation;

/// Storage keys for settlement.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Auto-incrementing id for the next trade.
    NextId,
    /// Auto-incrementing id for settlement batches.
    NextBatchId,
    /// A trade keyed by its id.
    Trade(u64),
}

/// The lifecycle phase and status of a settlement.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementStatus {
    Pending = 0,
    Initiated = 1,
    Settled = 2,
    Failed = 3,
    Cancelled = 4,
}

pub type Phase = SettlementStatus;

/// A trade between a buyer and seller with base and quote asset specifications.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub base_asset: Address,
    pub quote_asset: Address,
    pub base_amount: i128,
    pub quote_amount: i128,
    pub status: SettlementStatus,
    pub failure_reason: Option<Symbol>,
}

/// Result of a batched netted settlement execution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchSettlementResult {
    pub batch_id: u64,
    pub trade_ids: Vec<u64>,
    pub net_transfers_executed: u32,
    pub original_transfers_count: u32,
    pub status: SettlementStatus,
}

/// Errors surfaced by settlement.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    TradeNotFound = 1,
    InvalidAmount = 2,
    NotPending = 3,
    Unauthorized = 4,
    InsufficientBalance = 5,
    TransferFailed = 6,
    InvalidBatch = 7,
    SettlementAlreadyCompleted = 8,
    NotFailed = 9,
}

#[contract]
pub struct TradeSettlement;

#[contractimpl]
impl TradeSettlement {
    /// Open a pending trade between a buyer and seller. Requires buyer authorization.
    pub fn open(
        env: Env,
        buyer: Address,
        seller: Address,
        base_asset: Address,
        quote_asset: Address,
        base_amount: i128,
        quote_amount: i128,
    ) -> Result<u64, Error> {
        buyer.require_auth();
        if base_amount <= 0 || quote_amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);
        let trade = Trade {
            id,
            buyer,
            seller,
            base_asset,
            quote_asset,
            base_amount,
            quote_amount,
            status: SettlementStatus::Pending,
            failure_reason: None,
        };
        env.storage().persistent().set(&DataKey::Trade(id), &trade);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));

        env.events().publish(
            (Symbol::new(&env, "SettlementInitiated"), id),
            trade.clone(),
        );

        Ok(id)
    }

    /// Settle a pending trade atomically. Either counterparty or authorized agent may initiate.
    pub fn settle_trade(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        caller.require_auth();
        let mut trade = Self::get(env.clone(), id)?;
        if caller != trade.buyer && caller != trade.seller {
            return Err(Error::Unauthorized);
        }
        if trade.status != SettlementStatus::Pending && trade.status != SettlementStatus::Failed {
            return Err(Error::NotPending);
        }

        // Emit SettlementInitiated
        env.events().publish(
            (Symbol::new(&env, "SettlementInitiated"), id),
            trade.clone(),
        );

        // Pre-check balances using Soroban token interfaces
        let base_client = token::Client::new(&env, &trade.base_asset);
        let quote_client = token::Client::new(&env, &trade.quote_asset);

        let seller_base_bal = base_client.balance(&trade.seller);
        let buyer_quote_bal = quote_client.balance(&trade.buyer);

        if seller_base_bal < trade.base_amount || buyer_quote_bal < trade.quote_amount {
            let reason = Symbol::new(&env, "InsufficientBalance");
            trade.status = SettlementStatus::Failed;
            trade.failure_reason = Some(reason.clone());
            env.storage().persistent().set(&DataKey::Trade(id), &trade);

            env.events().publish(
                (Symbol::new(&env, "SettlementFailed"), id),
                (reason, trade.clone()),
            );

            return Ok(trade);
        }

        // Perform atomic bilateral transfers:
        // 1. Seller -> Buyer for base asset
        base_client.transfer(&trade.seller, &trade.buyer, &trade.base_amount);
        // 2. Buyer -> Seller for quote asset
        quote_client.transfer(&trade.buyer, &trade.seller, &trade.quote_amount);

        trade.status = SettlementStatus::Settled;
        trade.failure_reason = None;
        env.storage().persistent().set(&DataKey::Trade(id), &trade);

        env.events().publish(
            (Symbol::new(&env, "SettlementCompleted"), id),
            trade.clone(),
        );

        Ok(trade)
    }

    /// Alias for backwards compatibility / simplified invocation.
    pub fn settle(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        Self::settle_trade(env, id, caller)
    }

    /// Settle a batch of pending trades using netting to minimize on-chain transfers.
    pub fn settle_batch_netted(
        env: Env,
        trade_ids: Vec<u64>,
        caller: Address,
    ) -> Result<BatchSettlementResult, Error> {
        caller.require_auth();

        if trade_ids.is_empty() {
            return Err(Error::InvalidBatch);
        }

        let batch_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextBatchId)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::NextBatchId, &(batch_id + 1));

        let mut trades = Vec::new(&env);
        for id in trade_ids.iter() {
            let trade = Self::get(env.clone(), id)?;
            if trade.status != SettlementStatus::Pending && trade.status != SettlementStatus::Failed
            {
                return Err(Error::NotPending);
            }
            trades.push_back(trade);
        }

        env.events().publish(
            (Symbol::new(&env, "SettlementInitiated"), batch_id),
            (batch_id, trade_ids.clone()),
        );

        let (net_obligations, original_count) =
            netting::NettingEngine::compute_net_obligations(&env, &trades);

        // Verify balances for all net debtors
        for oblig in net_obligations.iter() {
            let token_client = token::Client::new(&env, &oblig.asset);
            let debtor_bal = token_client.balance(&oblig.debtor);
            if debtor_bal < oblig.amount {
                let reason = Symbol::new(&env, "InsufficientBalance");
                for mut trade in trades.iter() {
                    trade.status = SettlementStatus::Failed;
                    trade.failure_reason = Some(reason.clone());
                    env.storage()
                        .persistent()
                        .set(&DataKey::Trade(trade.id), &trade);
                }

                env.events().publish(
                    (Symbol::new(&env, "SettlementFailed"), batch_id),
                    (reason, batch_id, trade_ids.clone()),
                );

                return Err(Error::InsufficientBalance);
            }
        }

        // Execute net transfer obligations atomically
        for oblig in net_obligations.iter() {
            let token_client = token::Client::new(&env, &oblig.asset);
            token_client.transfer(&oblig.debtor, &oblig.creditor, &oblig.amount);
        }

        // Update trade statuses to Settled
        for mut trade in trades.iter() {
            trade.status = SettlementStatus::Settled;
            trade.failure_reason = None;
            env.storage()
                .persistent()
                .set(&DataKey::Trade(trade.id), &trade);
        }

        let result = BatchSettlementResult {
            batch_id,
            trade_ids: trade_ids.clone(),
            net_transfers_executed: net_obligations.len(),
            original_transfers_count: original_count,
            status: SettlementStatus::Settled,
        };

        env.events().publish(
            (Symbol::new(&env, "SettlementCompleted"), batch_id),
            result.clone(),
        );

        Ok(result)
    }

    /// Retry settlement for a previously failed trade.
    pub fn retry_settlement(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        let trade = Self::get(env.clone(), id)?;
        if trade.status != SettlementStatus::Failed {
            return Err(Error::NotFailed);
        }
        Self::settle_trade(env, id, caller)
    }

    /// Preview netting results without executing transfers.
    pub fn preview_netting(
        env: Env,
        trade_ids: Vec<u64>,
    ) -> Result<(Vec<NetObligation>, u32), Error> {
        if trade_ids.is_empty() {
            return Err(Error::InvalidBatch);
        }
        let mut trades = Vec::new(&env);
        for id in trade_ids.iter() {
            let trade = Self::get(env.clone(), id)?;
            trades.push_back(trade);
        }
        Ok(netting::NettingEngine::compute_net_obligations(
            &env, &trades,
        ))
    }

    /// Cancel a pending trade. Either counterparty may cancel.
    pub fn cancel(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        caller.require_auth();
        let mut trade = Self::get(env.clone(), id)?;
        if caller != trade.buyer && caller != trade.seller {
            return Err(Error::Unauthorized);
        }
        if trade.status != SettlementStatus::Pending && trade.status != SettlementStatus::Failed {
            return Err(Error::NotPending);
        }
        trade.status = SettlementStatus::Cancelled;
        env.storage().persistent().set(&DataKey::Trade(id), &trade);

        env.events().publish(
            (Symbol::new(&env, "SettlementCancelled"), id),
            trade.clone(),
        );

        Ok(trade)
    }

    /// Fetch a trade by id.
    pub fn get(env: Env, id: u64) -> Result<Trade, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Trade(id))
            .ok_or(Error::TradeNotFound)
    }
}

#[cfg(test)]
mod test;
