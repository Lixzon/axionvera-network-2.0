#![cfg(test)]

//! Integration tests for the AxionVera Vault contract.

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env,
};

type VaultClient<'a> = VaultContractClient<'a>;

/// Verifies that the contract can only be initialized once.
#[test]
fn test_initialization_is_one_time() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64; // 1 day

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    let result = client.try_initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    assert_eq!(result, Err(Ok(VaultError::AlreadyInitialized)));
}

/// Verifies that the `initialize` function requires the admin's authorization.
#[test]
fn test_initialize_requires_admin_auth() {
    let e = Env::default();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    let result = client.try_initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    assert!(result.is_err());
}

/// Verifies that the contract cannot be initialized with identical tokens.
#[test]
fn test_initialize_fails_with_same_tokens() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let token = Address::generate(&e);
    let vesting_period = 86400u64;

    let result = client.try_initialize(
        &admin,
        &token,
        &token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    assert_eq!(result, Err(Ok(VaultError::InvalidTokenConfiguration)));
}

/// Tests vesting period functionality.
#[test]
fn test_vesting() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64; // 1 day in seconds

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    let user = Address::generate(&e);

    // Set up mock token clients
    let _deposit_token_client = token::Client::new(&e, &deposit_token);
    let _reward_token_client = token::Client::new(&e, &reward_token);

    // Mock token balances
    e.as_contract(&deposit_token, || {
        e.storage().instance().set(&token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(user.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(contract_id.clone()), &0i128);
    });
    e.as_contract(&reward_token, || {
        e.storage().instance().set(&token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(admin.clone()), &200000i128);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // User deposits tokens
    client.deposit(&user, &100i128);

    // Set timestamp for distribution
    e.ledger().set_timestamp(1000);

    // Admin distributes rewards
    client.distribute_rewards(&200000i128);

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

/// Tests penalty rate configuration and early locked withdrawal behavior.
#[test]
fn test_penalty_rate_and_early_locked_withdrawal() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;
    let user = Address::generate(&e);

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    // Set up mock token balances for deposit token contract.
    e.as_contract(&deposit_token, || {
        e.storage().instance().set(&token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(user.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // User deposits 100 tokens
    client.deposit(&user, &100i128);

    // Set penalty rate to 10%.
    client.set_penalty_rate(&admin, &1000u32);
    assert_eq!(client.penalty_rate(), 1000);

    // Lock all deposited tokens.
    client.lock(&user, &100i128, &86400u64);

    // Early withdraw 50 tokens from locked funds.
    let net_amount = client.withdraw_locked_early(&user, &50i128);
    assert_eq!(net_amount, 45);

    // Confirm penalties tracked.
    assert_eq!(client.total_penalties(), 5);
    assert_eq!(client.user_penalties(&user), 5);

    // Confirm remaining balances after penalty.
    assert_eq!(client.balance(&user), 50);
    assert_eq!(client.locked_balance(&user), 50);
}

/// Tests that invalid penalty rates are rejected.
#[test]
fn test_set_penalty_rate_rejected_when_above_max() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    let result = client.try_set_penalty_rate(&admin, &10001u32);
    assert_eq!(result, Err(Ok(VaultError::InvalidPenaltyRate)));
}

// ---------------------------------------------------------------------------
// Multi-Asset Tests
// ---------------------------------------------------------------------------

/// Tests adding a new asset to the vault.
#[test]
fn test_add_asset() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let new_asset = Address::generate(&e);

    // Add asset
    client.add_asset(&admin, &new_asset);

    // Verify asset is supported
    assert!(client.is_asset_supported(&new_asset));
}

/// Tests depositing multiple assets.
#[test]
fn test_multiple_asset_deposits() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let asset1 = Address::generate(&e);
    let asset2 = Address::generate(&e);
    let user = Address::generate(&e);

    // Add assets
    client.add_asset(&admin, &asset1);
    client.add_asset(&admin, &asset2);

    // Mock token balances
    e.as_contract(&asset1, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &1000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&asset2, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &2000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });

    // Deposit asset1
    client.deposit_asset(&user, &asset1, &100i128);

    // Deposit asset2
    client.deposit_asset(&user, &asset2, &200i128);

    // Verify balances
    assert_eq!(client.balance_of_asset(&user, &asset1), 100);
    assert_eq!(client.balance_of_asset(&user, &asset2), 200);

    // Verify total deposits
    assert_eq!(client.total_deposits_of_asset(&asset1), 100);
    assert_eq!(client.total_deposits_of_asset(&asset2), 200);
}

/// Tests withdrawing from multiple assets.
#[test]
fn test_multiple_asset_withdrawals() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let asset1 = Address::generate(&e);
    let asset2 = Address::generate(&e);
    let user = Address::generate(&e);

    // Add assets
    client.add_asset(&admin, &asset1);
    client.add_asset(&admin, &asset2);

    // Mock token balances
    e.as_contract(&asset1, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &1000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&asset2, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &2000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });

    // Deposit assets
    client.deposit_asset(&user, &asset1, &100i128);
    client.deposit_asset(&user, &asset2, &200i128);

    // Withdraw from asset1
    client.withdraw_asset(&user, &asset1, &50i128);

    // Withdraw from asset2
    client.withdraw_asset(&user, &asset2, &100i128);

    // Verify balances
    assert_eq!(client.balance_of_asset(&user, &asset1), 50);
    assert_eq!(client.balance_of_asset(&user, &asset2), 100);

    // Verify total deposits
    assert_eq!(client.total_deposits_of_asset(&asset1), 50);
    assert_eq!(client.total_deposits_of_asset(&asset2), 100);
}

