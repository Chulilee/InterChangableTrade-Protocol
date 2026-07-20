#![no_std]

//! # Escrow
//!
//! Holds a buyer's deposit for a trade until it is either released to the
//! seller (on successful settlement) or refunded to the buyer. This scaffold
//! models the escrow state machine; wiring to a concrete token contract is
//! left to the settlement layer.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Storage keys for the escrow.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Escrow record keyed by trade id.
    Escrow(u64),
}

/// The state of an escrowed deposit.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Funded,
    Released,
    Refunded,
}

/// A single escrow holding funds for one trade.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Escrow {
    pub trade_id: u64,
    pub buyer: Address,
    pub seller: Address,
    /// Token contract the amount is denominated in.
    pub token: Address,
    pub amount: i128,
    pub state: State,
}

/// Errors surfaced by the escrow.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    EscrowExists = 1,
    EscrowNotFound = 2,
    InvalidAmount = 3,
    NotFunded = 4,
}

const EVT_FUNDED: Symbol = symbol_short!("funded");
const EVT_RELEASED: Symbol = symbol_short!("released");
const EVT_REFUNDED: Symbol = symbol_short!("refunded");

#[contract]
pub struct EscrowContract;

#[contractimpl]
impl EscrowContract {
    /// Open and fund an escrow for a trade. Requires the buyer's auth.
    pub fn fund(
        env: Env,
        trade_id: u64,
        buyer: Address,
        seller: Address,
        token: Address,
        amount: i128,
    ) -> Result<(), Error> {
        buyer.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        let key = DataKey::Escrow(trade_id);
        if env.storage().persistent().has(&key) {
            return Err(Error::EscrowExists);
        }
        let escrow = Escrow {
            trade_id,
            buyer,
            seller,
            token,
            amount,
            state: State::Funded,
        };
        env.storage().persistent().set(&key, &escrow);
        env.events().publish((EVT_FUNDED, trade_id), escrow);
        Ok(())
    }

    /// Release the escrowed funds to the seller. Requires the buyer's auth
    /// (the buyer confirms receipt of the asset).
    pub fn release(env: Env, trade_id: u64) -> Result<Escrow, Error> {
        let mut escrow = Self::get(env.clone(), trade_id)?;
        escrow.buyer.require_auth();
        if escrow.state != State::Funded {
            return Err(Error::NotFunded);
        }
        escrow.state = State::Released;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(trade_id), &escrow);
        env.events()
            .publish((EVT_RELEASED, trade_id), escrow.clone());
        Ok(escrow)
    }

    /// Refund the escrowed funds to the buyer. Requires the seller's auth
    /// (the seller agrees to unwind the trade).
    pub fn refund(env: Env, trade_id: u64) -> Result<Escrow, Error> {
        let mut escrow = Self::get(env.clone(), trade_id)?;
        escrow.seller.require_auth();
        if escrow.state != State::Funded {
            return Err(Error::NotFunded);
        }
        escrow.state = State::Refunded;
        env.storage()
            .persistent()
            .set(&DataKey::Escrow(trade_id), &escrow);
        env.events()
            .publish((EVT_REFUNDED, trade_id), escrow.clone());
        Ok(escrow)
    }

    /// Fetch an escrow by trade id.
    pub fn get(env: Env, trade_id: u64) -> Result<Escrow, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Escrow(trade_id))
            .ok_or(Error::EscrowNotFound)
    }
}

#[cfg(test)]
mod test;
