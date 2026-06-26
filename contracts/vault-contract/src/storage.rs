use soroban_sdk::{contracttype, Address, Env, Map};

use crate::errors::{
    ArithmeticError, AuthorizationError, BalanceError, StateError, ValidationError, VaultError,
};

pub const PRECISION_FACTOR: i128 = 1_000_000_000;
const REWARD_INDEX_SCALE: i128 = PRECISION_FACTOR;

const INSTANCE_TTL_THRESHOLD: u32 = 518_400;
const INSTANCE_TTL_EXTEND_TO: u32 = 518_400;

const PERSISTENT_TTL_THRESHOLD: u32 = 518_400;
const PERSISTENT_TTL_EXTEND_TO: u32 = 518_400;

/// Keys used to store data in the contract's storage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Flag indicating if the contract has been initialized.
    Initialized,
    /// Admin address
    Admin,
    /// Pending admin address (for two-step transfer)
    PendingAdmin,
    /// Deposit token address (legacy, kept for backwards compatibility)
    DepositToken,
    /// Reward token address
    RewardToken,
    /// Total deposits amount (legacy, kept for backwards compatibility)
    TotalDeposits,
    /// Global reward index (legacy, kept for backwards compatibility)
    RewardIndex,
    /// Vesting period in seconds
    VestingPeriod,
    /// Target deposit amount for utilization calculation
    TargetDeposits,
    /// Multiplier points for dynamic reward calculation
    UtilizationMultipliers,
    /// Reentrancy guard flag
    ReentrancyGuard,
    /// Pause flag
    IsPaused,
    /// User balance (legacy, kept for backwards compatibility)
    UserBalance(Address),
    /// User's last synced reward index (legacy, kept for backwards compatibility)
    UserRewardIndex(Address),
    /// User's accrued but unvested rewards (legacy, kept for backwards compatibility)
    UserAccruedRewards(Address),
    /// User's last reward distribution timestamp (legacy, kept for backwards compatibility)
    UserLastRewardTimestamp(Address),
    /// User's liquid balance (available for immediate withdrawal)
    UserLiquidBalance(Address),
    /// User's active lock entries
    UserLocks(Address),
    /// Penalty rate in basis points for early withdrawals
    PenaltyRateBps,
    /// Total penalty amount collected by the vault
    TotalPenalties,
    /// Total penalty amount paid by a specific user
    UserPenaltyPaid(Address),
    /// Map of supported asset addresses
    SupportedAssets,
    /// Total deposits per asset
    AssetTotalDeposits(Address),
    /// Global reward index per asset
    AssetRewardIndex(Address),
    /// User balance per asset
    UserAssetBalance(Address, Address), // (user, asset)
    /// User's last synced reward index per asset
    UserAssetRewardIndex(Address, Address), // (user, asset)
    /// User's accrued but unvested rewards per asset
    UserAssetAccruedRewards(Address, Address), // (user, asset)
    /// User's last reward distribution timestamp per asset
    UserAssetLastRewardTimestamp(Address, Address), // (user, asset)
}

/// The global state of the vault contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultState {
    /// The address allowed to perform administrative actions like reward distribution.
    pub admin: Address,
    /// The address of the token that users deposit into the vault.
    pub deposit_token: Address,
    /// The address of the token distributed as rewards.
    pub reward_token: Address,
    /// The total amount of deposit tokens currently held by the vault.
    pub total_deposits: i128,
    /// The global reward index that tracks cumulative rewards per unit of deposit.
    pub reward_index: i128,
    /// The vesting period in seconds.
    pub vesting_period: u64,
    /// The target deposit amount for calculating utilization.
    pub target_deposits: i128,
    /// The penalty rate applied to early locked withdrawals (bps).
    pub penalty_rate_bps: u32,
    /// Total penalty amount collected by the vault.
    pub total_penalties: i128,
    /// A list of points defining the utilization-to-reward multiplier curve.
    pub utilization_multipliers: soroban_sdk::Vec<MultiplierPoint>,
}

/// Snapshot of a user's position in the vault for a specific asset.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPosition {
    /// The amount of deposit tokens the user has currently staked.
    pub balance: i128,
    /// The value of the global reward index at the time of the user's last interaction.
    pub reward_index: i128,
    /// The amount of rewards the user has earned but not yet vested/claimed.
    pub accrued_rewards: i128,
    /// The timestamp of the last reward distribution affecting this user.
    pub last_reward_timestamp: u64,
}

/// A single time-locked deposit entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Lock {
    /// The amount locked in this entry.
    pub amount: i128,
    /// The earliest timestamp when these funds may be unlocked without penalty.
    pub unlock_timestamp: u64,
    /// A placeholder for future locked-reward multiplier support.
    pub reward_multiplier: u32,
}

/// A reward multiplier entry used to compute dynamic reward curves.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiplierPoint {
    /// Utilization threshold in basis points.
    pub utilization_bps: u32,
    /// Reward multiplier in basis points.
    pub multiplier_bps: u32,
}

/// Snapshot of a user's position across multiple assets.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiAssetPosition {
    /// Map of asset address to user position
    pub positions: Map<Address, UserPosition>,
}

/// A helper struct for returning reward information in view functions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserRewardSnapshot {
    /// The current reward index applied to the snapshot.
    pub reward_index: i128,
    /// The total rewards (accrued + pending) for the user.
    pub rewards: i128,
    /// The amount of vested rewards available to claim.
    pub vested_rewards: i128,
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

