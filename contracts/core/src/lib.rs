#![no_std]

use soroban_sdk::{contracttype, Address, Env, Map, Vec};

use axionvera_events as events;

/// Maximum number of event log entries stored per user index.
const MAX_EVENTS_PER_USER: u32 = 50;

/// Maximum number of event log entries stored globally.
const MAX_GLOBAL_EVENTS: u32 = 200;

/// A lightweight on-chain event log entry for indexing.
/// Stores only key metadata; the full event is emitted via Soroban topics.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogEntry {
    pub action: soroban_sdk::Symbol,
    pub user: Option<Address>,
    pub asset: Option<Address>,
    pub amount: i128,
    pub timestamp: u64,
    pub ledger: u32,
}

/// Index a newly emitted event by appending to the global event log.
pub fn index_event(
    e: &Env,
    action: soroban_sdk::Symbol,
    user: Option<Address>,
    asset: Option<Address>,
    amount: i128,
) {
    let entry = EventLogEntry {
        action,
        user: user.clone(),
        asset,
        amount,
        timestamp: e.ledger().timestamp(),
        ledger: e.ledger().sequence(),
    };

    // Append to the global event log
    let mut global_log: Vec<EventLogEntry> = e
        .storage()
        .instance()
        .get(&events::DataKey::EventLog)
        .unwrap_or_else(|| Vec::new(e));
    global_log.push_back(entry.clone());
    // Trim oldest entries when the log exceeds capacity
    while global_log.len() > MAX_GLOBAL_EVENTS {
        _ = global_log.remove(0);
    }
    e.storage()
        .instance()
        .set(&events::DataKey::EventLog, &global_log);

    // Append to the per-user event log
    if let Some(user_addr) = user {
        let mut user_log: Vec<EventLogEntry> = e
            .storage()
            .persistent()
            .get(&events::DataKey::UserEventLog(user_addr.clone()))
            .unwrap_or_else(|| Vec::new(e));
        user_log.push_back(entry);
        while user_log.len() > MAX_EVENTS_PER_USER {
            _ = user_log.remove(0);
        }
        e.storage()
            .persistent()
            .set(&events::DataKey::UserEventLog(user_addr), &user_log);
    }

    e.storage().instance().extend_ttl(518400, 518400);
}

/// Retrieve the global event log.
pub fn get_global_event_log(e: &Env) -> Vec<EventLogEntry> {
    e.storage()
        .instance()
        .get(&events::DataKey::EventLog)
        .unwrap_or_else(|| Vec::new(e))
}

/// Retrieve the event log for a specific user.
pub fn get_user_event_log(e: &Env, user: &Address) -> Vec<EventLogEntry> {
    e.storage()
        .persistent()
        .get(&events::DataKey::UserEventLog(user.clone()))
        .unwrap_or_else(|| Vec::new(e))
}

/// Maintain a set of unique users who have interacted with the contract.
pub fn record_interacting_user(e: &Env, user: &Address) {
    let mut users: Map<Address, bool> = e
        .storage()
        .instance()
        .get(&events::DataKey::InteractingUsers)
        .unwrap_or_else(|| Map::new(e));
    if !users.contains_key(user.clone()) {
        users.set(user.clone(), true);
        e.storage()
            .instance()
            .set(&events::DataKey::InteractingUsers, &users);
    }
}

/// Retrieve the set of all users who have interacted with the contract.
pub fn get_interacting_users(e: &Env) -> Vec<Address> {
    let users: Map<Address, bool> = e
        .storage()
        .instance()
        .get(&events::DataKey::InteractingUsers)
        .unwrap_or_else(|| Map::new(e));
    users.keys()
}