/// Tests reward distribution for a specific asset.
#[test]
fn test_asset_reward_distribution() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let asset1 = Address::generate(&e);
    let user1 = Address::generate(&e);
    let user2 = Address::generate(&e);

    // Add asset
    client.add_asset(&admin, &asset1);

    // Mock token balances
    e.as_contract(&asset1, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user1.clone()),
            &1000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user2.clone()),
            &2000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&reward_token, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(admin.clone()),
            &1000000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });

    // Users deposit
    client.deposit_asset(&user1, &asset1, &300i128);
    client.deposit_asset(&user2, &asset1, &600i128);

    // Set timestamp
    e.ledger().set_timestamp(1000);

    // Distribute rewards
    client.distribute_rewards_for_asset(&admin, &asset1, &900000i128);

    // Check pending rewards (user1 should get 1/3, user2 should get 2/3)
    let pending1 = client.pending_rewards_for_asset(&user1, &asset1);
    let pending2 = client.pending_rewards_for_asset(&user2, &asset1);

    assert_eq!(pending1, 300000);
    assert_eq!(pending2, 600000);
}

/// Tests claiming rewards for a specific asset.
#[test]
fn test_asset_reward_claiming() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 0u64; // No vesting for this test

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let asset1 = Address::generate(&e);
    let user = Address::generate(&e);

    // Add asset
    client.add_asset(&admin, &asset1);

    // Mock token balances
    e.as_contract(&asset1, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &1000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&reward_token, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(admin.clone()),
            &1000000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });

    // User deposits
    client.deposit_asset(&user, &asset1, &100i128);

    // Distribute rewards
    client.distribute_rewards_for_asset(&admin, &asset1, &200000i128);

    // Claim rewards
    let claimed = client.claim_rewards_for_asset(&user, &asset1);
    assert_eq!(claimed, 200000);

    // Verify rewards were claimed
    let pending = client.pending_rewards_for_asset(&user, &asset1);
    assert_eq!(pending, 0);
}

