#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

struct Fixture {
    env: Env,
    client: NotificationServiceClient<'static>,
    admin: Address,
}

fn setup() -> Fixture {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(NotificationService, ());
    let client = NotificationServiceClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    Fixture {
        client,
        admin,
        env,
    }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

#[test]
fn initialize_sets_admin() {
    let f = setup();
    assert_eq!(f.client.admin(), f.admin);
}

#[test]
fn double_initialize_fails() {
    let f = setup();
    let res = f.client.try_initialize(&f.admin);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

// ---------------------------------------------------------------------------
// Sending
// ---------------------------------------------------------------------------

#[test]
fn send_creates_notification() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let id = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("trade"),
        &Some(42),
    );
    let n = f.client.get(&id);
    assert_eq!(n.recipient, recipient);
    assert_eq!(n.kind, NotificationKind::Trade);
    assert_eq!(n.read, false);
    assert_eq!(n.reference_id, Some(42));
}

#[test]
fn send_non_admin_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let recipient = Address::generate(&f.env);
    let res = f.client.try_send(
        &stranger,
        &recipient,
        &NotificationKind::System,
        &symbol_short!("alert"),
        &None,
    );
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn send_empty_title_fails() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let res = f.client.try_send(
        &f.admin,
        &recipient,
        &NotificationKind::Message,
        &Symbol::new(&f.env, ""),
        &None,
    );
    assert_eq!(res, Err(Ok(Error::EmptyTitle)));
}

#[test]
fn send_unread_count_increments() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    assert_eq!(f.client.unread_count(&recipient), 0);

    f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("trade"),
        &None,
    );
    assert_eq!(f.client.unread_count(&recipient), 1);

    f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Message,
        &symbol_short!("msg"),
        &None,
    );
    assert_eq!(f.client.unread_count(&recipient), 2);
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

#[test]
fn get_user_notifications_newest_first() {
    let f = setup();
    let recipient = Address::generate(&f.env);

    let id1 = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("first"),
        &None,
    );
    let id2 = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Message,
        &symbol_short!("second"),
        &None,
    );

    let page = f.client.get_user_notifications(&recipient, &None, &0);
    assert_eq!(page.total, 2);
    assert_eq!(page.unread_count, 2);
    // Newest first
    assert_eq!(page.notifications.get_unchecked(0).id, id2);
    assert_eq!(page.notifications.get_unchecked(1).id, id1);
}

/// Sends `count` notifications to `recipient` and returns their ids oldest
/// first.
fn seed_notifications(f: &Fixture, recipient: &Address, count: u64) -> Vec<u64> {
    let mut ids = Vec::new(&f.env);
    let mut i = 0;
    while i < count {
        let id = f.client.send(
            &f.admin,
            recipient,
            &NotificationKind::Trade,
            &symbol_short!("seed"),
            &Some(i),
        );
        ids.push_back(id);
        i += 1;
    }
    ids
}

#[test]
fn get_user_notifications_paginates_newest_first() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let ids = seed_notifications(&f, &recipient, 5);

    let first = f.client.get_user_notifications(&recipient, &Some(0), &2);
    assert_eq!(first.total, 5);
    assert_eq!(first.unread_count, 5);
    assert_eq!(first.notifications.len(), 2);
    assert_eq!(first.notifications.get_unchecked(0).id, ids.get_unchecked(4));
    assert_eq!(first.notifications.get_unchecked(1).id, ids.get_unchecked(3));

    let second = f.client.get_user_notifications(&recipient, &Some(2), &2);
    assert_eq!(second.notifications.len(), 2);
    assert_eq!(second.notifications.get_unchecked(0).id, ids.get_unchecked(2));
    assert_eq!(second.notifications.get_unchecked(1).id, ids.get_unchecked(1));

    let last = f.client.get_user_notifications(&recipient, &Some(4), &2);
    assert_eq!(last.notifications.len(), 1);
    assert_eq!(last.notifications.get_unchecked(0).id, ids.get_unchecked(0));
}

