#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, vec, Address, Env,
};

/// Deploy a Stellar Asset Contract and return (token client, admin/mint client).
fn create_token<'a>(
    env: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    let addr = sac.address();
    (
        token::Client::new(env, &addr),
        token::StellarAssetClient::new(env, &addr),
    )
}

fn setup(env: &Env) -> (FeeCommissionClient<'_>, Address) {
    let contract_id = env.register_contract(None, FeeCommission);
    let client = FeeCommissionClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let treasury = Address::generate(env);
    // 10 bps maker, 20 bps taker, 30% protocol cut.
    client.initialize(&admin, &10, &20, &3_000, &treasury);
    (client, admin)
}

#[test]
fn test_initialize_and_get_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let config = client.get_config();
    assert_eq!(config.maker_fee_bps, 10);
    assert_eq!(config.taker_fee_bps, 20);
    assert_eq!(config.protocol_fee_bps, 3_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #1)")]
fn test_initialize_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    client.initialize(&admin, &10, &20, &3_000, &treasury);
}

#[test]
#[should_panic(expected = "Error(Contract, #3)")]
fn test_initialize_rejects_excessive_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeCommission);
    let client = FeeCommissionClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    // 2000 bps (20%) exceeds MAX_FEE_BPS (1000).
    client.initialize(&admin, &2_000, &20, &3_000, &treasury);
}

#[test]
fn test_calculate_fees_maker_and_taker() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let payer = Address::generate(&env);

    // 10 bps of 1_000_000 = 1000.
    assert_eq!(client.calculate_fees(&payer, &1_000_000, &Side::Maker), 1_000);
    // 20 bps of 1_000_000 = 2000.
    assert_eq!(client.calculate_fees(&payer, &1_000_000, &Side::Taker), 2_000);
}

#[test]
fn test_calculate_fees_exempt_is_zero() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let payer = Address::generate(&env);

    client.set_exempt(&payer, &true);
    assert_eq!(client.calculate_fees(&payer, &1_000_000, &Side::Taker), 0);
    assert!(client.is_exempt(&payer));

    client.set_exempt(&payer, &false);
    assert_eq!(client.calculate_fees(&payer, &1_000_000, &Side::Taker), 2_000);
    assert!(!client.is_exempt(&payer));
}

#[test]
fn test_collect_fee_moves_tokens_into_pool() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let token_admin = Address::generate(&env);
    let (tok, mint) = create_token(&env, &token_admin);
    let payer = Address::generate(&env);
    mint.mint(&payer, &1_000_000);

    // Taker fee: 20 bps of 500_000 = 1000.
    let fee = client.collect_fee(&payer, &tok.address, &500_000, &Side::Taker);
    assert_eq!(fee, 1_000);
    assert_eq!(client.get_fee_pool(&tok.address), 1_000);
    assert_eq!(tok.balance(&payer), 999_000);
    assert_eq!(tok.balance(&client.address), 1_000);
}

#[test]
fn test_collect_fee_exempt_no_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let token_admin = Address::generate(&env);
    let (tok, mint) = create_token(&env, &token_admin);
    let payer = Address::generate(&env);
    mint.mint(&payer, &1_000_000);
    client.set_exempt(&payer, &true);

    let fee = client.collect_fee(&payer, &tok.address, &500_000, &Side::Taker);
    assert_eq!(fee, 0);
    assert_eq!(client.get_fee_pool(&tok.address), 0);
    assert_eq!(tok.balance(&payer), 1_000_000);
}

#[test]
fn test_distribute_fee_splits_treasury_and_recipients() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeCommission);
    let client = FeeCommissionClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    // 30% protocol cut so the maths are round.
    client.initialize(&admin, &100, &100, &3_000, &treasury);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    // Remainder split 60/40.
    client.set_recipients(&vec![
        &env,
        FeeRecipient {
            address: r1.clone(),
            share_bps: 6_000,
        },
        FeeRecipient {
            address: r2.clone(),
            share_bps: 4_000,
        },
    ]);

    let token_admin = Address::generate(&env);
    let (tok, mint) = create_token(&env, &token_admin);
    let payer = Address::generate(&env);
    mint.mint(&payer, &1_000_000);

    // 100 bps of 1_000_000 = 10_000 collected.
    client.collect_fee(&payer, &tok.address, &1_000_000, &Side::Taker);
    assert_eq!(client.get_fee_pool(&tok.address), 10_000);

    let distributed = client.distribute_fee(&tok.address);
    assert_eq!(distributed, 10_000);
    assert_eq!(client.get_fee_pool(&tok.address), 0);

    // Protocol: 30% of 10_000 = 3_000. Remainder 7_000: r1=4_200, r2=2_800.
    assert_eq!(tok.balance(&treasury), 3_000);
    assert_eq!(tok.balance(&r1), 4_200);
    assert_eq!(tok.balance(&r2), 2_800);
    assert_eq!(tok.balance(&client.address), 0);
}

