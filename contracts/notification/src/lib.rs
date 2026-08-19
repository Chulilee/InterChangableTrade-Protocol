#![no_std]

//! # Notification
//!
//! On-chain notification storage for the InterChangableTrade protocol.
//!
//! Provides per-user notification delivery (trade events, messages, system
//! alerts) with read/unread tracking, unread counts, and opt-in/out email
//! preferences persisted on-chain.
//!
//! Notifications are stored in persistent storage so they survive across
//! ledgers. An event is emitted for every new notification so off-chain
//! indexers (WebSocket relays, polling services) can pick them up in real time.

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const EVT_NOTIFICATION: Symbol = symbol_short!("new_notif");
const EVT_READ: Symbol = symbol_short!("read");
const EVT_READ_ALL: Symbol = symbol_short!("read_all");
const EVT_EMAIL_PREF: Symbol = symbol_short!("eml_pref");

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Storage keys used by the notification contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Admin address that can send notifications.
    Admin,
    /// Auto-incrementing id for the next notification.
    NextId,
    /// A single notification keyed by its id.
    Notification(u64),
    /// Per-user: set of notification ids addressed to this user.
    UserNotifications(Address),
    /// Per-user: the user's notification preferences.
    UserSettings(Address),
    /// Per-user: cached unread count.
    UnreadCount(Address),
}

/// The category of a notification.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotificationKind {
    /// A trade-related event (listing, fill, settlement, etc.).
    Trade = 1,
    /// A direct message from another participant.
    Message = 2,
    /// A system-level alert (pause, parameter change, etc.).
    System = 3,
}

/// A single notification record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notification {
    pub id: u64,
    /// The user this notification is addressed to.
    pub recipient: Address,
    /// Category of the notification.
    pub kind: NotificationKind,
    /// A short human-readable summary (max ~32 bytes).
    pub title: Symbol,
    /// Optional reference id (trade id, proposal id, etc.) so the client can
    /// navigate to the relevant page.
    pub reference_id: Option<u64>,
    /// Whether the recipient has read this notification.
    pub read: bool,
    /// Ledger timestamp when the notification was created.
    pub timestamp: u64,
}

/// Per-user email notification preference.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSettings {
    /// Whether the user opted in to receive email copies of notifications.
    pub email_opt_in: bool,
}

/// Summary returned by `get_user_notifications`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationPage {
    pub notifications: Vec<Notification>,
    pub unread_count: u64,
    pub total: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    NotificationNotFound = 4,
    AlreadyRead = 5,
    EmptyTitle = 6,
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct NotificationService;

#[contractimpl]
impl NotificationService {
    // -- lifecycle -----------------------------------------------------------

