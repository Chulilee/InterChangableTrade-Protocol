#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

struct Fixture {
    env: Env,
    client: TradeMatchingClient<'static>,
    admin: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1000);

    let contract_id = env.register(TradeMatching, ());
    let client = TradeMatchingClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    Fixture { env, client, admin }
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let f = setup();
    // Verify we can call get_active_listing_ids (initialized contract).
    let ids = f.client.get_active_listing_ids();
    assert_eq!(ids.len(), 0);
}

#[test]
fn test_initialize_twice_fails() {
    let f = setup();
    let res = f.client.try_initialize(&Address::generate(&f.env));
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_operations_before_init_fail() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(TradeMatching, ());
    let client = TradeMatchingClient::new(&env, &contract_id);

    let seller = Address::generate(&env);
    let res = client.try_register_listing(
        &0,
        &seller,
        &Vec::<Symbol>::new(&env),
        &symbol_short!("cat"),
        &100,
        &200,
    );
    assert_eq!(res, Err(Ok(Error::NotInitialized)));
}

// ---------------------------------------------------------------------------
// Listing registration
// ---------------------------------------------------------------------------

#[test]
fn test_register_listing() {
    let f = setup();
    let seller = Address::generate(&f.env);

    let mut tags = Vec::new(&f.env);
    tags.push_back(symbol_short!("elec"));
    tags.push_back(symbol_short!("phone"));

    let id = f
        .client
        .register_listing(&10, &seller, &tags, &symbol_short!("gadget"), &50, &150);
    assert_eq!(id, 0);

    let meta = f.client.get_listing_metadata(&0);
    assert_eq!(meta.external_id, 10);
    assert_eq!(meta.seller, seller);
    assert_eq!(meta.category, symbol_short!("gadget"));
    assert_eq!(meta.min_price, 50);
    assert_eq!(meta.max_price, 150);
    assert_eq!(meta.tags.len(), 2);

    let active = f.client.get_active_listing_ids();
    assert_eq!(active.len(), 1);
    assert_eq!(active.get(0).unwrap(), 0);
}

#[test]
fn test_register_listing_invalid_price_range() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    // min > max
    let res = f
        .client
        .try_register_listing(&0, &seller, &tags, &symbol_short!("cat"), &200, &100);
    assert_eq!(res, Err(Ok(Error::InvalidPriceRange)));

    // zero min
    let res = f
        .client
        .try_register_listing(&0, &seller, &tags, &symbol_short!("cat"), &0, &100);
    assert_eq!(res, Err(Ok(Error::InvalidPriceRange)));

    // negative max
    let res = f
        .client
        .try_register_listing(&0, &seller, &tags, &symbol_short!("cat"), &50, &-1);
    assert_eq!(res, Err(Ok(Error::InvalidPriceRange)));
}

#[test]
fn test_register_multiple_listings() {
    let f = setup();
    let s1 = Address::generate(&f.env);
    let s2 = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id1 = f
        .client
        .register_listing(&1, &s1, &tags, &symbol_short!("a"), &10, &20);
    let id2 = f
        .client
        .register_listing(&2, &s2, &tags, &symbol_short!("b"), &30, &40);

    assert_eq!(id1, 0);
    assert_eq!(id2, 1);

    let active = f.client.get_active_listing_ids();
    assert_eq!(active.len(), 2);
}

#[test]
fn test_get_listing_metadata_not_found() {
    let f = setup();
    let res = f.client.try_get_listing_metadata(&999);
    assert_eq!(res, Err(Ok(Error::ListingNotFound)));
}

// ---------------------------------------------------------------------------
// Matching algorithm — unit-level scoring
// ---------------------------------------------------------------------------

#[test]
fn test_score_same_category_full_points() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let meta = ListingMetadata {
        id: 0,
        external_id: 0,
        seller,
        tags,
        category: symbol_short!("crypto"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let meta2 = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("crypto"),
        min_price: 150,
        max_price: 250,
        created_at: 1000,
    };

    let score = TradeMatching::compute_score(&f.env, &meta, &meta2);
    // Category: 40 (match), Tags: 0 (both empty), Price: 30 (overlap [150,200] / total [100,250])
    // Price overlap = 50, total span = 150. (50*30000/150)/100 = 10. So total = 40+0+10 = 50.
    assert!(score >= 40);
    assert!(score <= 70);
}