pub fn is_initialized(e: &Env) -> bool {
    e.storage()
        .instance()
        .get::<_, bool>(&DataKey::Initialized)
        .unwrap_or(false)
}

pub fn require_initialized(e: &Env) -> Result<(), VaultError> {
    if is_initialized(e) {
        Ok(())
    } else {
        Err(StateError::NotInitialized.into())
    }
}

pub fn require_not_paused(e: &Env) -> Result<(), VaultError> {
    if e.storage()
        .instance()
        .get::<_, bool>(&DataKey::IsPaused)
        .unwrap_or(false)
    {
        Err(AuthorizationError::Unauthorized.into())
    } else {
        Ok(())
    }
}

pub fn initialize_state(
    e: &Env,
    admin: &Address,
    deposit_token: &Address,
    reward_token: &Address,
    vesting_period: u64,
    target_deposits: i128,
    utilization_multipliers: &soroban_sdk::Vec<MultiplierPoint>,
) {
    e.storage().instance().set(&DataKey::Initialized, &true);
    e.storage().instance().set(&DataKey::Admin, admin);
    e.storage().instance().remove(&DataKey::PendingAdmin);
    e.storage()
        .instance()
        .set(&DataKey::DepositToken, deposit_token);
    e.storage()
        .instance()
        .set(&DataKey::RewardToken, reward_token);
    e.storage()
        .instance()
        .set(&DataKey::VestingPeriod, &vesting_period);
    e.storage().instance().set(&DataKey::TotalDeposits, &0_i128);
    e.storage().instance().set(&DataKey::RewardIndex, &0_i128);
    e.storage()
        .instance()
        .set(&DataKey::TargetDeposits, &target_deposits);
    e.storage().instance().set(&DataKey::PenaltyRateBps, &0_u32);
    e.storage()
        .instance()
        .set(&DataKey::TotalPenalties, &0_i128);
    e.storage()
        .instance()
        .set(&DataKey::UtilizationMultipliers, utilization_multipliers);
    e.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &false);
    e.storage().instance().set(&DataKey::IsPaused, &false);
    bump_instance_ttl(e);
}

// ---------------------------------------------------------------------------
// State (global)
// ---------------------------------------------------------------------------

pub fn get_state(e: &Env) -> Result<VaultState, VaultError> {
    require_initialized(e)?;
    let admin = e
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(StateError::InvalidState)?;
    let deposit_token = e
        .storage()
        .instance()
        .get(&DataKey::DepositToken)
        .ok_or(StateError::InvalidState)?;
    let reward_token = e
        .storage()
        .instance()
        .get(&DataKey::RewardToken)
        .ok_or(StateError::InvalidState)?;
    let total_deposits = e
        .storage()
        .instance()
        .get(&DataKey::TotalDeposits)
        .unwrap_or(0_i128);
    let reward_index = e
        .storage()
        .instance()
        .get(&DataKey::RewardIndex)
        .unwrap_or(0_i128);
    let vesting_period = e
        .storage()
        .instance()
        .get(&DataKey::VestingPeriod)
        .unwrap_or(0_u64);
    let target_deposits = e
        .storage()
        .instance()
        .get(&DataKey::TargetDeposits)
        .unwrap_or(0_i128);
    let utilization_multipliers = e
        .storage()
        .instance()
        .get(&DataKey::UtilizationMultipliers)
        .unwrap_or_else(|| soroban_sdk::Vec::new(e));
    let penalty_rate_bps = e
        .storage()
        .instance()
        .get(&DataKey::PenaltyRateBps)
        .unwrap_or(0_u32);
    let total_penalties = e
        .storage()
        .instance()
        .get(&DataKey::TotalPenalties)
        .unwrap_or(0_i128);
    bump_instance_ttl(e);
    Ok(VaultState {
        admin,
        deposit_token,
        reward_token,
        total_deposits,
        reward_index,
        vesting_period,
        target_deposits,
        penalty_rate_bps,
        total_penalties,
        utilization_multipliers,
    })
}

pub fn get_admin(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.admin)
}

pub fn set_admin(e: &Env, admin: &Address) {
    e.storage().instance().set(&DataKey::Admin, admin);
    bump_instance_ttl(e);
}

pub fn get_pending_admin(e: &Env) -> Result<Option<Address>, VaultError> {
    require_initialized(e)?;
    let pending = e.storage().instance().get(&DataKey::PendingAdmin);
    bump_instance_ttl(e);
    Ok(pending)
}

pub fn set_pending_admin(e: &Env, pending_admin: &Address) {
    e.storage()
        .instance()
        .set(&DataKey::PendingAdmin, pending_admin);
    bump_instance_ttl(e);
}

pub fn clear_pending_admin(e: &Env) {
    e.storage().instance().remove(&DataKey::PendingAdmin);
    bump_instance_ttl(e);
}

pub fn get_deposit_token(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.deposit_token)
}

pub fn get_reward_token(e: &Env) -> Result<Address, VaultError> {
    Ok(get_state(e)?.reward_token)
}

pub fn get_total_deposits(e: &Env) -> Result<i128, VaultError> {
    Ok(get_state(e)?.total_deposits)
}

pub fn set_total_deposits(e: &Env, total: i128) {
    e.storage().instance().set(&DataKey::TotalDeposits, &total);
    bump_instance_ttl(e);
}

pub fn get_reward_index(e: &Env) -> Result<i128, VaultError> {
    Ok(get_state(e)?.reward_index)
}

