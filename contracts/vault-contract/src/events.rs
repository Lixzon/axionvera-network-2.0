use soroban_sdk::{Address, BytesN, Env};

use axionvera_core;
use axionvera_events::{
    self,
    AdminTransferAcceptedEvent, AdminTransferProposedEvent, AssetAddedEvent, AssetClaimEvent,
    AssetDepositEvent, AssetDistributeEvent, AssetWithdrawEvent, ClaimEvent, DepositEvent,
    DistributeEvent, InitializeEvent, LockEvent, PauseEvent, UnlockEvent, UnpauseEvent,
    UpgradeEvent, WithdrawEvent, EVENT_VERSION, PROTOCOL, ACT_INIT, ACT_DEPOSIT, ACT_WITHDRAW,
    ACT_DISTRIBUTE, ACT_CLAIM, ACT_LOCK, ACT_UNLOCK, ACT_ADMIN_PROPOSED, ACT_ADMIN_ACCEPTED,
    ACT_UPGRADE, ACT_PAUSE, ACT_UNPAUSE, ACT_ASSET_ADDED, ACT_ASSET_DEPOSIT, ACT_ASSET_WITHDRAW,
    ACT_ASSET_DISTRIBUTE, ACT_ASSET_CLAIM,
};

pub fn emit_initialize(e: &Env, admin: Address, deposit_token: Address, reward_token: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_INIT),
        InitializeEvent {
            event_version: EVENT_VERSION,
            admin: admin.clone(),
            deposit_token,
            reward_token,
            timestamp: ts,
        },
    );
}

pub fn emit_deposit(e: &Env, user: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_DEPOSIT),
        DepositEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            amount,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_DEPOSIT, Some(user.clone()), None, amount);
    axionvera_core::record_interacting_user(e, &user);
}

pub fn emit_withdraw(e: &Env, user: Address, amount: i128, remaining_balance: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_WITHDRAW),
        WithdrawEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            amount,
            remaining_balance,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_WITHDRAW, Some(user.clone()), None, amount);
}

pub fn emit_distribute(e: &Env, caller: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_DISTRIBUTE),
        DistributeEvent {
            event_version: EVENT_VERSION,
            caller,
            amount,
            timestamp: ts,
        },
    );
}

pub fn emit_claim_rewards(e: &Env, user: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_CLAIM),
        ClaimEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            amount,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_CLAIM, Some(user.clone()), None, amount);
}

pub fn emit_lock(e: &Env, user: Address, amount: i128, unlock_timestamp: u64) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_LOCK),
        LockEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            amount,
            unlock_timestamp,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_LOCK, Some(user.clone()), None, amount);
}

pub fn emit_unlock(e: &Env, user: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_UNLOCK),
        UnlockEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            amount,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_UNLOCK, Some(user.clone()), None, amount);
}

pub fn emit_admin_transfer_proposed(e: &Env, current_admin: Address, pending_admin: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ADMIN_PROPOSED),
        AdminTransferProposedEvent {
            event_version: EVENT_VERSION,
            current_admin,
            pending_admin,
            timestamp: ts,
        },
    );
}

pub fn emit_admin_transfer_accepted(e: &Env, previous_admin: Address, new_admin: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ADMIN_ACCEPTED),
        AdminTransferAcceptedEvent {
            event_version: EVENT_VERSION,
            previous_admin,
            new_admin,
            timestamp: ts,
        },
    );
}

pub fn emit_upgrade(e: &Env, admin: Address, new_wasm_hash: BytesN<32>) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_UPGRADE),
        UpgradeEvent {
            event_version: EVENT_VERSION,
            admin,
            new_wasm_hash,
            timestamp: ts,
        },
    );
}

pub fn emit_pause(e: &Env, admin: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_PAUSE),
        PauseEvent {
            event_version: EVENT_VERSION,
            admin,
            timestamp: ts,
        },
    );
}

pub fn emit_unpause(e: &Env, admin: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_UNPAUSE),
        UnpauseEvent {
            event_version: EVENT_VERSION,
            admin,
            timestamp: ts,
        },
    );
}

pub fn emit_asset_added(e: &Env, asset: Address) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ASSET_ADDED),
        AssetAddedEvent {
            event_version: EVENT_VERSION,
            asset,
            timestamp: ts,
        },
    );
}

pub fn emit_asset_deposit(e: &Env, user: Address, asset: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ASSET_DEPOSIT),
        AssetDepositEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            asset: asset.clone(),
            amount,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_ASSET_DEPOSIT, Some(user.clone()), Some(asset), amount);
}

pub fn emit_asset_withdraw(e: &Env, user: Address, asset: Address, amount: i128, remaining_balance: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ASSET_WITHDRAW),
        AssetWithdrawEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            asset: asset.clone(),
            amount,
            remaining_balance,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_ASSET_WITHDRAW, Some(user.clone()), Some(asset), amount);
}

pub fn emit_asset_distribute(e: &Env, caller: Address, asset: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ASSET_DISTRIBUTE),
        AssetDistributeEvent {
            event_version: EVENT_VERSION,
            caller,
            asset,
            amount,
            timestamp: ts,
        },
    );
}

pub fn emit_asset_claim_rewards(e: &Env, user: Address, asset: Address, amount: i128) {
    let ts = axionvera_events::ledger_timestamp(e);
    e.events().publish(
        (PROTOCOL, ACT_ASSET_CLAIM),
        AssetClaimEvent {
            event_version: EVENT_VERSION,
            user: user.clone(),
            asset: asset.clone(),
            amount,
            timestamp: ts,
        },
    );
    axionvera_core::index_event(e, ACT_ASSET_CLAIM, Some(user.clone()), Some(asset), amount);
}
