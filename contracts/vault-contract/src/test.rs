#![cfg(test)]

//! Integration tests for the AxionVera Vault contract.
//!
//! These tests verify the core functionality of the contract, including
//! initialization, security guards, and basic interaction flows.

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Verifies that the contract can only be initialized once.
#[test]
fn test_initialization_is_one_time() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64; // 1 day

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let result = client.try_initialize(&admin, &deposit_token, &reward_token, &vesting_period);
    
    assert_eq!(
        result,
        Err(Ok(VaultError::AlreadyInitialized))
    );
}

/// Verifies that the `initialize` function requires the admin's authorization.
#[test]
fn test_initialize_requires_admin_auth() {
    let e = Env::default();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    let result = client.try_initialize(&admin, &deposit_token, &reward_token, &vesting_period);
    
    assert!(result.is_err());
}

/// Verifies that the contract cannot be initialized with identical tokens.
#[test]
fn test_initialize_fails_with_same_tokens() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let vesting_period = 86400u64;

    let result = client.try_initialize(&admin, &token, &token, &vesting_period);
    
    assert_eq!(
        result,
        Err(Ok(VaultError::InvalidTokenConfiguration))
    );
}

/// Tests vesting period functionality.
#[test]
fn test_vesting() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64; // 1 day in seconds

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let user = Address::generate(&e);

    // Set up mock token clients
    let deposit_token_client = soroban_sdk::token::Client::new(&e, &deposit_token);
    let reward_token_client = soroban_sdk::token::Client::new(&e, &reward_token);

    // Mock token balances
    e.as_contract(&deposit_token, || {
        e.storage().instance().set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage().instance().set(&soroban_sdk::token::DataKey::Balance(user.clone()), &1000i128);
        e.storage().instance().set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });
    e.as_contract(&reward_token, || {
        e.storage().instance().set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage().instance().set(&soroban_sdk::token::DataKey::Balance(admin.clone()), &10000i128);
        e.storage().instance().set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // User deposits tokens
    client.deposit(&user, &100i128);

    // Set timestamp for distribution
    e.ledger().set_timestamp(1000);

    // Admin distributes rewards
    client.distribute_rewards(&admin, &200000i128);

    // Check pending rewards
    let pending = client.pending_rewards(&user);
    assert_eq!(pending, 200000);

    // Check vested rewards immediately (should be 0)
    let vested = client.vested_rewards(&user);
    assert_eq!(vested, 0);

    // Advance time halfway through vesting period
    e.ledger().set_timestamp(1000 + 43200);

    // Check vested rewards (should be half)
    let vested = client.vested_rewards(&user);
    assert_eq!(vested, 100000);

    // Advance time past vesting period
    e.ledger().set_timestamp(1000 + 86400 + 1);

    // Check vested rewards (should be full)
    let vested = client.vested_rewards(&user);
    assert_eq!(vested, 200000);

    // Claim rewards
    let claimed = client.claim_rewards(&user);
    assert_eq!(claimed, 200000);
}
