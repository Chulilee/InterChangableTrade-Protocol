#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};

struct Fixture {
    env: Env,
    client: TradeSettlementClient<'static>,
    _admin: Address,
    buyer: Address,
    seller: Address,
    base_asset: Address,
    base_admin: token::StellarAssetClient<'static>,
    quote_asset: Address,
    quote_admin: token::StellarAssetClient<'static>,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);

    let base_asset_sac = env.register_stellar_asset_contract_v2(admin.clone());
    let base_asset = base_asset_sac.address();
    let base_admin = token::StellarAssetClient::new(&env, &base_asset);

    let quote_asset_sac = env.register_stellar_asset_contract_v2(admin.clone());
    let quote_asset = quote_asset_sac.address();
    let quote_admin = token::StellarAssetClient::new(&env, &quote_asset);

    Fixture {
        env,
        client,
        _admin: admin,
        buyer,
        seller,
        base_asset,
        base_admin,
        quote_asset,
        quote_admin,
    }
}

#[test]
fn open_and_get_trade() {
    let f = setup();
    let id = f.client.open(
        &f.buyer,
        &f.seller,
        &f.base_asset,
        &f.quote_asset,
        &100,
        &10,
    );

    let trade = f.client.get(&id);
    assert_eq!(trade.status, SettlementStatus::Pending);
    assert_eq!(trade.base_amount, 100);
    assert_eq!(trade.quote_amount, 10);
    assert_eq!(trade.failure_reason, None);
}

#[test]
fn open_invalid_amount_fails() {
    let f = setup();
    let res = f
        .client
        .try_open(&f.buyer, &f.seller, &f.base_asset, &f.quote_asset, &-1, &10);
    assert_eq!(res, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn settle_trade_atomic_success() {
    let f = setup();

    // Mint 100 base_asset to seller, 10 quote_asset to buyer
    f.base_admin.mint(&f.seller, &100);
    f.quote_admin.mint(&f.buyer, &10);

    let base_client = token::Client::new(&f.env, &f.base_asset);
    let quote_client = token::Client::new(&f.env, &f.quote_asset);

    assert_eq!(base_client.balance(&f.seller), 100);
    assert_eq!(base_client.balance(&f.buyer), 0);
    assert_eq!(quote_client.balance(&f.buyer), 10);
    assert_eq!(quote_client.balance(&f.seller), 0);

    let id = f.client.open(
        &f.buyer,
        &f.seller,
        &f.base_asset,
        &f.quote_asset,
        &100,
        &10,
    );

    let trade = f.client.settle_trade(&id, &f.seller);
    assert_eq!(trade.status, SettlementStatus::Settled);

    // Verify atomic asset transfers
    assert_eq!(base_client.balance(&f.seller), 0);
    assert_eq!(base_client.balance(&f.buyer), 100);
    assert_eq!(quote_client.balance(&f.buyer), 0);
    assert_eq!(quote_client.balance(&f.seller), 10);
}

#[test]
fn settle_batch_netted_reduces_transfers() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let charlie = Address::generate(&env);

    let xlm_sac = env.register_stellar_asset_contract_v2(admin.clone());
    let xlm = xlm_sac.address();
    let xlm_admin = token::StellarAssetClient::new(&env, &xlm);
    let xlm_client = token::Client::new(&env, &xlm);

    let usdc_sac = env.register_stellar_asset_contract_v2(admin.clone());
    let usdc = usdc_sac.address();
    let usdc_admin = token::StellarAssetClient::new(&env, &usdc);
    let usdc_client = token::Client::new(&env, &usdc);

    // Set initial balances
    // Trade 1: Alice buys 100 XLM from Bob for 10 USDC
    // Trade 2: Bob buys 60 XLM from Charlie for 6 USDC
    // Trade 3: Charlie buys 40 XLM from Alice for 4 USDC
    xlm_admin.mint(&bob, &100);
    xlm_admin.mint(&charlie, &60);
    xlm_admin.mint(&alice, &40);

    usdc_admin.mint(&alice, &10);
    usdc_admin.mint(&bob, &6);
    usdc_admin.mint(&charlie, &4);

    let t1 = client.open(&alice, &bob, &xlm, &usdc, &100, &10);
    let t2 = client.open(&bob, &charlie, &xlm, &usdc, &60, &6);
    let t3 = client.open(&charlie, &alice, &xlm, &usdc, &40, &4);

    let mut trade_ids = Vec::new(&env);
    trade_ids.push_back(t1);
    trade_ids.push_back(t2);
    trade_ids.push_back(t3);

    // Preview netting before execution
    let (obligations, original_count) = client.preview_netting(&trade_ids);
    assert_eq!(original_count, 6);
    assert_eq!(obligations.len(), 4); // Reduced from 6 to 4 transfers!

    let batch_res = client.settle_batch_netted(&trade_ids, &admin);
    assert_eq!(batch_res.status, SettlementStatus::Settled);
    assert_eq!(batch_res.original_transfers_count, 6);
    assert_eq!(batch_res.net_transfers_executed, 4);

    // Verify exact final balances:
    // Alice net XLM: +100 - 40 = +60 (starts 40 -> ends 100)
    // Alice net USDC: -10 + 4 = -6 (starts 10 -> ends 4)
    assert_eq!(xlm_client.balance(&alice), 100);
    assert_eq!(usdc_client.balance(&alice), 4);

    // Bob net XLM: -100 + 60 = -40 (starts 100 -> ends 60)
    // Bob net USDC: +10 - 6 = +4 (starts 6 -> ends 10)
    assert_eq!(xlm_client.balance(&bob), 60);
    assert_eq!(usdc_client.balance(&bob), 10);

    // Charlie net XLM: -60 + 40 = -20 (starts 60 -> ends 40)
    // Charlie net USDC: +6 - 4 = +2 (starts 4 -> ends 6)
    assert_eq!(xlm_client.balance(&charlie), 40);
    assert_eq!(usdc_client.balance(&charlie), 6);
}

#[test]
fn netting_circular_trades_zero_transfers() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(TradeSettlement, ());
    let client = TradeSettlementClient::new(&env, &contract_id);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let xlm_sac = env.register_stellar_asset_contract_v2(alice.clone());
    let xlm = xlm_sac.address();

    let usdc_sac = env.register_stellar_asset_contract_v2(alice.clone());
    let usdc = usdc_sac.address();

    // Alice buys 100 XLM from Bob for 10 USDC
    let t1 = client.open(&alice, &bob, &xlm, &usdc, &100, &10);
    // Bob buys 100 XLM from Alice for 10 USDC (perfect circular offset)
    let t2 = client.open(&bob, &alice, &xlm, &usdc, &100, &10);

    let mut trade_ids = Vec::new(&env);
    trade_ids.push_back(t1);
    trade_ids.push_back(t2);

    let batch_res = client.settle_batch_netted(&trade_ids, &alice);
    assert_eq!(batch_res.status, SettlementStatus::Settled);
    assert_eq!(batch_res.original_transfers_count, 4);
    assert_eq!(batch_res.net_transfers_executed, 0); // Completely netted out!

    assert_eq!(client.get(&t1).status, SettlementStatus::Settled);
    assert_eq!(client.get(&t2).status, SettlementStatus::Settled);
}