pub fn set_reward_index(e: &Env, index: i128) {
    e.storage().instance().set(&DataKey::RewardIndex, &index);
    bump_instance_ttl(e);
}

pub fn get_vesting_period(e: &Env) -> Result<u64, VaultError> {
    Ok(get_state(e)?.vesting_period)
}

pub fn set_paused(e: &Env, paused: bool) {
    e.storage().instance().set(&DataKey::IsPaused, &paused);
    bump_instance_ttl(e);
}

pub fn set_target_deposits(e: &Env, new_target: i128) {
    e.storage()
        .instance()
        .set(&DataKey::TargetDeposits, &new_target);
    bump_instance_ttl(e);
}

pub fn set_utilization_multipliers(e: &Env, multipliers: &soroban_sdk::Vec<MultiplierPoint>) {
    e.storage()
        .instance()
        .set(&DataKey::UtilizationMultipliers, multipliers);
    bump_instance_ttl(e);
}

pub fn get_penalty_rate_bps(e: &Env) -> Result<u32, VaultError> {
    require_initialized(e)?;
    let rate = e
        .storage()
        .instance()
        .get(&DataKey::PenaltyRateBps)
        .unwrap_or(0_u32);
    bump_instance_ttl(e);
    Ok(rate)
}

pub fn set_penalty_rate_bps(e: &Env, rate_bps: u32) {
    e.storage()
        .instance()
        .set(&DataKey::PenaltyRateBps, &rate_bps);
    bump_instance_ttl(e);
}

pub fn get_total_penalties(e: &Env) -> Result<i128, VaultError> {
    require_initialized(e)?;
    let total = e
        .storage()
        .instance()
        .get(&DataKey::TotalPenalties)
        .unwrap_or(0_i128);
    bump_instance_ttl(e);
    Ok(total)
}

pub fn set_total_penalties(e: &Env, amount: i128) {
    e.storage()
        .instance()
        .set(&DataKey::TotalPenalties, &amount);
    bump_instance_ttl(e);
}

pub fn get_user_penalty_paid(e: &Env, user: &Address) -> Result<i128, VaultError> {
    require_initialized(e)?;
    let total = e
        .storage()
        .persistent()
        .get(&DataKey::UserPenaltyPaid(user.clone()))
        .unwrap_or(0_i128);
    if total != 0 {
        bump_persistent_ttl(e, &DataKey::UserPenaltyPaid(user.clone()));
    }
    Ok(total)
}

pub fn set_user_penalty_paid(e: &Env, user: &Address, amount: i128) {
    let key = DataKey::UserPenaltyPaid(user.clone());
    if amount == 0 {
        e.storage().persistent().remove(&key);
    } else {
        e.storage().persistent().set(&key, &amount);
        bump_persistent_ttl(e, &key);
    }
}

pub fn increase_penalty_totals(e: &Env, user: &Address, amount: i128) -> Result<(), VaultError> {
    let current_user = get_user_penalty_paid(e, user)?;
    let next_user = current_user
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    set_user_penalty_paid(e, user, next_user);

    let current_total = get_total_penalties(e)?;
    let next_total = current_total
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    set_total_penalties(e, next_total);
    Ok(())
}

pub fn store_early_withdraw_locked(
    e: &Env,
    user: &Address,
    amount: i128,
) -> Result<(VaultState, UserPosition, i128, i128), VaultError> {
    let state = get_state(e)?;
    let penalty_rate_bps = state.penalty_rate_bps;
    let mut position = get_user_position_unchecked(e, user)?;

    accrue_position_rewards(e, &state, &mut position)?;

    let current_timestamp = e.ledger().timestamp();
    let locks = get_user_locks_unchecked(e, user);

    let mut remaining = amount;
    let mut penalty_total: i128 = 0;
    let mut next_locks = soroban_sdk::Vec::new(e);

    for lock in locks.iter() {
        if remaining == 0 {
            next_locks.push_back(lock);
            continue;
        }

        let withdraw_amount = if lock.amount <= remaining {
            lock.amount
        } else {
            remaining
        };

        let penalty = if lock.unlock_timestamp > current_timestamp {
            withdraw_amount
                .checked_mul(penalty_rate_bps as i128)
                .ok_or(ArithmeticError::Overflow)?
                .checked_div(10000)
                .ok_or(ArithmeticError::RewardCalculationFailed)?
        } else {
            0
        };

        penalty_total = penalty_total
            .checked_add(penalty)
            .ok_or(ArithmeticError::Overflow)?;

        let remaining_lock_amount = lock
            .amount
            .checked_sub(withdraw_amount)
            .ok_or(ArithmeticError::Overflow)?;
        if remaining_lock_amount > 0 {
            next_locks.push_back(Lock {
                amount: remaining_lock_amount,
                unlock_timestamp: lock.unlock_timestamp,
                reward_multiplier: lock.reward_multiplier,
            });
        }

        remaining = remaining
            .checked_sub(withdraw_amount)
            .ok_or(ArithmeticError::Overflow)?;
    }

    if remaining != 0 {
        return Err(BalanceError::InsufficientBalance.into());
    }

    let net_amount = amount
        .checked_sub(penalty_total)
        .ok_or(ArithmeticError::Overflow)?;

    position.balance = position
        .balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    let next_total = state
        .total_deposits
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    set_user_locks(e, user, &next_locks);
    set_user_position(e, user, &position);
    set_total_deposits(e, next_total);
    increase_penalty_totals(e, user, penalty_total)?;

    Ok((
        VaultState {
            total_deposits: next_total,
            ..state
        },
        position,
        net_amount,
        penalty_total,
    ))
}

