#![no_std]

//! # Trade Matching & Suggestions
//!
//! Matches compatible listings by tags, categories, and value ranges, then
//! surfaces ranked suggestions so users can propose or accept trades.
//!
//! ## Matching algorithm
//!
//! For every pair of active listings `(a, b)` the contract computes a relevance
//! score out of 100 across three weighted dimensions:
//!
//! | Dimension          | Weight | How it's scored                                        |
//! |--------------------|--------|--------------------------------------------------------|
//! | Category match     | 40     | +40 if categories are identical, 0 otherwise            |
//! | Tag overlap        | 30     | Jaccard similarity × 30                                |
//! | Price compatibility| 30     | Overlap of price ranges scaled to the union range      |
//!
//! Listings whose seller is the caller are excluded from suggestions (a user
//! should not be matched against themselves).

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Storage keys for the trade-matching contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// The admin authorized to administer the contract.
    Admin,
    /// Auto-incrementing id for the next listing registration.
    NextListingId,
    /// Auto-incrementing id for the next trade proposal.
    NextProposalId,
    /// Metadata for a listing keyed by its local id.
    ListingMetadata(u64),
    /// Set of all active listing ids (for iteration during matching).
    ActiveListingIds,
    /// A trade proposal keyed by its id.
    Proposal(u64),
    /// Mapping from a listing id to the set of proposal ids it participates in.
    ListingProposals(u64),
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Rich metadata attached to a listing for matching purposes.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListingMetadata {
    /// Local id assigned by this contract.
    pub id: u64,
    /// The listing id in the marketplace / matching-engine contract.
    pub external_id: u64,
    /// The address of the user who registered this listing.
    pub seller: Address,
    /// Tags that describe the listing (e.g. "electronics", "gaming", "rare").
    pub tags: Vec<Symbol>,
    /// The category this listing belongs to (e.g. "crypto", "nft", "commodity").
    pub category: Symbol,
    /// Minimum acceptable price in the quote token.
    pub min_price: i128,
    /// Maximum acceptable price in the quote token.
    pub max_price: i128,
    /// Timestamp when the listing was registered.
    pub created_at: u64,
}

/// A single match result between two listings.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    /// The listing id this match is relative to.
    pub source_listing_id: u64,
    /// The suggested listing id.
    pub target_listing_id: u64,
    /// Relevance score in 0–100.
    pub score: u32,
    /// Seller of the target listing.
    pub target_seller: Address,
}

/// The lifecycle status of a trade proposal.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposalStatus {
    /// Proposal has been made and is awaiting a response.
    Pending,
    /// The target seller accepted the proposal.
    Accepted,
    /// The target seller rejected the proposal.
    Rejected,
}

/// A trade proposal created from a match.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeProposal {
    /// Auto-incrementing id.
    pub id: u64,
    /// The listing the proposer is offering.
    pub source_listing_id: u64,
    /// The listing being requested.
    pub target_listing_id: u64,
    /// The user who proposed the trade (must be the seller of the source listing).
    pub proposer: Address,
    /// The user whose listing is being requested (must accept to finalize).
    pub target_seller: Address,
    /// Relevance score at the time of proposal.
    pub score: u32,
    /// Current lifecycle status.
    pub status: ProposalStatus,
    /// Timestamp when the proposal was created.
    pub created_at: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors surfaced by the trade-matching contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidPriceRange = 3,
    ListingAlreadyRegistered = 4,
    ListingNotFound = 5,
    CannotMatchSelf = 6,
    NoMatchesFound = 7,
    ProposalNotFound = 8,
    ProposalNotPending = 9,
    Unauthorized = 10,
    InvalidListing = 11,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

const EVT_LISTING_REGISTERED: Symbol = symbol_short!("lst_reg");
const EVT_MATCHES_GENERATED: Symbol = symbol_short!("match_gen");
const EVT_PROPOSAL_CREATED: Symbol = symbol_short!("prop_crt");
const EVT_PROPOSAL_ACCEPTED: Symbol = symbol_short!("prop_acc");
const EVT_PROPOSAL_REJECTED: Symbol = symbol_short!("prop_rjt");

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct TradeMatching;

