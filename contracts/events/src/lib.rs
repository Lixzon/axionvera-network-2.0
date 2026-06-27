#![no_std]

use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

/// Current event schema version.
pub const EVENT_VERSION: u32 = 1;

/// Protocol identifier used as Topic 1 for all vault events.
pub const PROTOCOL: Symbol = symbol_short!("AxVault");

// ---------------------------------------------------------------------------
// Action Symbols — used as Topic 2 for all events
// ---------------------------------------------------------------------------
pub const ACT_INIT: Symbol = symbol_short!("init");
pub const ACT_DEPOSIT: Symbol = symbol_short!("deposit");
pub const ACT_WITHDRAW: Symbol = symbol_short!("withdraw");
pub const ACT_DISTRIBUTE: Symbol = symbol_short!("distrib");
pub const ACT_CLAIM: Symbol = symbol_short!("claim");
pub const ACT_LOCK: Symbol = symbol_short!("lock");
pub const ACT_UNLOCK: Symbol = symbol_short!("unlock");
pub const ACT_ADMIN_PROPOSED: Symbol = symbol_short!("admin_prp");
pub const ACT_ADMIN_ACCEPTED: Symbol = symbol_short!("adm_acpt");
pub const ACT_UPGRADE: Symbol = symbol_short!("upgrade");
pub const ACT_PAUSE: Symbol = symbol_short!("pause");
pub const ACT_UNPAUSE: Symbol = symbol_short!("unpause");
pub const ACT_ASSET_ADDED: Symbol = symbol_short!("asset_add");
pub const ACT_ASSET_DEPOSIT: Symbol = symbol_short!("asset_dep");
pub const ACT_ASSET_WITHDRAW: Symbol = symbol_short!("asset_wd");
pub const ACT_ASSET_DISTRIBUTE: Symbol = symbol_short!("ast_dist");
pub const ACT_ASSET_CLAIM: Symbol = symbol_short!("asset_clm");

// ---------------------------------------------------------------------------
// Storage keys used by the indexing layer
// ---------------------------------------------------------------------------
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Global event log (Vec<EventLogEntry>)
    EventLog,
    /// Per-user event log keyed by address (Vec<EventLogEntry>)
    UserEventLog(Address),
    /// Set of all users who have ever interacted (Map<Address, bool>)
    InteractingUsers,
}

// ---------------------------------------------------------------------------
// Event payload structs
// All events follow the two-topic (PROTOCOL, ACTION) design
// and include an `event_version` field for schema evolution.
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializeEvent {
    pub event_version: u32,
    pub admin: Address,
    pub deposit_token: Address,
    pub reward_token: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    pub event_version: u32,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawEvent {
    pub event_version: u32,
    pub user: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributeEvent {
    pub event_version: u32,
    pub caller: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimEvent {
    pub event_version: u32,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferProposedEvent {
    pub event_version: u32,
    pub current_admin: Address,
    pub pending_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferAcceptedEvent {
    pub event_version: u32,
    pub previous_admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEvent {
    pub event_version: u32,
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PauseEvent {
    pub event_version: u32,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnpauseEvent {
    pub event_version: u32,
    pub admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetAddedEvent {
    pub event_version: u32,
    pub asset: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetDepositEvent {
    pub event_version: u32,
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetWithdrawEvent {
    pub event_version: u32,
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetDistributeEvent {
    pub event_version: u32,
    pub caller: Address,
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetClaimEvent {
    pub event_version: u32,
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LockEvent {
    pub event_version: u32,
    pub user: Address,
    pub amount: i128,
    pub unlock_timestamp: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnlockEvent {
    pub event_version: u32,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}

// ---------------------------------------------------------------------------
// Helper: get the ledger timestamp
// ---------------------------------------------------------------------------

pub fn ledger_timestamp(e: &Env) -> u64 {
    e.ledger().timestamp()
}