// ---------------------------------------------------------------------------
// Reentrancy Guard
// ---------------------------------------------------------------------------

pub fn enter_non_reentrant(e: &Env) -> Result<(), VaultError> {
    if e.storage()
        .instance()
        .get::<_, bool>(&DataKey::ReentrancyGuard)
        .unwrap_or(false)
    {
        return Err(AuthorizationError::ReentrancyDetected.into());
    }
    e.storage().instance().set(&DataKey::ReentrancyGuard, &true);
    bump_instance_ttl(e);
    Ok(())
}

pub fn exit_non_reentrant(e: &Env) {
    e.storage()
        .instance()
        .set(&DataKey::ReentrancyGuard, &false);
    bump_instance_ttl(e);
}

// ---------------------------------------------------------------------------
// User Position
// ---------------------------------------------------------------------------

pub fn get_user_position(e: &Env, user: &Address) -> Result<UserPosition, VaultError> {
    require_initialized(e)?;
    get_user_position_unchecked(e, user)
}

pub fn get_user_position_unchecked(e: &Env, user: &Address) -> Result<UserPosition, VaultError> {
    let liquid_balance = get_liquid_balance_unchecked(e, user);
    let locks = get_user_locks_unchecked(e, user);
    let locked_balance: i128 = locks.iter().map(|lock: Lock| lock.amount).sum();
    let total_balance = liquid_balance
        .checked_add(locked_balance)
        .ok_or(ArithmeticError::Overflow)?;

    let reward_index_key = DataKey::UserRewardIndex(user.clone());
    let accrued_rewards_key = DataKey::UserAccruedRewards(user.clone());
    let last_reward_timestamp_key = DataKey::UserLastRewardTimestamp(user.clone());
    let reward_index = e
        .storage()
        .persistent()
        .get(&reward_index_key)
        .unwrap_or(0_i128);
    let accrued_rewards = e
        .storage()
        .persistent()
        .get(&accrued_rewards_key)
        .unwrap_or(0_i128);
    let last_reward_timestamp = e
        .storage()
        .persistent()
        .get(&last_reward_timestamp_key)
        .unwrap_or(0_u64);

    if reward_index != 0 {
        bump_persistent_ttl(e, &reward_index_key);
    }
    if accrued_rewards != 0 {
        bump_persistent_ttl(e, &accrued_rewards_key);
    }
    if last_reward_timestamp != 0 {
        bump_persistent_ttl(e, &last_reward_timestamp_key);
    }

    Ok(UserPosition {
        balance: total_balance,
        reward_index,
        accrued_rewards,
        last_reward_timestamp,
    })
}

pub fn set_user_position(e: &Env, user: &Address, position: &UserPosition) {
    let reward_index_key = DataKey::UserRewardIndex(user.clone());
    let accrued_rewards_key = DataKey::UserAccruedRewards(user.clone());
    let last_reward_timestamp_key = DataKey::UserLastRewardTimestamp(user.clone());

    if position.reward_index == 0 {
        e.storage().persistent().remove(&reward_index_key);
    } else {
        e.storage()
            .persistent()
            .set(&reward_index_key, &position.reward_index);
        bump_persistent_ttl(e, &reward_index_key);
    }

    if position.accrued_rewards == 0 {
        e.storage().persistent().remove(&accrued_rewards_key);
    } else {
        e.storage()
            .persistent()
            .set(&accrued_rewards_key, &position.accrued_rewards);
        bump_persistent_ttl(e, &accrued_rewards_key);
    }

    e.storage()
        .persistent()
        .set(&last_reward_timestamp_key, &position.last_reward_timestamp);
    bump_persistent_ttl(e, &last_reward_timestamp_key);
}

pub fn get_user_balance(e: &Env, user: &Address) -> Result<i128, VaultError> {
    let position = get_user_position(e, user)?;
    Ok(position.balance)
}

pub fn get_liquid_balance(e: &Env, user: &Address) -> Result<i128, VaultError> {
    require_initialized(e)?;
    Ok(get_liquid_balance_unchecked(e, user))
}

pub fn get_liquid_balance_unchecked(e: &Env, user: &Address) -> i128 {
    let key = DataKey::UserLiquidBalance(user.clone());
    let balance = e.storage().persistent().get(&key).unwrap_or(0_i128);
    if balance != 0 {
        bump_persistent_ttl(e, &key);
    }
    balance
}

fn set_liquid_balance(e: &Env, user: &Address, amount: i128) {
    let key = DataKey::UserLiquidBalance(user.clone());
    if amount == 0 {
        e.storage().persistent().remove(&key);
    } else {
        e.storage().persistent().set(&key, &amount);
        bump_persistent_ttl(e, &key);
    }
}

pub fn get_locked_balance(e: &Env, user: &Address) -> Result<i128, VaultError> {
    require_initialized(e)?;
    let locks = get_user_locks_unchecked(e, user);
    let locked_amount: i128 = locks
        .iter()
        .filter(|l| l.unlock_timestamp > e.ledger().timestamp())
        .map(|l| l.amount)
        .sum();
    Ok(locked_amount)
}