#[test]
fn test_score_different_category_zero_category_points() {
    let f = setup();
    let meta = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("crypto"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let meta2 = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("nft"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let score = TradeMatching::compute_score(&f.env, &meta, &meta2);
    // Category: 0, Tags: 0, Price: 30 (identical ranges).
    assert_eq!(score, 30);
}

#[test]
fn test_tag_overlap_scoring() {
    let f = setup();
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: {
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("a"));
            t.push_back(symbol_short!("b"));
            t.push_back(symbol_short!("c"));
            t
        },
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: {
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("b"));
            t.push_back(symbol_short!("c"));
            t.push_back(symbol_short!("d"));
            t
        },
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let tag_score = TradeMatching::tag_score(&a, &b);
    // Jaccard = 2/4 = 0.5 → 0.5 * 30 = 15
    assert_eq!(tag_score, 15);
}

#[test]
fn test_tag_no_overlap() {
    let f = setup();
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: {
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("a"));
            t
        },
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: {
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("z"));
            t
        },
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let tag_score = TradeMatching::tag_score(&a, &b);
    assert_eq!(tag_score, 0);
}

#[test]
fn test_tag_full_overlap() {
    let f = setup();
    let tags = {
        let mut t = Vec::new(&f.env);
        t.push_back(symbol_short!("x"));
        t.push_back(symbol_short!("y"));
        t
    };
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: tags.clone(),
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags,
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let tag_score = TradeMatching::tag_score(&a, &b);
    // Jaccard = 2/2 = 1.0 → 30
    assert_eq!(tag_score, 30);
}

#[test]
fn test_price_no_overlap() {
    let f = setup();
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 10,
        max_price: 20,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let score = TradeMatching::price_score(&a, &b);
    assert_eq!(score, 0);
}

#[test]
fn test_price_identical_ranges() {
    let f = setup();
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };

    let score = TradeMatching::price_score(&a, &b);
    assert_eq!(score, 30);
}

#[test]
fn test_price_partial_overlap() {
    let f = setup();
    let a = ListingMetadata {
        id: 0,
        external_id: 0,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 100,
        max_price: 200,
        created_at: 1000,
    };
    let b = ListingMetadata {
        id: 1,
        external_id: 1,
        seller: Address::generate(&f.env),
        tags: Vec::new(&f.env),
        category: symbol_short!("cat"),
        min_price: 150,
        max_price: 250,
        created_at: 1000,
    };

    // overlap = [150, 200] = 50, total = [100, 250] = 150
    // (50*30000/150)/100 = 10
    let score = TradeMatching::price_score(&a, &b);
    assert_eq!(score, 10);
}

// ---------------------------------------------------------------------------
// suggest_matches — integration-level
// ---------------------------------------------------------------------------

#[test]
fn test_suggest_matches_empty() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id = f
        .client
        .register_listing(&0, &seller, &tags, &symbol_short!("cat"), &100, &200);
    let matches = f.client.suggest_matches(&id);
    assert_eq!(matches.len(), 0);
}

#[test]
fn test_suggest_matches_excludes_self() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id1 = f
        .client
        .register_listing(&1, &seller, &tags, &symbol_short!("cat"), &100, &200);
    let _id2 = f
        .client
        .register_listing(&2, &seller, &tags, &symbol_short!("cat"), &100, &200);

    let matches = f.client.suggest_matches(&id1);
    assert_eq!(matches.len(), 0); // Same seller → excluded.
}