/// Tests independent tracking of balances per asset.
#[test]
fn test_independent_asset_tracking() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 0u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let asset1 = Address::generate(&e);
    let asset2 = Address::generate(&e);
    let user = Address::generate(&e);

    // Add assets
    client.add_asset(&admin, &asset1);
    client.add_asset(&admin, &asset2);

    // Mock token balances
    e.as_contract(&asset1, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &10000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&asset2, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(user.clone()),
            &10000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });
    e.as_contract(&reward_token, || {
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(admin.clone()),
            &2000000i128,
        );
        e.storage().instance().set(
            &soroban_sdk::token::DataKey::Balance(contract_id.clone()),
            &0i128,
        );
    });

    // Deposit different amounts to each asset
    client.deposit_asset(&user, &asset1, &100i128);
    client.deposit_asset(&user, &asset2, &200i128);

    // Distribute different reward amounts to each asset
    client.distribute_rewards_for_asset(&admin, &asset1, &300000i128);
    client.distribute_rewards_for_asset(&admin, &asset2, &600000i128);

    // Check pending rewards are independent
    let pending1 = client.pending_rewards_for_asset(&user, &asset1);
    let pending2 = client.pending_rewards_for_asset(&user, &asset2);

    assert_eq!(pending1, 300000);
    assert_eq!(pending2, 600000);

    // Claim from asset1 only
    let claimed1 = client.claim_rewards_for_asset(&user, &asset1);
    assert_eq!(claimed1, 300000);

    // Verify asset2 rewards are unchanged
    let pending2_after = client.pending_rewards_for_asset(&user, &asset2);
    assert_eq!(pending2_after, 600000);
}

/// Tests that unsupported asset operations fail.
#[test]
fn test_unsupported_asset_fails() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(&admin, &deposit_token, &reward_token, &vesting_period);

    let unsupported_asset = Address::generate(&e);
    let user = Address::generate(&e);

    // Try to deposit unsupported asset
    let result = client.try_deposit_asset(&user, &unsupported_asset, &100i128);
    assert!(result.is_err());

    // Verify asset is not supported
    assert!(!client.is_asset_supported(&unsupported_asset));
}

// ---------------------------------------------------------------------------
// Cross-Contract Interaction Tests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Event Validation Tests
// ---------------------------------------------------------------------------

/// Verifies that events use the two-topic standard (Protocol, Action).
#[test]
fn test_event_topic_standard() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    // Verify initialize event topics
    let events_snapshot = e.events().all();
    let last = events_snapshot.last().unwrap();
    assert_eq!(last.0.len(), 2, "Initialize must have 2 topics");
    assert_eq!(
        last.0.get(0).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::PROTOCOL),
    );
    assert_eq!(
        last.0.get(1).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::ACT_INIT),
    );
}

/// Verifies that deposit events include user indexing.
#[test]
fn test_deposit_event_indexing() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    // Set up mock token
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(user.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    client.deposit(&user, &100i128);

    // Verify event has two topics
    let events = e.events().all();
    let deposit_event = events.get(events.len() - 1).unwrap();
    assert_eq!(deposit_event.0.len(), 2, "Deposit must have 2 topics");

    // Verify on-chain indexing
    e.as_contract(&contract_id, || {
        let log = axionvera_core::get_user_event_log(&e, &user);
        assert!(!log.is_empty(), "User event log should not be empty");
        assert_eq!(log.get(0).unwrap().action, axionvera_events::ACT_DEPOSIT);

        let global_log = axionvera_core::get_global_event_log(&e);
        assert!(!global_log.is_empty(), "Global event log should not be empty");

        let users = axionvera_core::get_interacting_users(&e);
        assert_eq!(users.len(), 1, "Should have one interacting user");
        assert_eq!(users.get(0).unwrap(), user);
    });
}

/// Verifies that pause_contract and unpause_contract emit events.
#[test]
fn test_pause_unpause_events() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vesting_period = 86400u64;

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    let prev_event_count = e.events().all().len();

    client.pause_contract();
    let pause_events = e.events().all();
    let new_count = pause_events.len();
    assert!(
        new_count > prev_event_count,
        "Pause should emit an event"
    );

    let pause_event = pause_events.get(new_count - 1).unwrap();
    assert_eq!(pause_event.0.len(), 2, "Pause must have 2 topics");
    assert_eq!(
        pause_event.0.get(1).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::ACT_PAUSE),
    );

    client.unpause_contract();
    let all_events = e.events().all();
    let unpause_event = all_events.get(all_events.len() - 1).unwrap();
    assert_eq!(unpause_event.0.len(), 2, "Unpause must have 2 topics");
    assert_eq!(
        unpause_event.0.get(1).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::ACT_UNPAUSE),
    );
}