pub fn get_user_locks_unchecked(e: &Env, user: &Address) -> soroban_sdk::Vec<Lock> {
    let key = DataKey::UserLocks(user.clone());
    let locks = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| soroban_sdk::Vec::new(e));
    if !locks.is_empty() {
        bump_persistent_ttl(e, &key);
    }
    locks
}

fn set_user_locks(e: &Env, user: &Address, locks: &soroban_sdk::Vec<Lock>) {
    let key = DataKey::UserLocks(user.clone());
    e.storage().persistent().set(&key, locks);
    if !locks.is_empty() {
        bump_persistent_ttl(e, &key);
    }
}

// ---------------------------------------------------------------------------
// Deposit/Withdraw Logic
// ---------------------------------------------------------------------------

pub fn store_deposit(
    e: &Env,
    user: &Address,
    amount: i128,
) -> Result<(VaultState, UserPosition), VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user)?;

    // Accrue rewards earned up to this point using the old balance.
    accrue_position_rewards(e, &state, &mut position)?;

    // Update total balance for reward calculation purposes in the returned position.
    position.balance = position
        .balance
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Update liquid balance and total contract deposits.
    let new_liquid_balance = get_liquid_balance_unchecked(e, user)
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = state
        .total_deposits
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;

    set_liquid_balance(e, user, new_liquid_balance);
    set_total_deposits(e, next_total);
    set_user_position(e, user, &position);

    Ok((
        VaultState {
            total_deposits: next_total,
            ..state
        },
        position,
    ))
}

pub fn store_withdraw(
    e: &Env,
    user: &Address,
    amount: i128,
) -> Result<(VaultState, UserPosition), VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user)?;

    // Accrue rewards earned up to this point using the old balance.
    accrue_position_rewards(e, &state, &mut position)?;

    // To prevent DoS, process a small batch of expired locks automatically.
    // If more locks are expired, the user must call `unlock_expired` manually.
    const WITHDRAW_UNLOCK_LIMIT: u32 = 5;
    // Process any expired locks, moving them to the liquid balance.
    unlock_expired_locks(e, user, WITHDRAW_UNLOCK_LIMIT)?;

    let liquid_balance = get_liquid_balance_unchecked(e, user);
    if liquid_balance < amount {
        return Err(BalanceError::InsufficientBalance.into());
    }

    // Update total balance for reward calculation purposes.
    position.balance = position
        .balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Update liquid balance and total contract deposits.
    let new_liquid_balance = liquid_balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = state
        .total_deposits
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    set_liquid_balance(e, user, new_liquid_balance);
    set_total_deposits(e, next_total);
    set_user_position(e, user, &position);

    Ok((
        VaultState {
            total_deposits: next_total,
            ..state
        },
        position,
    ))
}

// ---------------------------------------------------------------------------
// Lock/Unlock Logic
// ---------------------------------------------------------------------------

pub fn store_lock(e: &Env, user: &Address, amount: i128, duration: u64) -> Result<(), VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user)?;

    // Accrue rewards before changing balance distribution
    accrue_position_rewards(e, &state, &mut position)?;
    set_user_position(e, user, &position);

    let liquid_balance = get_liquid_balance_unchecked(e, user);
    if liquid_balance < amount {
        return Err(BalanceError::InsufficientBalance.into());
    }

    // Move funds from liquid to a new lock
    let new_liquid_balance = liquid_balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;
    set_liquid_balance(e, user, new_liquid_balance);

    let mut locks = get_user_locks_unchecked(e, user);
    locks.push_back(Lock {
        amount,
        unlock_timestamp: e
            .ledger()
            .timestamp()
            .checked_add(duration)
            .ok_or(ArithmeticError::Overflow)?,
        reward_multiplier: 0, // Not implemented yet
    });
    set_user_locks(e, user, &locks);

    // Note: Total balance and total_deposits do not change.
    Ok(())
}

pub fn unlock_expired_locks(e: &Env, user: &Address, limit: u32) -> Result<i128, VaultError> {
    if limit == 0 {
        return Ok(0);
    }

    let current_timestamp = e.ledger().timestamp();
    let locks = get_user_locks_unchecked(e, user);

    let mut unlocked_amount: i128 = 0;
    let mut new_locks = soroban_sdk::Vec::new(e);
    let mut processed_count = 0;

    for lock in locks.iter() {
        if lock.unlock_timestamp <= current_timestamp && processed_count < limit {
            unlocked_amount = unlocked_amount
                .checked_add(lock.amount)
                .ok_or(ArithmeticError::Overflow)?;
            processed_count += 1;
        } else {
            new_locks.push_back(lock);
        }
    }

    if unlocked_amount > 0 {
        let liquid_balance = get_liquid_balance_unchecked(e, user);
        let new_liquid_balance = liquid_balance
            .checked_add(unlocked_amount)
            .ok_or(ArithmeticError::Overflow)?;
        set_liquid_balance(e, user, new_liquid_balance);
        set_user_locks(e, user, &new_locks);
    }

    Ok(unlocked_amount)
}

// ---------------------------------------------------------------------------
// Reward Distribution
// ---------------------------------------------------------------------------

