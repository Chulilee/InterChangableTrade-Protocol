#![cfg(test)]
use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env,
};

/// Owned handles for a configured module + pool. Clients borrow `Env`, so we
/// store addresses and build clients on demand to avoid a self-referential
/// struct.
struct Fixture {
    env: Env,
    contract: Address,
    admin: Address,
    stake: Address,
    reward: Address,
    pool_id: u64,
}

impl Fixture {
    fn client(&self) -> LiquidityIncentivesClient<'_> {
        LiquidityIncentivesClient::new(&self.env, &self.contract)
    }
    fn stake_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.stake)
    }
    fn reward_token(&self) -> token::Client<'_> {
        token::Client::new(&self.env, &self.reward)
    }
    fn stake_mint(&self) -> token::StellarAssetClient<'_> {
        token::StellarAssetClient::new(&self.env, &self.stake)
    }
    /// Mint `amount` staking tokens to a fresh provider and return the address.
    fn provider(&self, amount: i128) -> Address {
        let p = Address::generate(&self.env);
        self.stake_mint().mint(&p, &amount);
        p
    }
}

/// Set the ledger clock to an absolute timestamp.
fn set_time(env: &Env, t: u64) {
    env.ledger().with_mut(|l| l.timestamp = t);
}

/// Build a module with one pool emitting `rate` reward tokens/sec for `duration`
/// seconds, funded with `fund` reward tokens. Ledger starts at t=1_000.
fn setup(rate: i128, duration: u64, fund: i128) -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, 1_000);

    let contract = env.register_contract(None, LiquidityIncentives);
    let client = LiquidityIncentivesClient::new(&env, &contract);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    let token_admin = Address::generate(&env);
    let stake = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let reward = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let pool_id = client.create_pool(&stake, &reward, &rate, &duration);

    // Fund the reward reserve from the admin.
    token::StellarAssetClient::new(&env, &reward).mint(&admin, &fund);
    client.fund_pool(&pool_id, &fund);

    Fixture {
        env,
        contract,
        admin,
        stake,
        reward,
        pool_id,
    }
}

#[test]
fn test_single_provider_accrues_over_time() {
    // 100 reward tokens/sec.
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);

    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    // Advance 100 seconds: sole provider earns the full emission = 100 * 100 = 10_000.
    set_time(&fx.env, 1_100);
    assert_eq!(client.view_accrued_rewards(&pos), 10_000);

    // Claim transfers exactly that and resets the counter.
    let claimed = client.claim_rewards(&alice, &pos);
    assert_eq!(claimed, 10_000);
    assert_eq!(fx.reward_token().balance(&alice), 10_000);
    assert_eq!(client.view_accrued_rewards(&pos), 0);
}

#[test]
fn test_deposit_pulls_and_withdraw_returns_staking_tokens() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);

    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);
    assert_eq!(fx.stake_token().balance(&alice), 0);
    assert_eq!(fx.stake_token().balance(&fx.contract), 1_000);

    client.withdraw_liquidity(&alice, &pos, &400);
    assert_eq!(fx.stake_token().balance(&alice), 400);
    assert_eq!(fx.stake_token().balance(&fx.contract), 600);

    let position = client.get_position(&pos);
    assert_eq!(position.liquidity, 600);
}

#[test]
fn test_two_providers_split_pro_rata() {
    // 90 reward tokens/sec, split by liquidity share.
    let fx = setup(90, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let bob = fx.provider(2_000);

    // Alice deposits 1000, Bob deposits 2000 at the same instant: 1/3 vs 2/3.
    let pa = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);
    let pb = client.deposit_liquidity(&bob, &fx.pool_id, &2_000, &-10, &10);

    // 100 seconds → 9_000 emitted total. Alice: 3_000, Bob: 6_000.
    set_time(&fx.env, 1_100);
    assert_eq!(client.view_accrued_rewards(&pa), 3_000);
    assert_eq!(client.view_accrued_rewards(&pb), 6_000);
}