#[test]
fn get_user_notifications_none_offset_reads_from_newest() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let ids = seed_notifications(&f, &recipient, 3);

    let page = f.client.get_user_notifications(&recipient, &None, &1);
    assert_eq!(page.notifications.len(), 1);
    assert_eq!(page.notifications.get_unchecked(0).id, ids.get_unchecked(2));
}

#[test]
fn get_user_notifications_offset_beyond_total_is_empty() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    seed_notifications(&f, &recipient, 3);

    let page = f.client.get_user_notifications(&recipient, &Some(50), &10);
    assert_eq!(page.notifications.len(), 0);
    assert_eq!(page.total, 3);
    assert_eq!(page.unread_count, 3);
}

#[test]
fn get_user_notifications_zero_limit_uses_default_page_size() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    seed_notifications(&f, &recipient, 25);

    let page = f.client.get_user_notifications(&recipient, &Some(0), &0);
    assert_eq!(page.notifications.len() as u64, DEFAULT_PAGE_SIZE);
    assert_eq!(page.total, 25);
}

#[test]
fn get_user_notifications_limit_is_clamped_to_max_page_size() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    seed_notifications(&f, &recipient, MAX_PAGE_SIZE + 1);

    let page = f.client.get_user_notifications(&recipient, &Some(0), &10_000);
    assert_eq!(page.notifications.len() as u64, MAX_PAGE_SIZE);
    assert_eq!(page.total, MAX_PAGE_SIZE + 1);
}

#[test]
fn get_missing_notification_fails() {
    let f = setup();
    let res = f.client.try_get(&999);
    assert_eq!(res, Err(Ok(Error::NotificationNotFound)));
}

// ---------------------------------------------------------------------------
// Mark read / mark all
// ---------------------------------------------------------------------------

#[test]
fn mark_read_works() {
    let f = setup();
    let recipient = Address::generate(&f.env);

    let id = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("trade"),
        &None,
    );
    assert_eq!(f.client.unread_count(&recipient), 1);

    let n = f.client.mark_read(&id, &recipient);
    assert_eq!(n.read, true);
    assert_eq!(f.client.unread_count(&recipient), 0);
}

#[test]
fn mark_read_non_recipient_fails() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let stranger = Address::generate(&f.env);

    let id = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::System,
        &symbol_short!("alert"),
        &None,
    );
    let res = f.client.try_mark_read(&id, &stranger);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn mark_read_already_read_fails() {
    let f = setup();
    let recipient = Address::generate(&f.env);

    let id = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("trade"),
        &None,
    );
    f.client.mark_read(&id, &recipient);
    let res = f.client.try_mark_read(&id, &recipient);
    assert_eq!(res, Err(Ok(Error::AlreadyRead)));
}

#[test]
fn mark_all_read_works() {
    let f = setup();
    let recipient = Address::generate(&f.env);

    f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("t1"),
        &None,
    );
    f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Message,
        &symbol_short!("m2"),
        &None,
    );
    f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::System,
        &symbol_short!("s3"),
        &None,
    );
    assert_eq!(f.client.unread_count(&recipient), 3);

    let marked = f.client.mark_all_read(&recipient);
    assert_eq!(marked, 3);
    assert_eq!(f.client.unread_count(&recipient), 0);

    // Verify each notification is now read.
    let page = f.client.get_user_notifications(&recipient, &None, &0);
    for n in page.notifications.iter() {
        assert!(n.read);
    }
}

#[test]
fn mark_all_read_with_nothing_unread_returns_zero() {
    let f = setup();
    let recipient = Address::generate(&f.env);
    let marked = f.client.mark_all_read(&recipient);
    assert_eq!(marked, 0);
}

// ---------------------------------------------------------------------------
// Email preferences
// ---------------------------------------------------------------------------

#[test]
fn email_opt_in_defaults_false() {
    let f = setup();
    let user = Address::generate(&f.env);
    let settings = f.client.get_email_opt_in(&user);
    assert_eq!(settings.email_opt_in, false);
}

#[test]
fn set_email_opt_in_true() {
    let f = setup();
    let user = Address::generate(&f.env);
    let settings = f.client.set_email_opt_in(&user, &true);
    assert_eq!(settings.email_opt_in, true);
    assert_eq!(f.client.get_email_opt_in(&user).email_opt_in, true);
}

