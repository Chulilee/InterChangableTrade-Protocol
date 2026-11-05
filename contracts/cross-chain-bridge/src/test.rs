#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};

const BRIDGE_FEE: i128 = 5;
const DEPOSIT: i128 = 1_000;

/// Shared fixture: real Stellar Asset Contracts for the locked asset and the
/// wrapped asset (the bridge is the wrapped-asset admin so it can mint on
/// completion). The bridge starts initialized with 3 validators, a required
/// attestation threshold of 2, and a 5-unit fee routed to `fee_address`.
struct BridgeFixture {
    env: Env,
    client: CrossChainBridgeClient<'static>,
    contract_id: Address,
    admin: Address,
    fee_address: Address,
    v1: Address,
    v2: Address,
    v3: Address,
    asset: Address,
    asset_admin: token::StellarAssetClient<'static>,
    wrapped_asset: Address,
    user: Address,
}

fn setup() -> BridgeFixture {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();

    let contract_id = env.register(CrossChainBridge, ());
    let client = CrossChainBridgeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let fee_address = Address::generate(&env);
    let v1 = Address::generate(&env);
    let v2 = Address::generate(&env);
    let v3 = Address::generate(&env);

    // Locked asset: an SAC whose admin (the bridge admin) mints to the user.
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let asset = sac.address();
    let asset_admin = token::StellarAssetClient::new(&env, &asset);
    asset_admin.mint(&user, &10_000);

    // Wrapped asset: SAC owned by the bridge contract, matching the
    // register_asset wiring the bridge expects at completion time.
    let wrapped_asset = env
        .register_stellar_asset_contract_v2(contract_id.clone())
        .address();

    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(v1.clone());
    validators.push_back(v2.clone());
    validators.push_back(v3.clone());

    client.initialize(&admin, &validators, &2u32, &BRIDGE_FEE, &fee_address);

    client.register_asset(&asset, &wrapped_asset);

    BridgeFixture {
        env,
        client,
        contract_id,
        admin,
        fee_address,
        v1,
        v2,
        v3,
        asset,
        asset_admin,
        wrapped_asset,
        user,
    }
}

fn lock(f: &BridgeFixture, amount: i128) -> u64 {
    let to_chain = String::from_str(&f.env, "ETH");
    let to_address = String::from_str(&f.env, "0xabc123");
    f.client
        .lock_asset(&f.user, &f.asset, &amount, &to_chain, &to_address)
}

fn asset_client(f: &BridgeFixture) -> token::Client<'static> {
    token::Client::new(&f.env, &f.asset)
}

fn wrapped_client(f: &BridgeFixture) -> token::Client<'static> {
    token::Client::new(&f.env, &f.wrapped_asset)
}

/* ------------------------------------------------------------------ */
/* Initialization                                                      */
/* ------------------------------------------------------------------ */

#[test]
fn test_initialize_sets_state_and_validators() {
    let f = setup();

    let state: BridgeState = f.env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 3);
    assert_eq!(state.required_attestations, 2);
    assert!(!state.is_paused);
    assert_eq!(state.next_transaction_id, 0);
    assert_eq!(state.fee, BRIDGE_FEE);
    assert_eq!(state.fee_address, f.fee_address);

    let stored_admin: Address = f.env.storage().instance().get(&DataKey::Admin).unwrap();
    assert_eq!(stored_admin, f.admin);

    let stored_validators: Vec<Validator> = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Validators)
        .unwrap();
    assert_eq!(stored_validators.len(), 3);
}

#[test]
fn test_initialize_twice_fails() {
    let f = setup();

    let mut validators: Vec<Address> = Vec::new(&f.env);
    validators.push_back(f.v1.clone());
    let res = f
        .client
        .try_initialize(&f.admin, &validators, &1u32, &BRIDGE_FEE, &f.fee_address);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn test_initialize_threshold_exceeding_validators_fails() {
    let env = Env::default();
    env.mock_all_auths_allowing_non_root_auth();
    let contract_id = env.register(CrossChainBridge, ());
    let client = CrossChainBridgeClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let fee_address = Address::generate(&env);
    let mut validators: Vec<Address> = Vec::new(&env);
    validators.push_back(Address::generate(&env));
    validators.push_back(Address::generate(&env));

    let res = client.try_initialize(&admin, &validators, &3u32, &BRIDGE_FEE, &fee_address);
    assert_eq!(res, Err(Ok(Error::InvalidAttestationThreshold)));
}

/* ------------------------------------------------------------------ */
/* lock_asset                                                          */
/* ------------------------------------------------------------------ */

#[test]
fn test_fixture_can_mint_more_assets_to_user() {
    let f = setup();

    f.asset_admin.mint(&f.user, &500);

    assert_eq!(asset_client(&f).balance(&f.user), 10_500);
}

#[test]
fn test_lock_asset_deducts_fee_and_transfers_remainder() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    assert_eq!(asset_client(&f).balance(&f.user), 10_000 - DEPOSIT);
    assert_eq!(asset_client(&f).balance(&f.fee_address), BRIDGE_FEE);
    assert_eq!(
        asset_client(&f).balance(&f.contract_id),
        DEPOSIT - BRIDGE_FEE
    );

    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.amount, DEPOSIT - BRIDGE_FEE);
    assert_eq!(transaction.status, TransactionStatus::Pending);
    assert_eq!(transaction.attestations.len(), 0);

    let state: BridgeState = f.env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.next_transaction_id, 1);
}