#[contractimpl]
impl TradeMatching {
    // -- Lifecycle ----------------------------------------------------------

    /// Initialize the contract with an admin. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextListingId, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::NextProposalId, &0u64);
        env.storage()
            .persistent()
            .set(&DataKey::ActiveListingIds, &Vec::<u64>::new(&env));
        Ok(())
    }

    // -- Listing management -------------------------------------------------

    /// Register a listing with matching metadata. Returns the local listing id.
    ///
    /// * `external_id` — the id from the marketplace or matching-engine contract.
    /// * `seller` — the listing owner (requires auth).
    /// * `tags` — descriptive tags for matching.
    /// * `category` — the listing category.
    /// * `min_price` / `max_price` — acceptable price range in the quote token.
    pub fn register_listing(
        env: Env,
        external_id: u64,
        seller: Address,
        tags: Vec<Symbol>,
        category: Symbol,
        min_price: i128,
        max_price: i128,
    ) -> Result<u64, Error> {
        Self::ensure_init(&env)?;
        seller.require_auth();

        if min_price <= 0 || max_price <= 0 || min_price > max_price {
            return Err(Error::InvalidPriceRange);
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextListingId)
            .unwrap_or(0);

        let metadata = ListingMetadata {
            id,
            external_id,
            seller,
            tags,
            category,
            min_price,
            max_price,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::ListingMetadata(id), &metadata);

        // Add to active listing ids.
        let mut active: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ActiveListingIds)
            .unwrap_or_else(|| Vec::new(&env));
        active.push_back(id);
        env.storage()
            .persistent()
            .set(&DataKey::ActiveListingIds, &active);

        env.storage()
            .instance()
            .set(&DataKey::NextListingId, &(id + 1));

        env.events().publish((EVT_LISTING_REGISTERED, id), metadata);

        Ok(id)
    }

    /// Fetch the metadata for a registered listing.
    pub fn get_listing_metadata(env: Env, id: u64) -> Result<ListingMetadata, Error> {
        Self::ensure_init(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::ListingMetadata(id))
            .ok_or(Error::ListingNotFound)
    }

    /// Return all active listing ids.
    pub fn get_active_listing_ids(env: Env) -> Result<Vec<u64>, Error> {
        Self::ensure_init(&env)?;
        let active: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ActiveListingIds)
            .unwrap_or_else(|| Vec::new(&env));
        Ok(active)
    }

    // -- Matching -----------------------------------------------------------

    /// Compute ranked suggestions for a given listing.
    ///
    /// Returns a `Vec<Match>` sorted by descending relevance score (highest
    /// first). Self-matches (same seller) are excluded.
    pub fn suggest_matches(env: Env, listing_id: u64) -> Result<Vec<Match>, Error> {
        Self::ensure_init(&env)?;

        let source: ListingMetadata = env
            .storage()
            .persistent()
            .get(&DataKey::ListingMetadata(listing_id))
            .ok_or(Error::ListingNotFound)?;

        let active: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::ActiveListingIds)
            .unwrap_or_else(|| Vec::new(&env));

        let mut matches: Vec<Match> = Vec::new(&env);

        for &other_id in active.iter() {
            if other_id == listing_id {
                continue;
            }

            let other: ListingMetadata = env
                .storage()
                .persistent()
                .get(&DataKey::ListingMetadata(other_id))
                .ok_or(Error::ListingNotFound)?;

            // Skip self-matches.
            if other.seller == source.seller {
                continue;
            }

            let score = Self::compute_score(&env, &source, &other);
            if score > 0 {
                matches.push_back(Match {
                    source_listing_id: listing_id,
                    target_listing_id: other_id,
                    score,
                    target_seller: other.seller,
                });
            }
        }

        // Sort matches by descending score using an insertion-sort (cheap for
        // typical result set sizes and works within Soroban's compute budget).
        Self::sort_matches_desc(&mut matches);

        // Emit analytics event.
        env.events().publish(
            (EVT_MATCHES_GENERATED, listing_id),
            (matches.len(), source.seller.clone()),
        );

        Ok(matches)
    }

    // -- Proposals ----------------------------------------------------------

    /// Propose a trade between two matched listings. Requires the proposer to
    /// be the seller of the source listing.
    pub fn propose_trade(
        env: Env,
        source_listing_id: u64,
        target_listing_id: u64,
    ) -> Result<u64, Error> {
        Self::ensure_init(&env)?;

        let source: ListingMetadata = env
            .storage()
            .persistent()
            .get(&DataKey::ListingMetadata(source_listing_id))
            .ok_or(Error::ListingNotFound)?;

        source.seller.require_auth();

        let target: ListingMetadata = env
            .storage()
            .persistent()
            .get(&DataKey::ListingMetadata(target_listing_id))
            .ok_or(Error::ListingNotFound)?;

        if source.seller == target.seller {
            return Err(Error::CannotMatchSelf);
        }

        // Compute score for analytics / audit trail.
        let score = Self::compute_score(&env, &source, &target);

        let proposal_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextProposalId)
            .unwrap_or(0);

        let proposal = TradeProposal {
            id: proposal_id,
            source_listing_id,
            target_listing_id,
            proposer: source.seller.clone(),
            target_seller: target.seller.clone(),
            score,
            status: ProposalStatus::Pending,
            created_at: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        // Track proposals per listing.
        Self::add_proposal_to_listing(&env, source_listing_id, proposal_id);
        Self::add_proposal_to_listing(&env, target_listing_id, proposal_id);

        env.storage()
            .instance()
            .set(&DataKey::NextProposalId, &(proposal_id + 1));

        env.events()
            .publish((EVT_PROPOSAL_CREATED, proposal_id), proposal);

        Ok(proposal_id)
    }

    /// Accept a pending trade proposal. Only the target seller may accept.
    pub fn accept_proposal(env: Env, proposal_id: u64) -> Result<TradeProposal, Error> {
        Self::ensure_init(&env)?;
        let mut proposal: TradeProposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        proposal.target_seller.require_auth();

        if proposal.status != ProposalStatus::Pending {
            return Err(Error::ProposalNotPending);
        }

        proposal.status = ProposalStatus::Accepted;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((EVT_PROPOSAL_ACCEPTED, proposal_id), proposal.clone());

        Ok(proposal)
    }

    /// Reject a pending trade proposal. Only the target seller may reject.
    pub fn reject_proposal(env: Env, proposal_id: u64) -> Result<TradeProposal, Error> {
        Self::ensure_init(&env)?;
        let mut proposal: TradeProposal = env
            .storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)?;

        proposal.target_seller.require_auth();

        if proposal.status != ProposalStatus::Pending {
            return Err(Error::ProposalNotPending);
        }

        proposal.status = ProposalStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((EVT_PROPOSAL_REJECTED, proposal_id), proposal.clone());

        Ok(proposal)
    }

    /// Fetch a trade proposal by id.
    pub fn get_proposal(env: Env, proposal_id: u64) -> Result<TradeProposal, Error> {
        Self::ensure_init(&env)?;
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .ok_or(Error::ProposalNotFound)
    }

    /// Return all proposal ids associated with a listing.
    pub fn get_listing_proposals(env: Env, listing_id: u64) -> Result<Vec<u64>, Error> {
        Self::ensure_init(&env)?;
        Ok(Self::proposals_for_listing(&env, listing_id))
    }

    // -- Internal helpers ---------------------------------------------------

    fn ensure_init(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    // -- Matching algorithm -------------------------------------------------

    /// Compute a relevance score (0–100) between two listings.
    ///
    /// Weights:
    ///   - Category:  40 points
    ///   - Tags:      30 points (Jaccard similarity)
    ///   - Price:     30 points (range overlap)
    fn compute_score(_env: &Env, a: &ListingMetadata, b: &ListingMetadata) -> u32 {
        let category_score = Self::category_score(a, b);
        let tag_score = Self::tag_score(a, b);
        let price_score = Self::price_score(a, b);
        category_score + tag_score + price_score
    }

    /// Category matching: 40 points if identical, 0 otherwise.
    fn category_score(a: &ListingMetadata, b: &ListingMetadata) -> u32 {
        if a.category == b.category {
            40
        } else {
            0
        }
    }

    /// Tag overlap using Jaccard similarity, scaled to 30 points.
    ///
    /// Jaccard(A, B) = |A ∩ B| / |A ∪ B|
    fn tag_score(a: &ListingMetadata, b: &ListingMetadata) -> u32 {
        if a.tags.is_empty() && b.tags.is_empty() {
            return 0;
        }

        let intersection = Self::tag_intersection_count(a, b);
        let union_count = a.tags.len() + b.tags.len() - intersection;

        if union_count == 0 {
            return 0;
        }

        // Use integer math: (intersection * 30 * 100) / (union * 100)
        // to avoid floats. Scale to percentage first.
        ((intersection * 30_000) / union_count) / 100
    }

    /// Count the number of tags in the intersection of two listings' tag sets.
    fn tag_intersection_count(a: &ListingMetadata, b: &ListingMetadata) -> u32 {
        let mut count: u32 = 0;
        for tag_a in a.tags.iter() {
            for tag_b in b.tags.iter() {
                if tag_a == tag_b {
                    count += 1;
                    break;
                }
            }
        }
        count
    }

    /// Price compatibility based on range overlap, scaled to 30 points.
    ///
    /// Overlap is computed as the intersection of [a.min_price, a.max_price]
    /// and [b.min_price, b.max_price]. The score is proportional to the overlap
    /// width divided by the total span of both ranges.
    fn price_score(a: &ListingMetadata, b: &ListingMetadata) -> u32 {
        let overlap_start = if a.min_price > b.min_price {
            a.min_price
        } else {
            b.min_price
        };
        let overlap_end = if a.max_price < b.max_price {
            a.max_price
        } else {
            b.max_price
        };

        let overlap = overlap_end - overlap_start;
        if overlap <= 0 {
            return 0;
        }

        // Total span = union of both ranges.
        let overall_start = if a.min_price < b.min_price {
            a.min_price
        } else {
            b.min_price
        };
        let overall_end = if a.max_price > b.max_price {
            a.max_price
        } else {
            b.max_price
        };
        let total_span = overall_end - overall_start;

        if total_span <= 0 {
            return 30; // Identical ranges → perfect score.
        }

        // (overlap / total_span) * 30, with integer math.
        ((overlap * 30_000) / total_span) / 100
    }

    // -- Helpers for proposals per listing ----------------------------------

    fn add_proposal_to_listing(env: &Env, listing_id: u64, proposal_id: u64) {
        let key = DataKey::ListingProposals(listing_id);
        let mut proposal_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        proposal_ids.push_back(proposal_id);
        env.storage().persistent().set(&key, &proposal_ids);
    }

    fn proposals_for_listing(env: &Env, listing_id: u64) -> Vec<u64> {
        let key = DataKey::ListingProposals(listing_id);
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env))
    }

    // -- Sorting ------------------------------------------------------------

    /// Sort a `Vec<Match>` by descending score using insertion sort.
    /// This is O(n²) but acceptable for the expected result set sizes
    /// (typically < 100 matches) and stays within Soroban's compute budget.
    fn sort_matches_desc(matches: &mut Vec<Match>) {
        let len = matches.len();
        if len <= 1 {
            return;
        }

        // Build sorted vector manually (Soroban Vec doesn't support in-place swap).
        let mut sorted: Vec<Match> = Vec::new(matches.env());
        for i in 0..len {
            let current = matches.get(i).unwrap();
            let mut insert_pos = sorted.len();
            for j in 0..sorted.len() {
                if current.score > sorted.get(j).unwrap().score {
                    insert_pos = j;
                    break;
                }
            }
            sorted.insert(insert_pos, current);
        }

        // Replace original.
        matches.clear();
        for item in sorted.iter() {
            matches.push_back(item);
        }
    }
}

#[cfg(test)]
mod test;