pub fn store_reward_distribution(e: &Env, amount: i128) -> Result<VaultState, VaultError> {
    let state = get_state(e)?;

    let multiplier_bps = calculate_utilization_multiplier(
        state.total_deposits,
        state.target_deposits,
        &state.utilization_multipliers,
    )?;

    // Apply the multiplier to the distributed amount.
    let effective_amount = (amount as u128)
        .checked_mul(multiplier_bps as u128)
        .ok_or(ArithmeticError::Overflow)?
        .checked_div(10000) // Convert from basis points
        .ok_or(ArithmeticError::RewardCalculationFailed)? as i128;

    let increment = checked_reward_index_increment(effective_amount, state.total_deposits)?;

    let next_reward_index = state
        .reward_index
        .checked_add(increment)
        .ok_or(ArithmeticError::Overflow)?;

    set_reward_index(e, next_reward_index);

    Ok(VaultState {
        reward_index: next_reward_index,
        ..state
    })
}

// ---------------------------------------------------------------------------
// Claim Rewards
// ---------------------------------------------------------------------------

pub fn calculate_vested_rewards(
    current_timestamp: u64,
    position: &UserPosition,
    vesting_period: u64,
) -> Result<i128, VaultError> {
    if vesting_period == 0 {
        // No vesting period, all rewards are immediately vested
        return Ok(position.accrued_rewards);
    }

    if position.last_reward_timestamp == 0 {
        return Ok(0);
    }

    let time_elapsed = current_timestamp
        .checked_sub(position.last_reward_timestamp)
        .unwrap_or(0);

    if time_elapsed >= vesting_period {
        // All rewards are vested
        Ok(position.accrued_rewards)
    } else {
        // Calculate partial vesting
        let vested = (position.accrued_rewards as u128)
            .checked_mul(time_elapsed as u128)
            .ok_or(ArithmeticError::Overflow)?
            .checked_div(vesting_period as u128)
            .ok_or(ArithmeticError::RewardCalculationFailed)? as i128;
        Ok(vested)
    }
}

pub fn store_claimable_rewards(e: &Env, user: &Address) -> Result<i128, VaultError> {
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user)?;

    // Accrue all rewards earned up to the current global index.
    accrue_position_rewards(e, &state, &mut position)?;

    // Calculate vested rewards
    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    // Update position with remaining accrued rewards
    position.accrued_rewards = position
        .accrued_rewards
        .checked_sub(vested)
        .ok_or(ArithmeticError::Overflow)?;

    set_user_position(e, user, &position);

    Ok(vested)
}

// ---------------------------------------------------------------------------
// Read-only reward preview
// ---------------------------------------------------------------------------

pub fn preview_user_rewards(e: &Env, user: &Address) -> Result<UserRewardSnapshot, VaultError> {
    require_initialized(e)?;
    let state = get_state(e)?;
    let mut position = get_user_position_unchecked(e, user)?;

    // Calculate accrued rewards without modifying state
    accrue_position_rewards(e, &state, &mut position)?;

    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    Ok(UserRewardSnapshot {
        reward_index: position.reward_index,
        rewards: position.accrued_rewards,
        vested_rewards: vested,
    })
}

pub fn pending_user_rewards_view(e: &Env, user: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_rewards(e, user)?.rewards)
}

pub fn vested_user_rewards_view(e: &Env, user: &Address) -> Result<i128, VaultError> {
    Ok(preview_user_rewards(e, user)?.vested_rewards)
}

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

pub(crate) fn checked_reward_index_increment(
    amount: i128,
    total_deposits: i128,
) -> Result<i128, VaultError> {
    if total_deposits <= 0 {
        return Err(BalanceError::NoDeposits.into());
    }

    let scaled = amount
        .checked_mul(REWARD_INDEX_SCALE)
        .ok_or(ArithmeticError::Overflow)?;
    let increment = scaled
        .checked_div(total_deposits)
        .ok_or(ArithmeticError::RewardCalculationFailed)?;

    if increment <= 0 {
        return Err(ArithmeticError::ZeroRewardIncrement.into());
    }

    Ok(increment)
}

pub(crate) fn checked_accrued_rewards(balance: i128, delta: i128) -> Result<i128, VaultError> {
    balance
        .checked_mul(delta)
        .ok_or(ArithmeticError::Overflow)?
        .checked_div(REWARD_INDEX_SCALE)
        .ok_or(ArithmeticError::RewardCalculationFailed.into())
}

fn calculate_utilization_multiplier(
    total_deposits: i128,
    target_deposits: i128,
    multipliers: &soroban_sdk::Vec<MultiplierPoint>,
) -> Result<u32, VaultError> {
    // If no target is set or no multipliers are defined, default to 1.0x.
    if target_deposits <= 0 || multipliers.is_empty() {
        return Ok(10000);
    }

    let utilization_bps = total_deposits
        .checked_mul(10000)
        .ok_or(ArithmeticError::Overflow)?
        .checked_div(target_deposits)
        .ok_or(ArithmeticError::RewardCalculationFailed)? as u32;

    // The multiplier curve is defined by points. Find the first point that
    // the current utilization is less than or equal to.
    // The list of points is expected to be sorted by `utilization_bps`.
    let mut selected_multiplier = multipliers.last().unwrap().multiplier_bps;
    for point in multipliers.iter() {
        if utilization_bps <= point.utilization_bps {
            selected_multiplier = point.multiplier_bps;
            break;
        }
    }

    Ok(selected_multiplier)
}

