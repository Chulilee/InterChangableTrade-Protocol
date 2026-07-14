#![no_std]

//! # Marketplace
//!
//! Lets sellers create listings offering a quantity of an asset at a fixed
//! price, and buyers accept them. Listings are the entry point of a trade;
//! settlement of funds/assets is handled by the escrow and settlement
//! contracts.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol,
};

/// Storage keys for the marketplace.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin authorized to administer the marketplace.
    Admin,
    /// Auto-incrementing id for the next listing.
    NextId,
    /// A listing keyed by its id.
    Listing(u64),
}

/// The lifecycle status of a listing.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Open,
    Filled,
    Cancelled,
}

/// A fixed-price offer to sell `amount` of `asset` for `price` of `quote`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Listing {
    pub id: u64,
    pub seller: Address,
    /// Asset being sold (token contract address).
    pub asset: Address,
    /// Quote token the price is denominated in.
    pub quote: Address,
    /// Amount of `asset` on offer.
    pub amount: i128,
    /// Total price in `quote` for the whole amount.
    pub price: i128,
    pub status: Status,
}

/// Errors surfaced by the marketplace.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    ListingNotFound = 3,
    ListingNotOpen = 4,
    InvalidAmount = 5,
    NotSeller = 6,
}

const EVT_LISTED: Symbol = symbol_short!("listed");
const EVT_FILLED: Symbol = symbol_short!("filled");
const EVT_CANCEL: Symbol = symbol_short!("cancelled");

#[contract]
pub struct Marketplace;

#[contractimpl]
impl Marketplace {
    /// Initialize the marketplace with an admin. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextId, &0u64);
        Ok(())
    }

    /// Create a new listing. Requires the seller's authorization.
    pub fn create_listing(
        env: Env,
        seller: Address,
        asset: Address,
        quote: Address,
        amount: i128,
        price: i128,
    ) -> Result<u64, Error> {
        Self::ensure_init(&env)?;
        seller.require_auth();
        if amount <= 0 || price <= 0 {
            return Err(Error::InvalidAmount);
        }

        let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(0);
        let listing = Listing {
            id,
            seller,
            asset,
            quote,
            amount,
            price,
            status: Status::Open,
        };
        env.storage().persistent().set(&DataKey::Listing(id), &listing);
        env.storage().instance().set(&DataKey::NextId, &(id + 1));

        env.events().publish((EVT_LISTED, id), listing);
        Ok(id)
    }

    /// Accept an open listing as a buyer. Marks it filled and returns it so a
    /// settlement/escrow flow can move the funds.
    pub fn fill_listing(env: Env, id: u64, buyer: Address) -> Result<Listing, Error> {
        buyer.require_auth();
        let mut listing = Self::get(env.clone(), id)?;
        if listing.status != Status::Open {
            return Err(Error::ListingNotOpen);
        }
        listing.status = Status::Filled;
        env.storage().persistent().set(&DataKey::Listing(id), &listing);

        env.events().publish((EVT_FILLED, id), buyer);
        Ok(listing)
    }

    /// Cancel an open listing. Only the original seller may cancel.
    pub fn cancel_listing(env: Env, id: u64, seller: Address) -> Result<(), Error> {
        seller.require_auth();
        let mut listing = Self::get(env.clone(), id)?;
        if listing.seller != seller {
            return Err(Error::NotSeller);
        }
        if listing.status != Status::Open {
            return Err(Error::ListingNotOpen);
        }
        listing.status = Status::Cancelled;
        env.storage().persistent().set(&DataKey::Listing(id), &listing);

        env.events().publish((EVT_CANCEL, id), ());
        Ok(())
    }

    /// Fetch a listing by id.
    pub fn get(env: Env, id: u64) -> Result<Listing, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Listing(id))
            .ok_or(Error::ListingNotFound)
    }

    fn ensure_init(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }
}

#[cfg(test)]
mod test;
