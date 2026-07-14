#![no_std]

//! # Trade Settlement
//!
//! Records the lifecycle of a trade agreed between a buyer and seller and
//! drives it to a terminal state (settled or cancelled). It is the
//! coordination point that the marketplace and escrow contracts feed into.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Storage keys for settlement.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Auto-incrementing id for the next trade.
    NextId,
    /// A trade keyed by its id.
    Trade(u64),
}

/// The lifecycle phase of a trade.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Pending,
    Settled,
    Cancelled,
}

/// A trade between a buyer and seller for a given asset and price.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trade {
    pub id: u64,
    pub buyer: Address,
    pub seller: Address,
    /// Asset being transferred to the buyer.
    pub asset: Address,
    /// Amount of `asset`.
    pub amount: i128,
    /// Total price paid by the buyer.
    pub price: i128,
    pub phase: Phase,
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
}

const EVT_OPENED: Symbol = symbol_short!("opened");
const EVT_SETTLED: Symbol = symbol_short!("settled");
const EVT_CANCEL: Symbol = symbol_short!("cancelled");

#[contract]
pub struct TradeSettlement;

#[contractimpl]
impl TradeSettlement {
    /// Open a pending trade. Requires the buyer's authorization.
    pub fn open(
        env: Env,
        buyer: Address,
        seller: Address,
        asset: Address,
        amount: i128,
        price: i128,
    ) -> Result<u64, Error> {
        buyer.require_auth();
        if amount <= 0 || price <= 0 {
            return Err(Error::InvalidAmount);
        }
        let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);
        let trade = Trade {
            id,
            buyer,
            seller,
            asset,
            amount,
            price,
            phase: Phase::Pending,
        };
        env.storage().persistent().set(&DataKey::Trade(id), &trade);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));
        env.events().publish((EVT_OPENED, id), trade);
        Ok(id)
    }

    /// Settle a pending trade. Either counterparty may settle once both sides
    /// of the exchange have been satisfied.
    pub fn settle(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        caller.require_auth();
        let mut trade = Self::get(env.clone(), id)?;
        if caller != trade.buyer && caller != trade.seller {
            return Err(Error::Unauthorized);
        }
        if trade.phase != Phase::Pending {
            return Err(Error::NotPending);
        }
        trade.phase = Phase::Settled;
        env.storage().persistent().set(&DataKey::Trade(id), &trade);
        env.events().publish((EVT_SETTLED, id), trade.clone());
        Ok(trade)
    }

    /// Cancel a pending trade. Either counterparty may cancel.
    pub fn cancel(env: Env, id: u64, caller: Address) -> Result<Trade, Error> {
        caller.require_auth();
        let mut trade = Self::get(env.clone(), id)?;
        if caller != trade.buyer && caller != trade.seller {
            return Err(Error::Unauthorized);
        }
        if trade.phase != Phase::Pending {
            return Err(Error::NotPending);
        }
        trade.phase = Phase::Cancelled;
        env.storage().persistent().set(&DataKey::Trade(id), &trade);
        env.events().publish((EVT_CANCEL, id), trade.clone());
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
