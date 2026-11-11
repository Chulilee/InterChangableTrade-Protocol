#![cfg(test)]

//! Integration tests that verify contracts work together correctly.
//!
//! These tests deploy multiple contracts into a single Soroban `Env` and
//! exercise the cross-contract flows documented in `docs/architecture.md`:
//!
//! 1. **Marketplace → Escrow → Settlement**: a seller lists an asset, a buyer
//!    fills the listing, funds an escrow, and the trade settles with tokens
//!    moving between both parties.
//! 2. **Matching Engine → Trade Settlement**: a buy order and a sell order
//!    cross in the order book, the match emits a trade, and settlement
//!    transfers the assets atomically.

use access_control::{AccessControl, AccessControlClient};
use escrow::{EscrowContract, EscrowContractClient, State};
use marketplace::{Marketplace, MarketplaceClient, Status as ListingStatus};
use matching_engine::{MatchingEngine, MatchingEngineClient, OrderSide};
use risk_management::{RiskManager, RiskManagerClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};
use trade_settlement::{SettlementStatus, TradeSettlement, TradeSettlementClient};

/// Shared fixture: deploys the marketplace, escrow, settlement, and matching
/// engine contracts plus two token contracts (base asset and quote token)
/// into a single environment.
struct ProtocolFixture {
    env: Env,
    seller: Address,
    buyer: Address,
    base_token: Address,
    quote_token: Address,
    marketplace: MarketplaceClient<'static>,
    escrow: EscrowContractClient<'static>,
    settlement: TradeSettlementClient<'static>,
    matching: MatchingEngineClient<'static>,
}

/// Deploy a Stellar Asset Contract token and mint `amount` to `owner`.
fn create_token(env: &Env, admin: &Address, owner: &Address, amount: i128) -> Address {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let token_address = sac.address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &amount);
    token_address
}

fn setup() -> ProtocolFixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    env.ledger().with_mut(|li| {
        li.timestamp = 1_000_000;
    });

    let admin = Address::generate(&env);
    let seller = Address::generate(&env);
    let buyer = Address::generate(&env);

    // Deploy tokens: base asset (what seller offers) and quote token (what buyer pays).
    let base_token = create_token(&env, &admin, &seller, 1_000);
    let quote_token = create_token(&env, &admin, &buyer, 10_000);

    // Deploy access-control and grant the deployer the ADMIN role.
    let ac_id = env.register(AccessControl, ());
    let ac = AccessControlClient::new(&env, &ac_id);
    ac.initialize(&admin);

    // Deploy marketplace with a risk manager wired in.
    let marketplace_id = env.register(Marketplace, ());
    let marketplace = MarketplaceClient::new(&env, &marketplace_id);
    marketplace.initialize(&admin, &ac_id);

    let rm_id = env.register(RiskManager, ());
    let risk = RiskManagerClient::new(&env, &rm_id);
    risk.initialize(&admin, &ac_id, &1_000_000);
    marketplace.set_risk_manager(&admin, &rm_id);

    // Deploy escrow.
    let escrow_id = env.register(EscrowContract, ());
    let escrow = EscrowContractClient::new(&env, &escrow_id);

    // Deploy trade settlement.
    let settlement_id = env.register(TradeSettlement, ());
    let settlement = TradeSettlementClient::new(&env, &settlement_id);

    // Deploy matching engine.
    let matching_id = env.register(MatchingEngine, ());
    let matching = MatchingEngineClient::new(&env, &matching_id);
    matching.initialize(&admin);

    ProtocolFixture {
        env,
        seller,
        buyer,
        base_token,
        quote_token,
        marketplace,
        escrow,
        settlement,
        matching,
    }
}

// ---------------------------------------------------------------------------
// Flow 1: Marketplace → Escrow → Settlement
// ---------------------------------------------------------------------------