#[test]
fn test_distribute_fee_dust_goes_to_treasury() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, FeeCommission);
    let client = FeeCommissionClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    // 0% protocol cut; entire pool splits among recipients (plus dust to treasury).
    client.initialize(&admin, &100, &100, &0, &treasury);

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    // Three-way even split forces rounding on a non-divisible pool.
    client.set_recipients(&vec![
        &env,
        FeeRecipient {
            address: r1.clone(),
            share_bps: 3_333,
        },
        FeeRecipient {
            address: r2.clone(),
            share_bps: 3_333,
        },
        FeeRecipient {
            address: r3.clone(),
            share_bps: 3_334,
        },
    ]);

    let token_admin = Address::generate(&env);
    let (tok, mint) = create_token(&env, &token_admin);
    let payer = Address::generate(&env);
    mint.mint(&payer, &1_000_000);

    // Collect a pool of 100.
    client.collect_fee(&payer, &tok.address, &10_000, &Side::Taker);
    assert_eq!(client.get_fee_pool(&tok.address), 100);

    client.distribute_fee(&tok.address);

    // r1: 100*3333/10000 = 33, r2: 33, r3: 100*3334/10000 = 33. Sum = 99.
    // Dust of 1 sweeps to treasury.
    assert_eq!(tok.balance(&r1), 33);
    assert_eq!(tok.balance(&r2), 33);
    assert_eq!(tok.balance(&r3), 33);
    assert_eq!(tok.balance(&treasury), 1);
    assert_eq!(tok.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_distribute_empty_pool_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let r1 = Address::generate(&env);
    client.set_recipients(&vec![
        &env,
        FeeRecipient {
            address: r1,
            share_bps: 10_000,
        },
    ]);
    let asset = Address::generate(&env);
    client.distribute_fee(&asset);
}

#[test]
#[should_panic(expected = "Error(Contract, #9)")]
fn test_distribute_without_recipients_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let asset = Address::generate(&env);
    client.distribute_fee(&asset);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_set_recipients_bad_shares_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let r1 = Address::generate(&env);
    // Shares sum to 5000, not 10000.
    client.set_recipients(&vec![
        &env,
        FeeRecipient {
            address: r1,
            share_bps: 5_000,
        },
    ]);
}

#[test]
fn test_set_fees_updates_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let treasury = Address::generate(&env);

    client.set_fees(&50, &75, &2_500, &treasury);
    let config = client.get_config();
    assert_eq!(config.maker_fee_bps, 50);
    assert_eq!(config.taker_fee_bps, 75);
    assert_eq!(config.protocol_fee_bps, 2_500);
    assert_eq!(config.protocol_treasury, treasury);
}

#[test]
fn test_scheduled_update_activates_after_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let treasury = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.schedule_fee_update(&40, &60, &1_000, &treasury, &2_000);

    let pending = client.get_pending().unwrap();
    assert_eq!(pending.activate_at, 2_000);
    // Config not yet changed.
    assert_eq!(client.get_config().maker_fee_bps, 10);

    // Advance past activation and apply.
    env.ledger().set_timestamp(2_500);
    client.apply_scheduled_update();

    assert_eq!(client.get_config().maker_fee_bps, 40);
    assert_eq!(client.get_config().taker_fee_bps, 60);
    assert!(client.get_pending().is_none());
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_scheduled_update_before_activation_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let treasury = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.schedule_fee_update(&40, &60, &1_000, &treasury, &5_000);

    // Still before activation.
    env.ledger().set_timestamp(2_000);
    client.apply_scheduled_update();
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_apply_without_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    client.apply_scheduled_update();
}