#[test]
fn test_suggest_matches_returns_ranked() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let seller_c = Address::generate(&f.env);

    // Listing A: the source
    let id_a = f.client.register_listing(
        &1,
        &seller_a,
        &{
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("elec"));
            t
        },
        &symbol_short!("tech"),
        &100,
        &200,
    );

    // Listing B: high match (same category, overlapping tags, overlapping price)
    let _id_b = f.client.register_listing(
        &2,
        &seller_b,
        &{
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("elec"));
            t.push_back(symbol_short!("rare"));
            t
        },
        &symbol_short!("tech"),
        &120,
        &180,
    );

    // Listing C: low match (different category, no shared tags, no price overlap)
    let _id_c = f.client.register_listing(
        &3,
        &seller_c,
        &{
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("food"));
            t
        },
        &symbol_short!("meal"),
        &500,
        &600,
    );

    let matches = f.client.suggest_matches(&id_a);
    assert_eq!(matches.len(), 2); // B is a match, C might score 0 (no price overlap).

    // B should rank higher than C.
    let b_match = matches.get(0).unwrap();
    assert_eq!(b_match.target_listing_id, 1); // _id_b = 1
    assert!(b_match.score > 0);

    // Verify ordering: scores are non-increasing.
    let first_score = matches.get(0).unwrap().score;
    let second_score = matches.get(1).unwrap().score;
    assert!(first_score >= second_score);
}

#[test]
fn test_suggest_matches_not_found() {
    let f = setup();
    let res = f.client.try_suggest_matches(&999);
    assert_eq!(res, Err(Ok(Error::ListingNotFound)));
}

// ---------------------------------------------------------------------------
// Propose / Accept / Reject
// ---------------------------------------------------------------------------

#[test]
fn test_propose_trade() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    let proposal_id = f.client.propose_trade(&id_a, &id_b);
    assert_eq!(proposal_id, 0);

    let proposal = f.client.get_proposal(&0);
    assert_eq!(proposal.source_listing_id, id_a);
    assert_eq!(proposal.target_listing_id, id_b);
    assert_eq!(proposal.proposer, seller_a);
    assert_eq!(proposal.target_seller, seller_b);
    assert_eq!(proposal.status, ProposalStatus::Pending);

    // Verify proposals are tracked per listing.
    let a_proposals = f.client.get_listing_proposals(&id_a);
    assert_eq!(a_proposals.len(), 1);
    assert_eq!(a_proposals.get(0).unwrap(), 0);

    let b_proposals = f.client.get_listing_proposals(&id_b);
    assert_eq!(b_proposals.len(), 1);
    assert_eq!(b_proposals.get(0).unwrap(), 0);
}

#[test]
fn test_accept_proposal() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    let proposal_id = f.client.propose_trade(&id_a, &id_b);
    let accepted = f.client.accept_proposal(&proposal_id);
    assert_eq!(accepted.status, ProposalStatus::Accepted);

    // Verify persisted.
    let stored = f.client.get_proposal(&proposal_id);
    assert_eq!(stored.status, ProposalStatus::Accepted);
}

#[test]
fn test_reject_proposal() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    let proposal_id = f.client.propose_trade(&id_a, &id_b);
    let rejected = f.client.reject_proposal(&proposal_id);
    assert_eq!(rejected.status, ProposalStatus::Rejected);

    let stored = f.client.get_proposal(&proposal_id);
    assert_eq!(stored.status, ProposalStatus::Rejected);
}

#[test]
fn test_accept_nonexistent_proposal_fails() {
    let f = setup();
    let res = f.client.try_accept_proposal(&999);
    assert_eq!(res, Err(Ok(Error::ProposalNotFound)));
}

#[test]
fn test_accept_already_accepted_fails() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    let proposal_id = f.client.propose_trade(&id_a, &id_b);
    f.client.accept_proposal(&proposal_id);

    let res = f.client.try_accept_proposal(&proposal_id);
    assert_eq!(res, Err(Ok(Error::ProposalNotPending)));
}

#[test]
fn test_reject_already_rejected_fails() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    let proposal_id = f.client.propose_trade(&id_a, &id_b);
    f.client.reject_proposal(&proposal_id);

    let res = f.client.try_reject_proposal(&proposal_id);
    assert_eq!(res, Err(Ok(Error::ProposalNotPending)));
}