/// End-to-end happy path: seller lists, buyer fills, buyer funds escrow,
/// trade settles, escrow releases to seller.
#[test]
fn marketplace_escrow_settlement_happy_path() {
    let f = setup();

    // 1. Seller lists 10 units of base_token for 100 quote_token.
    let listing_id =
        f.marketplace
            .create_listing(&f.seller, &f.base_token, &f.quote_token, &10, &100);
    let listing = f.marketplace.get(&listing_id);
    assert_eq!(listing.status, ListingStatus::Open);

    // 2. Buyer fills the listing.
    let filled = f.marketplace.fill_listing(&listing_id, &f.buyer);
    assert_eq!(filled.status, ListingStatus::Filled);

    // 3. Buyer funds escrow with 100 quote_token.
    f.escrow
        .fund(&1u64, &f.buyer, &f.seller, &f.quote_token, &100);
    let escrow = f.escrow.get(&1u64);
    assert_eq!(escrow.state, State::Funded);
    assert_eq!(escrow.amount, 100);

    // Verify buyer's quote balance decreased.
    let quote_client = soroban_sdk::token::Client::new(&f.env, &f.quote_token);
    assert_eq!(quote_client.balance(&f.buyer), 10_000 - 100);

    // 4. Open a settlement trade: buyer gives 100 quote, seller gives 10 base.
    let trade_id = f.settlement.open(
        &f.buyer,
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &10,
        &100,
    );
    let trade = f.settlement.get(&trade_id);
    assert_eq!(trade.status, SettlementStatus::Pending);

    // 5. Settle the trade — tokens move atomically.
    let settled = f.settlement.settle_trade(&trade_id, &f.buyer);
    assert_eq!(settled.status, SettlementStatus::Settled);

    // Verify balances after settlement: seller got 100 quote, buyer got 10 base.
    // Buyer paid 100 to escrow + 100 to seller = 200 total from 10 000.
    assert_eq!(quote_client.balance(&f.seller), 100);
    assert_eq!(quote_client.balance(&f.buyer), 10_000 - 200);
    let base_client = soroban_sdk::token::Client::new(&f.env, &f.base_token);
    assert_eq!(base_client.balance(&f.buyer), 10);
    assert_eq!(base_client.balance(&f.seller), 1_000 - 10);

    // 6. Release escrow to seller (buyer confirms receipt).
    let released = f.escrow.release(&1u64);
    assert_eq!(released.state, State::Released);

    // Seller's quote balance includes the escrowed 100.
    assert_eq!(quote_client.balance(&f.seller), 100 + 100);
}

/// Refund path: seller cancels after buyer funded escrow; escrow returns funds.
#[test]
fn marketplace_escrow_refund_path() {
    let f = setup();

    let listing_id =
        f.marketplace
            .create_listing(&f.seller, &f.base_token, &f.quote_token, &10, &100);
    f.marketplace.fill_listing(&listing_id, &f.buyer);

    // Buyer funds escrow.
    f.escrow
        .fund(&2u64, &f.buyer, &f.seller, &f.quote_token, &100);
    let quote_client = soroban_sdk::token::Client::new(&f.env, &f.quote_token);
    assert_eq!(quote_client.balance(&f.buyer), 10_000 - 100);

    // Seller refunds the escrow (unwinds the trade).
    let refunded = f.escrow.refund(&2u64);
    assert_eq!(refunded.state, State::Refunded);

    // Buyer's quote balance restored.
    assert_eq!(quote_client.balance(&f.buyer), 10_000);
}

/// Settlement fails when buyer lacks quote tokens; escrow still holds funds.
#[test]
fn settlement_fails_on_insufficient_balance() {
    let f = setup();

    // Give buyer only 50 quote (less than the 100 needed).
    let poor_buyer = Address::generate(&f.env);
    let quote_client = soroban_sdk::token::StellarAssetClient::new(&f.env, &f.quote_token);
    quote_client.mint(&poor_buyer, &50);

    let trade_id = f.settlement.open(
        &poor_buyer,
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &10,
        &100,
    );
    let result = f.settlement.settle_trade(&trade_id, &poor_buyer);
    assert_eq!(result.status, SettlementStatus::Failed);
    assert!(result.failure_reason.is_some());
}

// ---------------------------------------------------------------------------
// Flow 2: Matching Engine → Trade Settlement
// ---------------------------------------------------------------------------

/// Matching engine produces a trade; settlement transfers the assets.
#[test]
fn matching_engine_to_settlement_flow() {
    let f = setup();

    // 1. Place a sell order: seller offers 10 base at price 100 quote each.
    let sell_id = f.matching.place_order(
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Sell,
        &100,
        &10,
    );

    // 2. Place a buy order: buyer wants 10 base at price 100 quote each.
    let buy_id = f.matching.place_order(
        &f.buyer,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Buy,
        &100,
        &10,
    );

    // 3. Match orders — should execute a single trade.
    let match_result = f.matching.match_orders(&f.base_token, &f.quote_token);
    assert_eq!(match_result.trades.len(), 1);
    assert!(!match_result.more_available);
    let trade = match_result.trades.get(0).unwrap();
    assert_eq!(trade.buyer, f.buyer);
    assert_eq!(trade.seller, f.seller);
    assert_eq!(trade.quantity, 10);
    assert_eq!(trade.price, 100);

    // 4. Open a settlement trade based on the matched trade.
    let settlement_id = f.settlement.open(
        &f.buyer,
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &trade.quantity,
        &(trade.price * trade.quantity),
    );

    // 5. Settle atomically.
    let settled = f.settlement.settle_trade(&settlement_id, &f.buyer);
    assert_eq!(settled.status, SettlementStatus::Settled);

    // Verify token movements.
    let base_client = soroban_sdk::token::Client::new(&f.env, &f.base_token);
    let quote_client = soroban_sdk::token::Client::new(&f.env, &f.quote_token);
    assert_eq!(base_client.balance(&f.buyer), 10);
    assert_eq!(base_client.balance(&f.seller), 1_000 - 10);
    assert_eq!(quote_client.balance(&f.seller), 10 * 100);
    assert_eq!(quote_client.balance(&f.buyer), 10_000 - 10 * 100);

    // 6. Verify orders are marked Filled.
    let buy_order = f.matching.get_order(&f.base_token, &f.quote_token, &buy_id);
    let sell_order = f
        .matching
        .get_order(&f.base_token, &f.quote_token, &sell_id);
    assert_eq!(buy_order.status, matching_engine::OrderStatus::Filled);
    assert_eq!(sell_order.status, matching_engine::OrderStatus::Filled);
}