#[test]
fn test_lock_records_full_transaction() {
    let f = setup();
    let to_chain = String::from_str(&f.env, "ETH");
    let to_address = String::from_str(&f.env, "0xabc123");

    let tx_id = f
        .client
        .lock_asset(&f.user, &f.asset, &DEPOSIT, &to_chain, &to_address);

    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.id, tx_id);
    assert_eq!(transaction.user, f.user);
    assert_eq!(transaction.asset, f.asset);
    assert_eq!(transaction.to_chain, to_chain);
    assert_eq!(transaction.to_address, to_address);
    assert_eq!(transaction.amount, DEPOSIT - BRIDGE_FEE);
}

#[test]
fn test_lock_when_paused_fails_and_fee_is_kept_away() {
    let f = setup();
    f.client.pause_bridge();

    let to_chain = String::from_str(&f.env, "ETH");
    let to_address = String::from_str(&f.env, "0xabc123");
    let res = f
        .client
        .try_lock_asset(&f.user, &f.asset, &DEPOSIT, &to_chain, &to_address);

    assert_eq!(res, Err(Ok(Error::BridgePaused)));
    // No fee was taken and nothing was locked.
    assert_eq!(asset_client(&f).balance(&f.fee_address), 0);
    assert_eq!(asset_client(&f).balance(&f.contract_id), 0);
}

#[test]
fn test_lock_after_unpause_works_again() {
    let f = setup();
    f.client.pause_bridge();
    f.client.unpause_bridge();

    let tx_id = lock(&f, DEPOSIT);
    assert_eq!(tx_id, 0);
    assert_eq!(asset_client(&f).balance(&f.fee_address), BRIDGE_FEE);
}

/* ------------------------------------------------------------------ */
/* Attestation flow                                                    */
/* ------------------------------------------------------------------ */

#[test]
fn test_attest_below_threshold_keeps_pending() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.attest_transaction(&f.v1, &tx_id);

    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Pending);
    assert_eq!(transaction.attestations.len(), 1);
    assert_eq!(wrapped_client(&f).balance(&f.user), 0);
}

#[test]
fn test_full_attestation_flow_with_multiple_validators() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    // First attestation: still pending.
    f.client.attest_transaction(&f.v1, &tx_id);
    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Pending);

    // Second attestation: required_attestations reached.
    f.client.attest_transaction(&f.v2, &tx_id);
    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Completed);
    assert_eq!(transaction.attestations.len(), 2);
    // The wrapped asset is minted to the user with the post-fee amount.
    assert_eq!(wrapped_client(&f).balance(&f.user), DEPOSIT - BRIDGE_FEE);
    assert_eq!(
        asset_client(&f).balance(&f.contract_id),
        DEPOSIT - BRIDGE_FEE
    );
}

#[test]
fn test_attest_past_threshold_rejected_and_mints_only_once() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.attest_transaction(&f.v1, &tx_id);
    f.client.attest_transaction(&f.v2, &tx_id);

    // v3's attestation must be refused and must not re-mint.
    let res = f.client.try_attest_transaction(&f.v3, &tx_id);
    assert_eq!(res, Err(Ok(Error::InvalidTransactionState)));
    assert_eq!(wrapped_client(&f).balance(&f.user), DEPOSIT - BRIDGE_FEE);
}

#[test]
fn test_attest_by_non_validator_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    let stranger = Address::generate(&f.env);
    let res = f.client.try_attest_transaction(&stranger, &tx_id);
    assert_eq!(res, Err(Ok(Error::NotActiveValidator)));
}

#[test]
fn test_duplicate_attestation_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.attest_transaction(&f.v1, &tx_id);

    let res = f.client.try_attest_transaction(&f.v1, &tx_id);
    assert_eq!(res, Err(Ok(Error::DuplicateAttestation)));
}

/* ------------------------------------------------------------------ */
/* Validator management                                                 */
/* ------------------------------------------------------------------ */