#[test]
fn test_rapid_deposit_withdraw_prorates_exactly() {
    // 100 reward tokens/sec.
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let bob = fx.provider(1_000);

    // Phase 1 (t=1000..1050): only Alice staked (1000). She earns all 50*100=5_000.
    let pa = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    set_time(&fx.env, 1_050);
    // Bob joins with equal liquidity.
    let pb = client.deposit_liquidity(&bob, &fx.pool_id, &1_000, &-10, &10);

    // Phase 2 (t=1050..1100): Alice & Bob split 50*100=5_000 evenly → 2_500 each.
    set_time(&fx.env, 1_100);
    // Bob withdraws everything right at t=1100. His accrual freezes at 2_500.
    client.withdraw_liquidity(&bob, &pb, &1_000);

    // Phase 3 (t=1100..1150): only Alice staked again. She earns 50*100=5_000.
    set_time(&fx.env, 1_150);

    // Alice: 5_000 + 2_500 + 5_000 = 12_500. Bob: 2_500 flat (withdrawn).
    assert_eq!(client.view_accrued_rewards(&pa), 12_500);
    assert_eq!(client.view_accrued_rewards(&pb), 2_500);

    // Bob can still claim his frozen rewards after full withdrawal.
    let bob_claim = client.claim_rewards(&bob, &pb);
    assert_eq!(bob_claim, 2_500);
    assert_eq!(fx.reward_token().balance(&bob), 2_500);
}

#[test]
fn test_no_rewards_accrue_while_pool_empty() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);

    // 100 seconds pass with no liquidity staked — those emissions are not owed
    // to anyone.
    set_time(&fx.env, 1_100);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    // Another 100 seconds with Alice as sole provider → 10_000.
    set_time(&fx.env, 1_200);
    assert_eq!(client.view_accrued_rewards(&pos), 10_000);
}

#[test]
fn test_rewards_stop_at_period_finish() {
    // Rate 100/sec for only 50 seconds.
    let fx = setup(100, 50, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    // Run well past period_finish (t=1050). Emission caps at 50*100 = 5_000.
    set_time(&fx.env, 5_000);
    assert_eq!(client.view_accrued_rewards(&pos), 5_000);
}

#[test]
fn test_claim_resets_and_second_claim_fails() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    set_time(&fx.env, 1_100);
    client.claim_rewards(&alice, &pos);
    assert_eq!(fx.reward_token().balance(&alice), 10_000);

    // No time has passed since the claim → nothing to claim.
    let res = client.try_claim_rewards(&alice, &pos);
    assert_eq!(res, Err(Ok(Error::NothingToClaim)));
}

#[test]
fn test_set_reward_rate_settles_before_change() {
    let fx = setup(100, 10_000, 2_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);

    // 100s at rate 100 → 10_000 accrued so far.
    set_time(&fx.env, 1_100);

    // Governance doubles the rate. Past accrual must be preserved.
    client.set_reward_rate(&fx.pool_id, &200, &10_000);
    assert_eq!(client.view_accrued_rewards(&pos), 10_000);

    // 100s more at rate 200 → +20_000 = 30_000.
    set_time(&fx.env, 1_200);
    assert_eq!(client.view_accrued_rewards(&pos), 30_000);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_withdraw_more_than_liquidity_fails() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);
    client.withdraw_liquidity(&alice, &pos, &1_500);
}

#[test]
#[should_panic(expected = "Error(Contract, #7)")]
fn test_claim_by_non_owner_fails() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let pos = client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);
    set_time(&fx.env, 1_100);

    let mallory = Address::generate(&fx.env);
    client.claim_rewards(&mallory, &pos);
}

#[test]
#[should_panic(expected = "Error(Contract, #5)")]
fn test_deposit_zero_fails() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    client.deposit_liquidity(&alice, &fx.pool_id, &0, &-10, &10);
}

#[test]
#[should_panic(expected = "Error(Contract, #8)")]
fn test_deposit_bad_tick_range_fails() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    // tick_lower >= tick_upper is invalid.
    client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &10, &10);
}

#[test]
fn test_get_pool_tracks_total_liquidity() {
    let fx = setup(100, 10_000, 1_000_000);
    let client = fx.client();
    let alice = fx.provider(1_000);
    let bob = fx.provider(2_000);

    client.deposit_liquidity(&alice, &fx.pool_id, &1_000, &-10, &10);
    client.deposit_liquidity(&bob, &fx.pool_id, &2_000, &-20, &20);
    assert_eq!(client.get_pool(&fx.pool_id).total_liquidity, 3_000);
}

#[test]
fn test_admin_field_used_in_setup() {
    // The admin funds the pool during setup; assert the reserve landed in the
    // contract so `fund_pool` auth path is exercised end to end.
    let fx = setup(100, 10_000, 777_000);
    assert_eq!(fx.reward_token().balance(&fx.contract), 777_000);
    // `admin` is a distinct address from the contract.
    assert_ne!(fx.admin, fx.contract);
}