fn accrue_position_rewards(
    e: &Env,
    state: &VaultState,
    position: &mut UserPosition,
) -> Result<(), VaultError> {
    if state.reward_index == position.reward_index || position.balance == 0 {
        position.reward_index = state.reward_index;
        return Ok(());
    }

    if position.balance > 0 {
        let delta = state
            .reward_index
            .checked_sub(position.reward_index)
            .ok_or(ArithmeticError::Overflow)?;
        let accrued = checked_accrued_rewards(position.balance, delta)?;

        if accrued > 0 {
            position.accrued_rewards = position
                .accrued_rewards
                .checked_add(accrued)
                .ok_or(ArithmeticError::Overflow)?;
            // Update last reward timestamp whenever new rewards are accrued
            position.last_reward_timestamp = e.ledger().timestamp();
        }
    }

    position.reward_index = state.reward_index;
    Ok(())
}

fn bump_instance_ttl(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL_EXTEND_TO);
}

fn bump_persistent_ttl(e: &Env, key: &DataKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL_EXTEND_TO);
}

// ---------------------------------------------------------------------------
// Multi-Asset Support Functions
// ---------------------------------------------------------------------------

/// Add a new supported asset to the vault
pub fn add_supported_asset(e: &Env, asset: &Address) -> Result<(), VaultError> {
    let mut assets = get_supported_assets(e);
    if !assets.contains_key(asset.clone()) {
        assets.set(asset.clone(), true);
        e.storage()
            .instance()
            .set(&DataKey::SupportedAssets, &assets);

        // Initialize asset-specific state
        e.storage()
            .instance()
            .set(&DataKey::AssetTotalDeposits(asset.clone()), &0_i128);
        e.storage()
            .instance()
            .set(&DataKey::AssetRewardIndex(asset.clone()), &0_i128);

        bump_instance_ttl(e);
    }
    Ok(())
}

/// Get all supported assets
pub fn get_supported_assets(e: &Env) -> Map<Address, bool> {
    e.storage()
        .instance()
        .get(&DataKey::SupportedAssets)
        .unwrap_or(Map::new(e))
}

/// Check if an asset is supported
pub fn is_asset_supported(e: &Env, asset: &Address) -> bool {
    let assets = get_supported_assets(e);
    assets.contains_key(asset.clone())
}

/// Get total deposits for a specific asset
pub fn get_asset_total_deposits(e: &Env, asset: &Address) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(e.storage()
        .instance()
        .get(&DataKey::AssetTotalDeposits(asset.clone()))
        .unwrap_or(0_i128))
}

/// Set total deposits for a specific asset
pub fn set_asset_total_deposits(e: &Env, asset: &Address, total: i128) {
    e.storage()
        .instance()
        .set(&DataKey::AssetTotalDeposits(asset.clone()), &total);
    bump_instance_ttl(e);
}

/// Get reward index for a specific asset
pub fn get_asset_reward_index(e: &Env, asset: &Address) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(e.storage()
        .instance()
        .get(&DataKey::AssetRewardIndex(asset.clone()))
        .unwrap_or(0_i128))
}

/// Set reward index for a specific asset
pub fn set_asset_reward_index(e: &Env, asset: &Address, index: i128) {
    e.storage()
        .instance()
        .set(&DataKey::AssetRewardIndex(asset.clone()), &index);
    bump_instance_ttl(e);
}

/// Get user position for a specific asset
pub fn get_user_asset_position(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<UserPosition, VaultError> {
    require_initialized(e)?;
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }
    Ok(get_user_asset_position_unchecked(e, user, asset))
}

pub fn get_user_asset_position_unchecked(e: &Env, user: &Address, asset: &Address) -> UserPosition {
    let balance_key = DataKey::UserAssetBalance(user.clone(), asset.clone());
    let reward_index_key = DataKey::UserAssetRewardIndex(user.clone(), asset.clone());
    let accrued_rewards_key = DataKey::UserAssetAccruedRewards(user.clone(), asset.clone());
    let last_reward_timestamp_key =
        DataKey::UserAssetLastRewardTimestamp(user.clone(), asset.clone());

    let balance = e.storage().persistent().get(&balance_key).unwrap_or(0_i128);
    let reward_index = e
        .storage()
        .persistent()
        .get(&reward_index_key)
        .unwrap_or(0_i128);
    let accrued_rewards = e
        .storage()
        .persistent()
        .get(&accrued_rewards_key)
        .unwrap_or(0_i128);
    let last_reward_timestamp = e
        .storage()
        .persistent()
        .get(&last_reward_timestamp_key)
        .unwrap_or(0_u64);

    if balance != 0 {
        bump_persistent_ttl(e, &balance_key);
    }
    if reward_index != 0 {
        bump_persistent_ttl(e, &reward_index_key);
    }
    if accrued_rewards != 0 {
        bump_persistent_ttl(e, &accrued_rewards_key);
    }
    if last_reward_timestamp != 0 {
        bump_persistent_ttl(e, &last_reward_timestamp_key);
    }

    UserPosition {
        balance,
        reward_index,
        accrued_rewards,
        last_reward_timestamp,
    }
}

