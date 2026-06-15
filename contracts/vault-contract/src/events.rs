use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, Symbol};

const PROTOCOL: Symbol = symbol_short!("AxionVault");
const ACT_INIT: Symbol = symbol_short!("Initialize");
const ACT_DEPOSIT: Symbol = symbol_short!("Deposit");
const ACT_WITHDRAW: Symbol = symbol_short!("Withdraw");
const ACT_DISTRIBUTE: Symbol = symbol_short!("Distribute");
const ACT_CLAIM: Symbol = symbol_short!("Claim");
const EVT_ADMIN_PROPOSED: Symbol = symbol_short!("AdminProp");
const EVT_ADMIN_ACCEPTED: Symbol = symbol_short!("AdminAcpt");
const EVT_UPGRADE: Symbol = symbol_short!("Upgrade");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InitializeEvent {
    pub admin: Address,
    pub deposit_token: Address,
    pub reward_token: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawEvent {
    pub user: Address,
    pub amount: i128,
    pub remaining_balance: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributeEvent {
    pub caller: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimEvent {
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferProposedEvent {
    pub current_admin: Address,
    pub pending_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminTransferAcceptedEvent {
    pub previous_admin: Address,
    pub new_admin: Address,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeEvent {
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
    pub timestamp: u64,
}

pub fn emit_initialize(e: &Env, admin: Address, deposit_token: Address, reward_token: Address) {
    e.events().publish(
        (PROTOCOL, ACT_INIT),
        InitializeEvent {
            admin,
            deposit_token,
            reward_token,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_deposit(e: &Env, user: Address, amount: i128) {
    e.events().publish(
        (PROTOCOL, ACT_DEPOSIT),
        DepositEvent {
            user,
            amount,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_withdraw(e: &Env, user: Address, amount: i128, remaining_balance: i128) {
    e.events().publish(
        (PROTOCOL, ACT_WITHDRAW),
        WithdrawEvent {
            user,
            amount,
            remaining_balance,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_distribute(e: &Env, caller: Address, amount: i128) {
    e.events().publish(
        (PROTOCOL, ACT_DISTRIBUTE),
        DistributeEvent {
            caller,
            amount,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_claim_rewards(e: &Env, user: Address, amount: i128) {
    e.events().publish(
        (PROTOCOL, ACT_CLAIM),
        ClaimEvent {
            user,
            amount,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_admin_transfer_proposed(e: &Env, current_admin: Address, pending_admin: Address) {
    e.events().publish(
        (EVT_ADMIN_PROPOSED,),
        AdminTransferProposedEvent {
            current_admin,
            pending_admin,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_admin_transfer_accepted(e: &Env, previous_admin: Address, new_admin: Address) {
    e.events().publish(
        (EVT_ADMIN_ACCEPTED,),
        AdminTransferAcceptedEvent {
            previous_admin,
            new_admin,
            timestamp: e.ledger().timestamp(),
        },
    );
}

pub fn emit_upgrade(e: &Env, admin: Address, new_wasm_hash: BytesN<32>) {
    e.events().publish(
        (EVT_UPGRADE,),
        UpgradeEvent {
            admin,
            new_wasm_hash,
            timestamp: e.ledger().timestamp(),
        },
    );
}