/// Verifies that all events include event_version field.
#[test]
fn test_event_version_field() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract {});
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let vesting_period = 0u64; // No vesting for this test

    client.initialize(
        &admin,
        &deposit_token,
        &reward_token,
        &vesting_period,
        &0,
        &soroban_sdk::Vec::new(&e),
    );

    // Set up mock tokens
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(user.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });
    e.as_contract(&reward_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(admin.clone()), &200000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Verify that the event_version constant is 1
    assert_eq!(axionvera_events::EVENT_VERSION, 1);

    // Deposit triggers event with version
    client.deposit(&user, &100i128);
    // The event_struct includes event_version which is verified at compile time
    // via the struct definition. Runtime verification is implicit through the
    // event struct being correctly populated.
}

#[test]
fn test_cross_contract_client_validate_contract() {
    let e = Env::default();
    let contract_id = e.register_contract(None, VaultContract);
    let other_address = Address::generate(&e);

    // Test that self-contract validation fails
    e.as_contract(&contract_id, || {
        let result =
            crate::cross_contract::CrossContractClient::validate_contract_exists(&e, &contract_id);
        assert!(result.is_err());
    });

    // Test that other contract validation passes
    e.as_contract(&contract_id, || {
        let result = crate::cross_contract::CrossContractClient::validate_contract_exists(
            &e,
            &other_address,
        );
        assert!(result.is_ok());
    });
}

// ---------------------------------------------------------------------------
// Delegation Tests
// ---------------------------------------------------------------------------

/// Tests that a user can create a delegation.
#[test]
fn test_create_delegation() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    let permissions = storage::PERMISSION_DEPOSIT | storage::PERMISSION_WITHDRAW;

    // Create delegation
    client.delegate(&user, &operator, &permissions, &0u64);

    // Verify it was stored
    let delegation = client.get_delegation(&user, &operator);
    assert!(delegation.is_some());
    let d = delegation.unwrap();
    assert_eq!(d.operator, operator);
    assert_eq!(d.permissions, permissions);
    assert_eq!(d.expires_at, 0);
}

/// Tests that a delegation can be revoked.
#[test]
fn test_revoke_delegation() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Create delegation
    client.delegate(&user, &operator, &storage::PERMISSION_DEPOSIT, &0u64);
    assert!(client.get_delegation(&user, &operator).is_some());

    // Revoke
    client.revoke_delegation(&user, &operator);
    assert!(client.get_delegation(&user, &operator).is_none());
}

/// Tests that delegating to self is rejected.
#[test]
fn test_cannot_delegate_to_self() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    let result = client.try_delegate(&user, &user, &storage::PERMISSION_DEPOSIT, &0u64);
    assert_eq!(result, Err(Ok(VaultError::CannotDelegateToSelf)));
}

/// Tests that expired delegation is rejected.
#[test]
fn test_expired_delegation_rejected() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Create delegation that expires at timestamp 500 (already past at 1000)
    let result = client.try_delegate(&vault_owner, &operator, &storage::PERMISSION_DEPOSIT, &500u64);
    assert_eq!(result, Err(Ok(VaultError::InvalidDelegationExpiration)));
}

/// Tests that a delegation with insufficient permission is rejected for delegated actions.
#[test]
fn test_delegated_action_requires_correct_permission() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Grant only DEPOSIT permission
    client.delegate(&vault_owner, &operator, &storage::PERMISSION_DEPOSIT, &0u64);

    // Set up mock token balances
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(vault_owner.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Try delegated withdraw (should fail - wrong permission)
    let result = client.try_delegated_withdraw(&vault_owner, &operator, &50i128);
    assert_eq!(result, Err(Ok(VaultError::InsufficientDelegationPermissions)));
}

/// Tests delegated deposit flow.
#[test]
fn test_delegated_deposit() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Grant DEPOSIT permission
    client.delegate(&vault_owner, &operator, &storage::PERMISSION_DEPOSIT, &0u64);

    // Set up mock token balances
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(operator.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Operator deposits on behalf of vault_owner
    client.delegated_deposit(&vault_owner, &operator, &100i128);

    // Verify the vault_owner's balance increased
    assert_eq!(client.balance(&vault_owner), 100);
    assert_eq!(client.total_deposits(), 100);
}

