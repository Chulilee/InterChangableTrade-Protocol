#![cfg(test)]

use super::*;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events, Ledger},
    Address, Env,
};

fn setup() -> (Env, GovernanceClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(Governance, ());
    let client = GovernanceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn full_lifecycle_create_vote_queue_execute() {
    let (env, client, _admin) = setup();

    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let caller = Address::generate(&env);

    let proposal_id = client.propose(
        &proposer,
        &symbol_short!("fee_chng"),
        &symbol_short!("fee"),
        &100,
        &10,
        &20,
        &5,
        &100,
        &150,
    );
    // `env.events().all()` returns only the most recent invocation's events,
    // so accumulate the count after each emitting call across the lifecycle.
    let mut total_events = env.events().all().len();

    env.ledger().set_timestamp(15);
    client.cast_vote(&proposal_id, &voter, &true, &200);
    total_events += env.events().all().len();
    env.ledger().set_timestamp(25);
    client.finalize(&proposal_id);

    let state = client.get_state(&proposal_id);
    assert_eq!(state, ProposalState::Succeeded);

    client.queue_execution(&proposal_id, &caller);
    total_events += env.events().all().len();
    env.ledger().set_timestamp(35);
    client.execute_proposal(&proposal_id, &caller);
    total_events += env.events().all().len();

    let metadata = client.get_proposal(&proposal_id);
    assert_eq!(metadata.value, 100);
    let executed_state = client.get_state(&proposal_id);
    assert_eq!(executed_state, ProposalState::Executed);
    assert_eq!(client.get_parameter(&symbol_short!("fee")), Some(100));
    assert!(total_events >= 4);
}

#[test]
fn cancellation_path_marks_proposal_cancelled() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);

    let proposal_id = client.propose(
        &proposer,
        &symbol_short!("cancel"),
        &symbol_short!("limit"),
        &50,
        &1,
        &5,
        &1,
        &50,
        &80,
    );
    client.cancel_proposal(&proposal_id, &proposer);
    let metadata = client.get_proposal(&proposal_id);
    assert!(metadata.canceled);
}

#[test]
fn timelock_prevents_execution_before_delay() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let caller = Address::generate(&env);

    let proposal_id = client.propose(
        &proposer,
        &symbol_short!("delay"),
        &symbol_short!("oracle"),
        &10,
        &1,
        &2,
        &2,
        &10,
        &20,
    );
    env.ledger().set_timestamp(1);
    client.cast_vote(&proposal_id, &voter, &true, &50);
    env.ledger().set_timestamp(3);
    client.finalize(&proposal_id);
    client.queue_execution(&proposal_id, &caller);

    let res = client.try_execute_proposal(&proposal_id, &caller);
    assert_eq!(res, Err(Ok(Error::TimelockNotElapsed)));
}

#[test]
fn invalid_quorum_and_threshold_rejected() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);
    let res = client.try_propose(
        &proposer,
        &symbol_short!("bad"),
        &symbol_short!("param"),
        &10,
        &1,
        &2,
        &1,
        &0,
        &10,
    );
    assert_eq!(res, Err(Ok(Error::InvalidQuorum)));

    let res = client.try_propose(
        &proposer,
        &symbol_short!("bad"),
        &symbol_short!("param"),
        &10,
        &1,
        &2,
        &1,
        &10,
        &0,
    );
    assert_eq!(res, Err(Ok(Error::InvalidThreshold)));
}

#[test]
fn cast_vote_preserves_order_when_updating() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);

    let proposal_id = client.propose(
        &proposer,
        &symbol_short!("order"),
        &symbol_short!("test"),
        &100,
        &10,
        &50,
        &5,
        &100,
        &150,
    );
    env.ledger().set_timestamp(20);

    // Cast votes in order: voter1, voter2, voter3
    client.cast_vote(&proposal_id, &voter1, &true, &100);
    client.cast_vote(&proposal_id, &voter2, &false, &200);
    client.cast_vote(&proposal_id, &voter3, &true, &150);

    // Get initial vote order
    let votes_before = client.get_votes(&proposal_id);
    let mut order_before: Vec<Address> = Vec::new(&env);
    for v in votes_before.iter() {
        order_before.push_back(v.voter.clone());
    }

    // Update voter2's vote (should stay in the same position)
    client.cast_vote(&proposal_id, &voter2, &true, &250);

    // Get vote order after update
    let votes_after = client.get_votes(&proposal_id);
    let mut order_after: Vec<Address> = Vec::new(&env);
    for v in votes_after.iter() {
        order_after.push_back(v.voter.clone());
    }

    // Vote order should be preserved: voter1, voter2, voter3
    assert_eq!(order_before, order_after);
    assert_eq!(order_after.len(), 3);
    assert_eq!(order_after.get(0).unwrap(), voter1.clone());
    assert_eq!(order_after.get(1).unwrap(), voter2.clone());
    assert_eq!(order_after.get(2).unwrap(), voter3.clone());

    // Verify voter2's updated vote
    let voter2_vote = votes_after.iter().find(|v| v.voter == voter2).unwrap();
    assert!(voter2_vote.support);
    assert_eq!(voter2_vote.weight, 250);
}

#[test]
fn cast_vote_updates_count_correctly() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    let proposal_id = client.propose(
        &proposer,
        &symbol_short!("count"),
        &symbol_short!("test"),
        &100,
        &10,
        &50,
        &5,
        &100,
        &150,
    );
    env.ledger().set_timestamp(20);

    // Initial vote
    client.cast_vote(&proposal_id, &voter, &true, &100);
    let votes = client.get_votes(&proposal_id);
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 100);

    // Update vote (should not change count)
    client.cast_vote(&proposal_id, &voter, &false, &200);
    let votes = client.get_votes(&proposal_id);
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 200);
    assert!(!votes.get(0).unwrap().support);

    // Update vote with same values (should not change anything)
    client.cast_vote(&proposal_id, &voter, &false, &200);
    let votes = client.get_votes(&proposal_id);
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 200);
}

#[test]
fn propose_rejects_invalid_time_range() {
    let (env, client, _admin) = setup();
    let proposer = Address::generate(&env);

    // end_time must be strictly greater than start_time.
    let equal = client.try_propose(
        &proposer,
        &symbol_short!("time"),
        &symbol_short!("param"),
        &10,
        &10,
        &10,
        &1,
        &10,
        &10,
    );
    assert_eq!(equal, Err(Ok(Error::InvalidTimeRange)));

    let inverted = client.try_propose(
        &proposer,
        &symbol_short!("time"),
        &symbol_short!("param"),
        &10,
        &20,
        &10,
        &1,
        &10,
        &10,
    );
    assert_eq!(inverted, Err(Ok(Error::InvalidTimeRange)));
}