#[test]
fn failure_path_leaves_funds_recoverable_and_supports_retry() {
    let f = setup();

    // Seller has base asset, but buyer lacks quote asset
    f.base_admin.mint(&f.seller, &100);
    // Buyer has 0 quote asset!

    let id = f.client.open(
        &f.buyer,
        &f.seller,
        &f.base_asset,
        &f.quote_asset,
        &100,
        &10,
    );

    // Settle attempt fails due to insufficient balance
    let trade = f.client.settle_trade(&id, &f.seller);
    assert_eq!(trade.status, SettlementStatus::Failed);
    assert_eq!(
        trade.failure_reason,
        Some(Symbol::new(&f.env, "InsufficientBalance"))
    );

    // Verify funds remain untouched (recoverable)
    let base_client = token::Client::new(&f.env, &f.base_asset);
    assert_eq!(base_client.balance(&f.seller), 100);

    // Fund the buyer with missing quote asset
    f.quote_admin.mint(&f.buyer, &10);

    // Retry settlement
    let retried_trade = f.client.retry_settlement(&id, &f.seller);
    assert_eq!(retried_trade.status, SettlementStatus::Settled);
    assert_eq!(retried_trade.failure_reason, None);

    // Verify funds transferred
    assert_eq!(base_client.balance(&f.seller), 0);
    assert_eq!(base_client.balance(&f.buyer), 100);
}

#[test]
fn unauthorized_settlement_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let id = f.client.open(
        &f.buyer,
        &f.seller,
        &f.base_asset,
        &f.quote_asset,
        &100,
        &10,
    );
    let res = f.client.try_settle_trade(&id, &stranger);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn cancel_trade_success() {
    let f = setup();
    let id = f.client.open(
        &f.buyer,
        &f.seller,
        &f.base_asset,
        &f.quote_asset,
        &100,
        &10,
    );
    let trade = f.client.cancel(&id, &f.buyer);
    assert_eq!(trade.status, SettlementStatus::Cancelled);
}