/// Set user position for a specific asset
pub fn set_user_asset_position(e: &Env, user: &Address, asset: &Address, position: &UserPosition) {
    let balance_key = DataKey::UserAssetBalance(user.clone(), asset.clone());
    let reward_index_key = DataKey::UserAssetRewardIndex(user.clone(), asset.clone());
    let accrued_rewards_key = DataKey::UserAssetAccruedRewards(user.clone(), asset.clone());
    let last_reward_timestamp_key =
        DataKey::UserAssetLastRewardTimestamp(user.clone(), asset.clone());

    if position.balance == 0 {
        e.storage().persistent().remove(&balance_key);
    } else {
        e.storage()
            .persistent()
            .set(&balance_key, &position.balance);
        bump_persistent_ttl(e, &balance_key);
    }

    if position.reward_index == 0 {
        e.storage().persistent().remove(&reward_index_key);
    } else {
        e.storage()
            .persistent()
            .set(&reward_index_key, &position.reward_index);
        bump_persistent_ttl(e, &reward_index_key);
    }

    if position.accrued_rewards == 0 {
        e.storage().persistent().remove(&accrued_rewards_key);
    } else {
        e.storage()
            .persistent()
            .set(&accrued_rewards_key, &position.accrued_rewards);
        bump_persistent_ttl(e, &accrued_rewards_key);
    }

    e.storage()
        .persistent()
        .set(&last_reward_timestamp_key, &position.last_reward_timestamp);
    bump_persistent_ttl(e, &last_reward_timestamp_key);
}

/// Get user balance for a specific asset
pub fn get_user_asset_balance(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<i128, VaultError> {
    Ok(get_user_asset_position(e, user, asset)?.balance)
}

/// Store deposit for a specific asset
pub fn store_asset_deposit(
    e: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<UserPosition, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }

    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    let asset_total = get_asset_total_deposits(e, asset)?;

    // Accrue rewards earned up to this point using the old balance.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = asset_total
        .checked_add(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_asset_total_deposits(e, asset, next_total);
    set_user_asset_position(e, user, asset, &position);

    Ok(position)
}

/// Store withdraw for a specific asset
pub fn store_asset_withdraw(
    e: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<UserPosition, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }

    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;
    let asset_total = get_asset_total_deposits(e, asset)?;

    // Accrue rewards earned up to this point using the old balance.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    if position.balance < amount {
        return Err(BalanceError::InsufficientBalance.into());
    }

    // Update balance and total deposits.
    position.balance = position
        .balance
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;
    let next_total = asset_total
        .checked_sub(amount)
        .ok_or(ArithmeticError::Overflow)?;

    // Persist changes.
    set_asset_total_deposits(e, asset, next_total);
    set_user_asset_position(e, user, asset, &position);

    Ok(position)
}

/// Store reward distribution for a specific asset
pub fn store_asset_reward_distribution(
    e: &Env,
    asset: &Address,
    amount: i128,
) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }

    let asset_total = get_asset_total_deposits(e, asset)?;
    let asset_reward_index = get_asset_reward_index(e, asset)?;

    let increment = checked_reward_index_increment(amount, asset_total)?;
    let next_reward_index = asset_reward_index
        .checked_add(increment)
        .ok_or(ArithmeticError::Overflow)?;

    set_asset_reward_index(e, asset, next_reward_index);

    Ok(next_reward_index)
}

/// Claim rewards for a specific asset
pub fn store_asset_claimable_rewards(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<i128, VaultError> {
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }

    let state = get_state(e)?;
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;

    // Accrue all rewards earned up to the current global index.
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    // Calculate vested rewards
    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    // Update position with remaining accrued rewards
    position.accrued_rewards = position
        .accrued_rewards
        .checked_sub(vested)
        .ok_or(ArithmeticError::Overflow)?;

    set_user_asset_position(e, user, asset, &position);

    Ok(vested)
}

/// Preview user rewards for a specific asset
pub fn preview_user_asset_rewards(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<UserRewardSnapshot, VaultError> {
    require_initialized(e)?;
    if !is_asset_supported(e, asset) {
        return Err(ValidationError::InvalidAddress.into());
    }

    let state = get_state(e)?;
    let mut position = get_user_asset_position_unchecked(e, user, asset);
    let asset_reward_index = get_asset_reward_index(e, asset)?;

    // Calculate accrued rewards without modifying state
    accrue_asset_position_rewards(e, asset_reward_index, &mut position)?;

    let current_timestamp = e.ledger().timestamp();
    let vested = calculate_vested_rewards(current_timestamp, &position, state.vesting_period)?;

    Ok(UserRewardSnapshot {
        reward_index: position.reward_index,
        rewards: position.accrued_rewards,
        vested_rewards: vested,
    })
}

pub fn pending_user_asset_rewards_view(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<i128, VaultError> {
    Ok(preview_user_asset_rewards(e, user, asset)?.rewards)
}

pub fn vested_user_asset_rewards_view(
    e: &Env,
    user: &Address,
    asset: &Address,
) -> Result<i128, VaultError> {
    Ok(preview_user_asset_rewards(e, user, asset)?.vested_rewards)
}

fn accrue_asset_position_rewards(
    e: &Env,
    asset_reward_index: i128,
    position: &mut UserPosition,
) -> Result<(), VaultError> {
    if asset_reward_index == position.reward_index || position.balance == 0 {
        position.reward_index = asset_reward_index;
        return Ok(());
    }

    if position.balance > 0 {
        let delta = asset_reward_index
            .checked_sub(position.reward_index)
            .ok_or(ArithmeticError::Overflow)?;
        let accrued = checked_accrued_rewards(position.balance, delta)?;

        if accrued > 0 {
            position.accrued_rewards = position
                .accrued_rewards
                .checked_add(accrued)
                .ok_or(ArithmeticError::Overflow)?;
            // Update last reward timestamp whenever new rewards are accrued
            position.last_reward_timestamp = e.ledger().timestamp();
        }
    }

    position.reward_index = asset_reward_index;
    Ok(())
}
