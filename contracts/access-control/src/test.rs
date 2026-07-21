#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env};

fn setup() -> (Env, AccessControlClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(AccessControl, ());
    let client = AccessControlClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    (env, client, admin)
}

#[test]
fn initialize_is_idempotent() {
    let (_env, client, admin) = setup();
    let res = client.try_initialize(&admin);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn admin_has_admin_role() {
    let (env, client, admin) = setup();
    assert!(client.has_role(&ROLE_ADMIN, &admin));
}

#[test]
fn grant_role() {
    let (env, client, admin) = setup();
    let operator = Address::generate(&env);
    client.grant_role(&ROLE_OPERATOR, &operator);
    assert!(client.has_role(&ROLE_OPERATOR, &operator));
}

#[test]
fn grant_same_role_twice_no_error() {
    let (env, client, admin) = setup();
    let operator = Address::generate(&env);
    client.grant_role(&ROLE_OPERATOR, &operator);
    client.grant_role(&ROLE_OPERATOR, &operator);
    assert!(client.has_role(&ROLE_OPERATOR, &operator));
}

#[test]
fn revoke_role() {
    let (env, client, admin) = setup();
    let pauser = Address::generate(&env);
    client.grant_role(&ROLE_PAUSER, &pauser);
    assert!(client.has_role(&ROLE_PAUSER, &pauser));
    client.revoke_role(&ROLE_PAUSER, &pauser);
    assert!(!client.has_role(&ROLE_PAUSER, &pauser));
}

#[test]
fn revoke_nonexistent_role_no_error() {
    let (env, client, admin) = setup();
    let random = Address::generate(&env);
    client.revoke_role(&ROLE_PAUSER, &random);
    assert!(!client.has_role(&ROLE_PAUSER, &random));
}

#[test]
fn renounce_role() {
    let (env, client, admin) = setup();
    let governor = Address::generate(&env);
    client.grant_role(&ROLE_GOVERNOR, &governor);
    assert!(client.has_role(&ROLE_GOVERNOR, &governor));
    client.renounce_role(&ROLE_GOVERNOR, &governor);
    assert!(!client.has_role(&ROLE_GOVERNOR, &governor));
}

#[test]
fn renounce_nonexistent_role_no_error() {
    let (env, client, admin) = setup();
    let random = Address::generate(&env);
    client.renounce_role(&ROLE_GOVERNOR, &random);
    assert!(!client.has_role(&ROLE_GOVERNOR, &random));
}

#[test]
fn unauthorized_grant_fails() {
    let (env, client, _admin) = setup();
    let random = Address::generate(&env);
    let operator = Address::generate(&env);
    let res = client.try_grant_role(&ROLE_OPERATOR, &operator);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn unauthorized_revoke_fails() {
    let (env, client, admin) = setup();
    let operator = Address::generate(&env);
    client.grant_role(&ROLE_OPERATOR, &operator);
    let random = Address::generate(&env);
    let res = client.try_revoke_role(&ROLE_OPERATOR, &operator);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn events_are_emitted_on_grant() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    
    client.grant_role(&ROLE_OPERATOR, &account);
    let events = env.events().all();
    assert_eq!(events.len(), 2);
}

#[test]
fn events_are_emitted_on_revoke() {
    let (env, client, admin) = setup();
    let account = Address::generate(&env);
    
    client.grant_role(&ROLE_OPERATOR, &account);
    client.revoke_role(&ROLE_OPERATOR, &account);
    
    let events = env.events().all();
    assert_eq!(events.len(), 3);
}

#[test]
fn get_and_set_role_admin() {
    let (env, client, admin) = setup();
    assert_eq!(client.get_role_admin(&ROLE_OPERATOR), ROLE_ADMIN);
    
    let new_admin_role = symbol_short!("NEW_ADMIN");
    client.set_role_admin(&ROLE_OPERATOR, &new_admin_role);
    assert_eq!(client.get_role_admin(&ROLE_OPERATOR), new_admin_role);
}

#[test]
fn non_admin_cannot_set_role_admin() {
    let (env, client, admin) = setup();
    let random = Address::generate(&env);
    let new_admin_role = symbol_short!("NEW_ADMIN");
    let res = client.try_set_role_admin(&ROLE_OPERATOR, &new_admin_role);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn transfer_admin_role() {
    let (env, client, admin) = setup();
    let new_admin = Address::generate(&env);
    client.grant_role(&ROLE_ADMIN, &new_admin);
    assert!(client.has_role(&ROLE_ADMIN, &new_admin));
    
    let new_admin_role = symbol_short!("NEW_ADMIN");
    client.set_role_admin(&ROLE_OPERATOR, &new_admin_role);
    assert_eq!(client.get_role_admin(&ROLE_OPERATOR), new_admin_role);
}