#[test]
fn test_propose_self_trade_fails() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id_a = f
        .client
        .register_listing(&1, &seller, &tags, &symbol_short!("cat"), &100, &200);
    let id_b = f
        .client
        .register_listing(&2, &seller, &tags, &symbol_short!("cat"), &100, &200);

    let res = f.client.try_propose_trade(&id_a, &id_b);
    assert_eq!(res, Err(Ok(Error::CannotMatchSelf)));
}

#[test]
fn test_propose_listing_not_found() {
    let f = setup();
    let res = f.client.try_propose_trade(&1, &2);
    assert_eq!(res, Err(Ok(Error::ListingNotFound)));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_empty_tags_both_listings() {
    let f = setup();
    let s1 = Address::generate(&f.env);
    let s2 = Address::generate(&f.env);
    let tags = Vec::<Symbol>::new(&f.env);

    let id1 = f
        .client
        .register_listing(&1, &s1, &tags, &symbol_short!("a"), &100, &200);
    let _id2 = f
        .client
        .register_listing(&2, &s2, &tags, &symbol_short!("a"), &100, &200);

    let matches = f.client.suggest_matches(&id1);
    assert_eq!(matches.len(), 1);
    // Score: category 40 + tags 0 + price 30 = 70
    assert_eq!(matches.get(0).unwrap().score, 70);
}

#[test]
fn test_perfect_match_score() {
    let f = setup();
    let s1 = Address::generate(&f.env);
    let s2 = Address::generate(&f.env);
    let tags = {
        let mut t = Vec::new(&f.env);
        t.push_back(symbol_short!("x"));
        t
    };

    let id1 = f
        .client
        .register_listing(&1, &s1, &tags, &symbol_short!("cat"), &100, &200);
    let _id2 = f
        .client
        .register_listing(&2, &s2, &tags, &symbol_short!("cat"), &100, &200);

    let matches = f.client.suggest_matches(&id1);
    assert_eq!(matches.len(), 1);
    // Category 40 + tags 30 + price 30 = 100
    assert_eq!(matches.get(0).unwrap().score, 100);
}

#[test]
fn test_zero_score_matches_excluded() {
    let f = setup();
    let s1 = Address::generate(&f.env);
    let s2 = Address::generate(&f.env);

    let id1 = f.client.register_listing(
        &1,
        &s1,
        &{
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("a"));
            t
        },
        &symbol_short!("cat1"),
        &10,
        &20,
    );
    let _id2 = f.client.register_listing(
        &2,
        &s2,
        &{
            let mut t = Vec::new(&f.env);
            t.push_back(symbol_short!("z"));
            t
        },
        &symbol_short!("cat2"),
        &500,
        &600,
    );

    let matches = f.client.suggest_matches(&id1);
    // Different category (0), no tag overlap (0), no price overlap (0) → 0 total → excluded.
    assert_eq!(matches.len(), 0);
}

#[test]
fn test_sorting_stability() {
    let f = setup();
    let seller_a = Address::generate(&f.env);
    let seller_b = Address::generate(&f.env);
    let seller_c = Address::generate(&f.env);
    let tags = {
        let mut t = Vec::new(&f.env);
        t.push_back(symbol_short!("x"));
        t
    };

    // Source listing
    let id_a = f
        .client
        .register_listing(&1, &seller_a, &tags, &symbol_short!("cat"), &100, &200);

    // Three targets with varying match quality
    // B: perfect match (score 100)
    let _id_b = f
        .client
        .register_listing(&2, &seller_b, &tags, &symbol_short!("cat"), &100, &200);

    // C: same category, no tags, no price overlap → 40 (category only)
    let _id_c = f.client.register_listing(
        &3,
        &seller_c,
        &Vec::<Symbol>::new(&f.env),
        &symbol_short!("cat"),
        &500,
        &600,
    );

    let matches = f.client.suggest_matches(&id_a);
    assert_eq!(matches.len(), 2);

    // B should come first (higher score).
    let first = matches.get(0).unwrap();
    let second = matches.get(1).unwrap();
    assert!(first.score >= second.score);
    assert_eq!(first.target_listing_id, 1); // id_b = 1
    assert_eq!(second.target_listing_id, 2); // id_c = 2
}