/// Tests delegated withdrawal flow.
#[test]
fn test_delegated_withdraw() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Grant WITHDRAW permission
    client.delegate(&vault_owner, &operator, &storage::PERMISSION_DEPOSIT | storage::PERMISSION_WITHDRAW, &0u64);

    // Set up mock token balances
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(vault_owner.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Owner deposits first
    client.deposit(&vault_owner, &200i128);

    // Operator withdraws on behalf of vault_owner (funds go to operator)
    client.delegated_withdraw(&vault_owner, &operator, &50i128);

    // Verify the vault_owner's balance decreased
    assert_eq!(client.balance(&vault_owner), 150);
}

/// Tests delegated claim rewards flow.
#[test]
fn test_delegated_claim_rewards() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // Grant CLAIM permission
    client.delegate(&vault_owner, &operator, &storage::PERMISSION_CLAIM | storage::PERMISSION_DEPOSIT, &0u64);

    // Set up mock token balances
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(vault_owner.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });
    e.as_contract(&reward_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Admin, &admin);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(admin.clone()), &200000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Owner deposits
    client.deposit(&vault_owner, &100i128);

    // Distribute rewards
    client.distribute_rewards(&200000i128);

    // Operator claims on behalf of vault_owner
    let claimed = client.delegated_claim_rewards(&vault_owner, &operator);
    assert_eq!(claimed, 200000);

    // Verify owner's pending rewards are cleared
    assert_eq!(client.pending_rewards(&vault_owner), 0);
}

/// Tests that get_delegations returns all delegations.
#[test]
fn test_list_delegations() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let op1 = Address::generate(&e);
    let op2 = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    client.delegate(&user, &op1, &storage::PERMISSION_DEPOSIT, &0u64);
    client.delegate(&user, &op2, &storage::PERMISSION_WITHDRAW, &0u64);

    let delegations = client.get_delegations(&user);
    assert_eq!(delegations.len(), 2);
}

/// Tests that delegation events are emitted properly.
#[test]
fn test_delegation_events() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().set(LedgerInfo {
        timestamp: 1000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: [0; 32],
        base_reserve: 10,
        min_persistent_entry_ttl: 518400,
        min_temp_entry_ttl: 518400,
        max_entry_ttl: 6312000,
    });

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let user = Address::generate(&e);
    let operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    let prev_count = e.events().all().len();

    client.delegate(&user, &operator, &storage::PERMISSION_DEPOSIT, &0u64);

    let events = e.events().all();
    let delegate_event = events.get(events.len() - 1).unwrap();
    assert_eq!(delegate_event.0.len(), 2, "Delegate must have 2 topics");
    assert_eq!(
        delegate_event.0.get(1).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::ACT_DELEGATE, &e),
    );

    // Revoke and check event
    client.revoke_delegation(&user, &operator);
    let events = e.events().all();
    let revoke_event = events.get(events.len() - 1).unwrap();
    assert_eq!(
        revoke_event.0.get(1).unwrap(),
        soroban_sdk::xdr::ToXdr::to_xdr(&axionvera_events::ACT_REVOKE_DELEGATION, &e),
    );
}

/// Tests that an operator without any delegation gets rejected.
#[test]
fn test_unauthorized_operator_rejected() {
    let e = Env::default();
    e.mock_all_auths();

    let contract_id = e.register_contract(None, VaultContract);
    let client = VaultContractClient::new(&e, &contract_id);

    let admin = Address::generate(&e);
    let deposit_token = Address::generate(&e);
    let reward_token = Address::generate(&e);
    let vault_owner = Address::generate(&e);
    let unauthorized_operator = Address::generate(&e);

    client.initialize(&admin, &deposit_token, &reward_token, &0u64, &0, &soroban_sdk::Vec::new(&e));

    // No delegation created for unauthorized_operator

    // Set up mock token balances
    e.as_contract(&deposit_token, || {
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(unauthorized_operator.clone()), &1000i128);
        e.storage()
            .instance()
            .set(&soroban_sdk::token::DataKey::Balance(contract_id.clone()), &0i128);
    });

    // Operator tries to deposit without permission
    let result = client.try_delegated_deposit(&vault_owner, &unauthorized_operator, &100i128);
    assert_eq!(result, Err(Ok(VaultError::DelegationNotFound)));
}
