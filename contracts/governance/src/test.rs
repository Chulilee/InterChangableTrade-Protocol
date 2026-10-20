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
    client.initialize(&admin).unwrap();
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
    ).unwrap();
    // `env.events().all()` returns only the most recent invocation's events,
    // so accumulate the count after each emitting call across the lifecycle.
    let mut total_events = env.events().all().len();

    env.ledger().set_timestamp(15);
    client.cast_vote(&proposal_id, &voter, &true, &200).unwrap();
    total_events += env.events().all().len();
    env.ledger().set_timestamp(25);
    client.finalize(&proposal_id).unwrap();

    let state = client.get_state(&proposal_id).unwrap();
    assert_eq!(state, ProposalState::Succeeded);

    client.queue_execution(&proposal_id, &caller).unwrap();
    total_events += env.events().all().len();
    env.ledger().set_timestamp(35);
    client.execute_proposal(&proposal_id, &caller).unwrap();
    total_events += env.events().all().len();

    let metadata = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(metadata.value, 100);
    let executed_state = client.get_state(&proposal_id).unwrap();
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
    ).unwrap();

    client.cancel_proposal(&proposal_id, &proposer).unwrap();
    let metadata = client.get_proposal(&proposal_id).unwrap();
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
        &2,
        &10,
        &20,
    ).unwrap();

    env.ledger().set_timestamp(1);
    client.cast_vote(&proposal_id, &voter, &true, &50).unwrap();
    env.ledger().set_timestamp(3);
    client.finalize(&proposal_id).unwrap();
    client.queue_execution(&proposal_id, &caller).unwrap();

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
    ).unwrap();

    env.ledger().set_timestamp(20);

    // Cast votes in order: voter1, voter2, voter3
    client.cast_vote(&proposal_id, &voter1, &true, &100).unwrap();
    client.cast_vote(&proposal_id, &voter2, &false, &200).unwrap();
    client.cast_vote(&proposal_id, &voter3, &true, &150).unwrap();

    // Get initial vote order
    let votes_before = client.get_votes(&proposal_id).unwrap();
    let order_before: Vec<Address> = votes_before.iter().map(|v| v.voter).collect();

    // Update voter2's vote (should stay in the same position)
    client.cast_vote(&proposal_id, &voter2, &true, &250).unwrap();

    // Get vote order after update
    let votes_after = client.get_votes(&proposal_id).unwrap();
    let order_after: Vec<Address> = votes_after.iter().map(|v| v.voter).collect();

    // Vote order should be preserved: voter1, voter2, voter3
    assert_eq!(order_before, order_after);
    assert_eq!(order_after.len(), 3);
    assert_eq!(order_after.get(0).unwrap(), &voter1);
    assert_eq!(order_after.get(1).unwrap(), &voter2);
    assert_eq!(order_after.get(2).unwrap(), &voter3);

    // Verify voter2's updated vote
    let voter2_vote = votes_after.iter().find(|v| v.voter == voter2).unwrap();
    assert_eq!(voter2_vote.support, true);
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
    ).unwrap();

    env.ledger().set_timestamp(20);

    // Initial vote
    client.cast_vote(&proposal_id, &voter, &true, &100).unwrap();
    let votes = client.get_votes(&proposal_id).unwrap();
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 100);

    // Update vote (should not change count)
    client.cast_vote(&proposal_id, &voter, &false, &200).unwrap();
    let votes = client.get_votes(&proposal_id).unwrap();
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 200);
    assert_eq!(votes.get(0).unwrap().support, false);

    // Update vote with same values (should not change anything)
    client.cast_vote(&proposal_id, &voter, &false, &200).unwrap();
    let votes = client.get_votes(&proposal_id).unwrap();
    assert_eq!(votes.len(), 1);
    assert_eq!(votes.get(0).unwrap().weight, 200);
}