/// Partial fill: buy order larger than sell order; settlement covers the fill.
#[test]
fn matching_engine_partial_fill_then_settlement() {
    let f = setup();

    // Seller offers 5 units; buyer wants 10.
    f.matching.place_order(
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Sell,
        &100,
        &5,
    );
    f.matching.place_order(
        &f.buyer,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Buy,
        &100,
        &10,
    );

    let match_result = f.matching.match_orders(&f.base_token, &f.quote_token);
    assert_eq!(match_result.trades.len(), 1);
    assert!(!match_result.more_available);
    let trade = match_result.trades.get(0).unwrap();
    assert_eq!(trade.quantity, 5); // Only 5 matched.

    // Settle the matched quantity.
    let settlement_id = f.settlement.open(
        &f.buyer,
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &trade.quantity,
        &(trade.price * trade.quantity),
    );
    let settled = f.settlement.settle_trade(&settlement_id, &f.buyer);
    assert_eq!(settled.status, SettlementStatus::Settled);

    let base_client = soroban_sdk::token::Client::new(&f.env, &f.base_token);
    assert_eq!(base_client.balance(&f.buyer), 5);
}

/// Cancelled order does not produce a settlement.
#[test]
fn cancelled_order_produces_no_trade() {
    let f = setup();

    let _sell_id = f.matching.place_order(
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Sell,
        &100,
        &10,
    );

    // Buyer places then cancels.
    let buy_id = f.matching.place_order(
        &f.buyer,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Buy,
        &100,
        &10,
    );
    f.matching
        .cancel_order(&f.base_token, &f.quote_token, &buy_id, &f.buyer);

    // Matching should produce no trades (only one side remains).
    let match_result = f.matching.match_orders(&f.base_token, &f.quote_token);
    assert_eq!(match_result.trades.len(), 0);
    assert!(!match_result.more_available);
}

// ---------------------------------------------------------------------------
// Cross-flow: full lifecycle combining both flows
// ---------------------------------------------------------------------------

/// Full lifecycle: listing → fill → escrow → settlement → release, then a
/// second trade via the matching engine → settlement path.
#[test]
fn full_protocol_lifecycle() {
    let f = setup();

    // --- Phase 1: Marketplace path ---
    let listing_id =
        f.marketplace
            .create_listing(&f.seller, &f.base_token, &f.quote_token, &10, &100);
    f.marketplace.fill_listing(&listing_id, &f.buyer);
    f.escrow
        .fund(&10u64, &f.buyer, &f.seller, &f.quote_token, &100);

    let trade_id = f.settlement.open(
        &f.buyer,
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &10,
        &100,
    );
    let settled = f.settlement.settle_trade(&trade_id, &f.buyer);
    assert_eq!(settled.status, SettlementStatus::Settled);
    f.escrow.release(&10u64);

    // --- Phase 2: Matching engine path ---
    // After phase 1: buyer has 9 800 quote (10 000 − 100 escrow − 100 settle),
    // seller has 200 quote (100 settle + 100 escrow release) and 990 base.
    let base_client = soroban_sdk::token::Client::new(&f.env, &f.base_token);
    let quote_client = soroban_sdk::token::Client::new(&f.env, &f.quote_token);
    assert_eq!(base_client.balance(&f.buyer), 10);
    assert_eq!(base_client.balance(&f.seller), 990);
    assert_eq!(quote_client.balance(&f.seller), 200);
    assert_eq!(quote_client.balance(&f.buyer), 9_800);

    // New matching-engine trade: 5 units at price 200.
    f.matching.place_order(
        &f.seller,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Sell,
        &200,
        &5,
    );
    f.matching.place_order(
        &f.buyer,
        &f.base_token,
        &f.quote_token,
        &OrderSide::Buy,
        &200,
        &5,
    );
    let match_result = f.matching.match_orders(&f.base_token, &f.quote_token);
    assert_eq!(match_result.trades.len(), 1);
    assert!(!match_result.more_available);
    let mt = match_result.trades.get(0).unwrap();

    let settlement2 = f.settlement.open(
        &mt.buyer,
        &mt.seller,
        &mt.asset,
        &mt.quote,
        &mt.quantity,
        &(mt.price * mt.quantity),
    );
    let settled2 = f.settlement.settle_trade(&settlement2, &mt.buyer);
    assert_eq!(settled2.status, SettlementStatus::Settled);

    // Final balances: buyer has 15 base, 8 800 quote; seller has 985 base, 1 200 quote.
    assert_eq!(base_client.balance(&f.buyer), 15);
    assert_eq!(base_client.balance(&f.seller), 985);
    assert_eq!(quote_client.balance(&f.buyer), 8_800);
    assert_eq!(quote_client.balance(&f.seller), 1_200);
}