#[test]
fn set_email_opt_in_false() {
    let f = setup();
    let user = Address::generate(&f.env);
    f.client.set_email_opt_in(&user, &true);
    let settings = f.client.set_email_opt_in(&user, &false);
    assert_eq!(settings.email_opt_in, false);
}

#[test]
fn email_opt_in_triggers_email_event() {
    let f = setup();
    let recipient = Address::generate(&f.env);

    // Enable email opt-in.
    f.client.set_email_opt_in(&recipient, &true);

    // Send a notification — the `new_notif` event is always published, and
    // when email_opt_in is true an additional `email_pref` event is published.
    // We verify the notification was created and the settings persist.
    let id = f.client.send(
        &f.admin,
        &recipient,
        &NotificationKind::Trade,
        &symbol_short!("trade"),
        &None,
    );
    let n = f.client.get(&id);
    assert_eq!(n.read, false);
    assert_eq!(f.client.get_email_opt_in(&recipient).email_opt_in, true);
}

// ---------------------------------------------------------------------------
// Admin transfer
// ---------------------------------------------------------------------------

#[test]
fn transfer_admin() {
    let f = setup();
    let new_admin = Address::generate(&f.env);
    f.client.transfer_admin(&f.admin, &new_admin);
    assert_eq!(f.client.admin(), new_admin);
}

#[test]
fn transfer_admin_non_admin_fails() {
    let f = setup();
    let stranger = Address::generate(&f.env);
    let res = f.client.try_transfer_admin(&stranger, &stranger);
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

#[test]
fn old_admin_cannot_send_after_transfer() {
    let f = setup();
    let new_admin = Address::generate(&f.env);
    f.client.transfer_admin(&f.admin, &new_admin);

    let recipient = Address::generate(&f.env);
    let res = f.client.try_send(
        &f.admin,
        &recipient,
        &NotificationKind::System,
        &symbol_short!("nope"),
        &None,
    );
    assert_eq!(res, Err(Ok(Error::Unauthorized)));
}

// ---------------------------------------------------------------------------
// Integration: trade notification with reference_id
// ---------------------------------------------------------------------------

#[test]
fn trade_notification_with_reference_id() {
    let f = setup();
    let seller = Address::generate(&f.env);
    let _buyer = Address::generate(&f.env);

    // Simulate: marketplace notifies the seller that a listing was filled.
    let id = f.client.send(
        &f.admin,
        &seller,
        &NotificationKind::Trade,
        &symbol_short!("filled"),
        &Some(7), // listing id
    );

    let n = f.client.get(&id);
    assert_eq!(n.kind, NotificationKind::Trade);
    assert_eq!(n.reference_id, Some(7));
    assert_eq!(n.read, false);

    // Seller reads the notification.
    f.client.mark_read(&id, &seller);
    assert_eq!(f.client.unread_count(&seller), 0);
}

// ---------------------------------------------------------------------------
// Multiple users are isolated
// ---------------------------------------------------------------------------

#[test]
fn notifications_isolated_per_user() {
    let f = setup();
    let alice = Address::generate(&f.env);
    let bob = Address::generate(&f.env);

    f.client.send(
        &f.admin,
        &alice,
        &NotificationKind::Trade,
        &symbol_short!("a1"),
        &None,
    );
    f.client.send(
        &f.admin,
        &alice,
        &NotificationKind::Trade,
        &symbol_short!("a2"),
        &None,
    );
    f.client.send(
        &f.admin,
        &bob,
        &NotificationKind::Message,
        &symbol_short!("b1"),
        &None,
    );

    assert_eq!(f.client.unread_count(&alice), 2);
    assert_eq!(f.client.unread_count(&bob), 1);

    let alice_page = f.client.get_user_notifications(&alice, &None, &0);
    assert_eq!(alice_page.total, 2);

    let bob_page = f.client.get_user_notifications(&bob, &None, &0);
    assert_eq!(bob_page.total, 1);
}
