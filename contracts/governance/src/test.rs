#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

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
        &symbol_short!("fee_change"),
        &symbol_short!("fee"),
        &100,
        &10,
        &20,
        &5,
        &100,
        &150,
    );

    client.cast_vote(&proposal_id, &voter, &true, &200);
    env.ledger().set_timestamp(25);
    client.finalize(&proposal_id);

    let state = client.get_state(&proposal_id);
    assert_eq!(state, ProposalState::Succeeded);

    client.queue_execution(&proposal_id, &caller);
    env.ledger().set_timestamp(35);
    client.execute_proposal(&proposal_id, &caller);

    let metadata = client.get_proposal(&proposal_id);
    assert_eq!(metadata.value, 100);
    let executed_state = client.get_state(&proposal_id);
    assert_eq!(executed_state, ProposalState::Executed);
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