#[test]
fn test_add_validator_then_attest_with_it() {
    let f = setup();

    let v4 = Address::generate(&f.env);
    f.client.add_validator(&v4);

    let state: BridgeState = f.env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 4);

    // The new validator can participate in attestations.
    let tx_id = lock(&f, DEPOSIT);
    f.client.attest_transaction(&f.v1, &tx_id);
    f.client.attest_transaction(&v4, &tx_id);
    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Completed);
}

#[test]
fn test_add_duplicate_validator_fails() {
    let f = setup();

    let res = f.client.try_add_validator(&f.v1);
    assert_eq!(res, Err(Ok(Error::ValidatorAlreadyExists)));

    let state: BridgeState = f.env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 3);
}

#[test]
fn test_remove_validator_while_transaction_pending() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    // Removing a validator whose transaction is still pending.
    f.client.remove_validator(&f.v1);

    let state: BridgeState = f.env.storage().instance().get(&DataKey::State).unwrap();
    assert_eq!(state.validator_count, 2);

    // The removed validator can no longer attest.
    let res = f.client.try_attest_transaction(&f.v1, &tx_id);
    assert_eq!(res, Err(Ok(Error::NotActiveValidator)));

    // The pending transaction can still complete with the remaining validators.
    f.client.attest_transaction(&f.v2, &tx_id);
    f.client.attest_transaction(&f.v3, &tx_id);
    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Completed);
    assert_eq!(wrapped_client(&f).balance(&f.user), DEPOSIT - BRIDGE_FEE);
}

#[test]
fn test_remove_added_validator_twice_fails() {
    let f = setup();

    let v4 = Address::generate(&f.env);
    f.client.add_validator(&v4);
    f.client.remove_validator(&v4);

    let res = f.client.try_remove_validator(&v4);
    assert_eq!(res, Err(Ok(Error::ValidatorNotFound)));
}

#[test]
fn test_remove_nonexistent_validator_fails() {
    let f = setup();

    let stranger = Address::generate(&f.env);
    let res = f.client.try_remove_validator(&stranger);
    assert_eq!(res, Err(Ok(Error::ValidatorNotFound)));
}

/* ------------------------------------------------------------------ */
/* emergency_withdraw                                                   */
/* ------------------------------------------------------------------ */

#[test]
fn test_emergency_withdraw_refunds_pending_transaction() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.emergency_withdraw(&f.user, &tx_id);

    // The user recovers the locked amount (post-fee), not the fee.
    assert_eq!(asset_client(&f).balance(&f.user), 10_000 - BRIDGE_FEE);
    assert_eq!(asset_client(&f).balance(&f.contract_id), 0);
    assert_eq!(asset_client(&f).balance(&f.fee_address), BRIDGE_FEE);

    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Cancelled);
}

#[test]
fn test_emergency_withdraw_after_pause() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    // Bridge paused after funds were locked; the user can still recover them.
    f.client.pause_bridge();

    f.client.emergency_withdraw(&f.user, &tx_id);

    let transaction: CrossChainTransaction = f
        .env
        .storage()
        .instance()
        .get(&DataKey::Transaction(tx_id))
        .unwrap();

    assert_eq!(transaction.status, TransactionStatus::Cancelled);
    assert_eq!(asset_client(&f).balance(&f.user), 10_000 - BRIDGE_FEE);
}

#[test]
fn test_emergency_withdraw_not_owner_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    let stranger = Address::generate(&f.env);
    let res = f.client.try_emergency_withdraw(&stranger, &tx_id);
    assert_eq!(res, Err(Ok(Error::NotTransactionOwner)));
}

#[test]
fn test_emergency_withdraw_after_completion_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.attest_transaction(&f.v1, &tx_id);
    f.client.attest_transaction(&f.v2, &tx_id);

    let res = f.client.try_emergency_withdraw(&f.user, &tx_id);
    assert_eq!(res, Err(Ok(Error::InvalidTransactionState)));
    assert_eq!(wrapped_client(&f).balance(&f.user), DEPOSIT - BRIDGE_FEE);
}

#[test]
fn test_emergency_withdraw_twice_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.emergency_withdraw(&f.user, &tx_id);

    let res = f.client.try_emergency_withdraw(&f.user, &tx_id);
    assert_eq!(res, Err(Ok(Error::InvalidTransactionState)));
}

#[test]
fn test_attest_after_emergency_withdraw_fails() {
    let f = setup();
    let tx_id = lock(&f, DEPOSIT);

    f.client.emergency_withdraw(&f.user, &tx_id);

    let res = f.client.try_attest_transaction(&f.v1, &tx_id);
    assert_eq!(res, Err(Ok(Error::InvalidTransactionState)));
}