    /// One-time initialization. Sets the admin who may send notifications.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::NextId, &0u64);
        Ok(())
    }

    // -- sending ------------------------------------------------------------

    /// Send a notification to `recipient`. Only the admin may call this.
    ///
    /// Returns the new notification's id.
    pub fn send(
        env: Env,
        caller: Address,
        recipient: Address,
        kind: NotificationKind,
        title: Symbol,
        reference_id: Option<u64>,
    ) -> Result<u64, Error> {
        Self::ensure_init(&env)?;
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        if title == symbol_short!("") {
            return Err(Error::EmptyTitle);
        }

        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextId)
            .unwrap_or(0);
        let timestamp = env.ledger().timestamp();

        let notification = Notification {
            id,
            recipient: recipient.clone(),
            kind,
            title,
            reference_id,
            read: false,
            timestamp,
        };

        // Persist the notification.
        env.storage()
            .persistent()
            .set(&DataKey::Notification(id), &notification);

        // Append to the recipient's notification list.
        let mut user_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserNotifications(recipient.clone()))
            .unwrap_or(Vec::new(&env));
        user_ids.push_back(id);
        env.storage().persistent().set(
            &DataKey::UserNotifications(recipient.clone()),
            &user_ids,
        );

        // Increment unread count.
        let prev: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::UnreadCount(recipient.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(
            &DataKey::UnreadCount(recipient.clone()),
            &(prev + 1),
        );

        // Auto-update email_preference_sent flag (simulated email send).
        let settings: UserSettings = env
            .storage()
            .persistent()
            .get(&DataKey::UserSettings(recipient.clone()))
            .unwrap_or(UserSettings {
                email_opt_in: false,
            });
        if settings.email_opt_in {
            // Simulated email send – in production this would call an oracle
            // or off-chain worker. We simply emit a dedicated event.
            env.events().publish(
                (EVT_EMAIL_PREF, id),
                (recipient.clone(), notification.title.clone()),
            );
        }

        // Emit notification event for real-time off-chain indexing.
        env.events()
            .publish((EVT_NOTIFICATION, id), notification);

        // Increment global next id.
        env.storage()
            .instance()
            .set(&DataKey::NextId, &(id + 1));

        Ok(id)
    }

    // -- reading / marking ---------------------------------------------------

    /// Fetch a single notification by id.
    pub fn get(env: Env, id: u64) -> Result<Notification, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Notification(id))
            .ok_or(Error::NotificationNotFound)
    }

    /// Return all notifications for `user`, newest first, with unread count.
    pub fn get_user_notifications(env: Env, user: Address) -> NotificationPage {
        let user_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserNotifications(user.clone()))
            .unwrap_or(Vec::new(&env));

        let mut notifications = Vec::new(&env);
        let total = user_ids.len() as u64;

        // Iterate in reverse so newest appear first.
        let mut i = user_ids.len();
        while i > 0 {
            i -= 1;
            if let Some(id) = user_ids.get(i) {
                if let Some(n) = env
                    .storage()
                    .persistent()
                    .get::<DataKey, Notification>(&DataKey::Notification(id))
                {
                    notifications.push_back(n);
                }
            }
        }

        let unread_count = env
            .storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::UnreadCount(user))
            .unwrap_or(0);

        NotificationPage {
            notifications,
            unread_count,
            total,
        }
    }

    /// Mark a single notification as read. Only the recipient may do this.
    pub fn mark_read(env: Env, id: u64, caller: Address) -> Result<Notification, Error> {
        caller.require_auth();
        let mut notification: Notification = env
            .storage()
            .persistent()
            .get(&DataKey::Notification(id))
            .ok_or(Error::NotificationNotFound)?;

        if notification.recipient != caller {
            return Err(Error::Unauthorized);
        }
        if notification.read {
            return Err(Error::AlreadyRead);
        }

        notification.read = true;
        env.storage()
            .persistent()
            .set(&DataKey::Notification(id), &notification);

        // Decrement unread count.
        let prev: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::UnreadCount(caller.clone()))
            .unwrap_or(0);
        if prev > 0 {
            env.storage().persistent().set(
                &DataKey::UnreadCount(caller.clone()),
                &(prev - 1),
            );
        }

        env.events()
            .publish((EVT_READ, id), caller);

        Ok(notification)
    }

    /// Mark all notifications for `user` as read.
    pub fn mark_all_read(env: Env, caller: Address) -> Result<u64, Error> {
        caller.require_auth();
        let user_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserNotifications(caller.clone()))
            .unwrap_or(Vec::new(&env));

        let mut marked: u64 = 0;
        for id in user_ids.iter() {
            if let Some(mut n) = env
                .storage()
                .persistent()
                .get::<DataKey, Notification>(&DataKey::Notification(id))
            {
                if !n.read {
                    n.read = true;
                    env.storage()
                        .persistent()
                        .set(&DataKey::Notification(id), &n);
                    marked += 1;
                }
            }
        }

        env.storage().persistent().set(
            &DataKey::UnreadCount(caller.clone()),
            &0u64,
        );

        env.events()
            .publish((EVT_READ_ALL, marked), caller);

        Ok(marked)
    }

    /// Return the unread count for `user`.
    pub fn unread_count(env: Env, user: Address) -> u64 {
        env.storage()
            .persistent()
            .get(&DataKey::UnreadCount(user))
            .unwrap_or(0)
    }

    // -- email preferences ---------------------------------------------------

    /// Set email notification opt-in preference. Only the user themselves may
    /// call this.
    pub fn set_email_opt_in(env: Env, user: Address, opt_in: bool) -> Result<UserSettings, Error> {
        user.require_auth();

        let settings = UserSettings { email_opt_in: opt_in };
        env.storage().persistent().set(
            &DataKey::UserSettings(user.clone()),
            &settings,
        );

        env.events()
            .publish((EVT_EMAIL_PREF, opt_in), user);

        Ok(settings)
    }

    /// Read the current email preference for `user`.
    pub fn get_email_opt_in(env: Env, user: Address) -> UserSettings {
        env.storage()
            .persistent()
            .get(&DataKey::UserSettings(user))
            .unwrap_or(UserSettings {
                email_opt_in: false,
            })
    }

    // -- admin ---------------------------------------------------------------

    /// Transfer admin role. Only the current admin may call this.
    pub fn transfer_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), Error> {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if caller != admin {
            return Err(Error::Unauthorized);
        }
        env.storage()
            .instance()
            .set(&DataKey::Admin, &new_admin);
        Ok(())
    }

    /// Return the current admin.
    pub fn admin(env: Env) -> Address {
        env.storage().instance().get(&DataKey::Admin).unwrap()
    }

    // -- internal ------------------------------------------------------------

    fn ensure_init(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }
}

#[cfg(test)]
mod test;
