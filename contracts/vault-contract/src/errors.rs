use soroban_sdk::contracterror;

/// Categories of errors that can occur within the vault contract.
/// Used for grouping errors in logs and UI.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    /// Errors related to user permissions and authentication.
    Authorization,
    /// Errors related to insufficient token balances.
    Balance,
    /// Errors occurring during arithmetic calculations (overflow, etc.).
    Math,
    /// Errors related to the contract's initialization or internal state.
    State,
    /// Errors related to input validation (invalid amounts, addresses, etc.).
    Validation,
}

/// Structured information about an error.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ErrorInfo {
    /// The category this error belongs to.
    pub category: ErrorCategory,
    /// A human-readable message describing the error.
    pub message: &'static str,
}

/// Specific errors related to contract state management.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StateError {
    /// Thrown when `initialize` is called on an already initialized contract.
    AlreadyInitialized,
    /// Thrown when a sensitive function is called before the contract is initialized.
    NotInitialized,
    /// Thrown when the internal state is found to be inconsistent.
    InvalidState,
    NoPendingAdmin,
}

/// Specific errors related to input validation.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValidationError {
    /// Thrown when an amount is zero but must be positive.
    InvalidAmount,
    /// Thrown when a negative amount is provided where only positive values are allowed.
    NegativeAmount,
    /// Thrown when a provided address is invalid or not authorized.
    InvalidAddress,
    /// Thrown when token addresses (deposit/reward) are misconfigured (e.g., identical).
    InvalidTokenConfiguration,
    InsufficientRewardAmount,
}

/// Specific errors related to balance checks.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BalanceError {
    /// Thrown when a user tries to withdraw more than their staked balance.
    InsufficientBalance,
    /// Thrown when the vault itself doesn't have enough reward tokens to pay out.
    InsufficientContractBalance,
    /// Thrown when reward distribution is attempted but there are no depositors.
    NoDeposits,
}

/// Specific errors related to mathematical operations.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ArithmeticError {
    /// Thrown when an addition, subtraction, or multiplication would overflow.
    Overflow,
    /// Thrown when reward calculation logic fails due to precision issues or zero division.
    RewardCalculationFailed,
    /// Thrown when a distributed reward amount is too small to increment the index.
    ZeroRewardIncrement,
}

/// Specific errors related to vesting.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum VestingError {
    /// Thrown when a user tries to claim unvested rewards.
    RewardsNotVested,
}

/// Specific errors related to authorization and security.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AuthorizationError {
    /// Thrown when a caller lacks the necessary permissions for an action.
    Unauthorized,
    /// Thrown when a reentrant call is detected by the guard.
    ReentrancyDetected,
    UpgradeFailed,
}

/// The primary error type for the Vault contract.
///
/// This enum is exposed to the Soroban runtime and mapped to specific error codes (u32).
/// It implements `From` for all sub-error types for easy conversion.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum VaultError {
    /// Vault has already been initialized
    AlreadyInitialized = 1,
    /// Vault has not been initialized
    NotInitialized = 2,
    /// Caller is not authorized to perform this action
    Unauthorized = 3,
    /// Amount must be greater than zero
    InvalidAmount = 4,
    /// Available balance is lower than the requested amount
    InsufficientBalance = 5,
    /// Arithmetic overflow or underflow detected
    MathOverflow = 6,
    /// Reward distribution requires at least one active deposit
    NoDeposits = 7,
    /// Deposit and reward token addresses must be different
    InvalidTokenConfiguration = 8,
    /// Vault token balance is lower than the requested amount
    InsufficientContractBalance = 9,
    /// Amount must not be negative
    NegativeAmount = 10,
    /// Provided address is invalid
    InvalidAddress = 11,
    /// Reward calculation failed due to checked arithmetic
    RewardCalculationFailed = 12,
    ReentrancyDetected = 13,
    /// Vault state is internally inconsistent
    InvalidState = 14,
    /// Reward distribution rounded down to zero
    ZeroRewardIncrement = 15,
    NoPendingAdmin = 16,
    /// Rewards are not yet vested
    RewardsNotVested = 17,
}

impl VaultError {
    pub const fn info(self) -> ErrorInfo {
        match self {
            Self::AlreadyInitialized => ErrorInfo {
                category: ErrorCategory::State,
                message: "vault has already been initialized",
            },
            Self::NotInitialized => ErrorInfo {
                category: ErrorCategory::State,
                message: "vault has not been initialized",
            },
            Self::Unauthorized => ErrorInfo {
                category: ErrorCategory::Authorization,
                message: "caller is not authorized to perform this action",
            },
            Self::InvalidAmount => ErrorInfo {
                category: ErrorCategory::Validation,
                message: "amount must be greater than zero",
            },
            Self::InsufficientBalance => ErrorInfo {
                category: ErrorCategory::Balance,
                message: "available balance is lower than the requested amount",
            },
            Self::MathOverflow => ErrorInfo {
                category: ErrorCategory::Math,
                message: "arithmetic overflow or underflow detected",
            },
            Self::NoDeposits => ErrorInfo {
                category: ErrorCategory::Balance,
                message: "reward distribution requires at least one active deposit",
            },
            Self::InvalidTokenConfiguration => ErrorInfo {
                category: ErrorCategory::Validation,
                message: "deposit and reward token addresses must be different",
            },
            Self::InsufficientContractBalance => ErrorInfo {
                category: ErrorCategory::Balance,
                message: "vault token balance is lower than the requested amount",
            },
            Self::NegativeAmount => ErrorInfo {
                category: ErrorCategory::Validation,
                message: "amount must not be negative",
            },
            Self::InvalidAddress => ErrorInfo {
                category: ErrorCategory::Validation,
                message: "provided address is invalid",
            },
            Self::RewardCalculationFailed => ErrorInfo {
                category: ErrorCategory::Math,
                message: "reward calculation failed due to checked arithmetic",
            },
            Self::ReentrancyDetected => ErrorInfo {
                category: ErrorCategory::Authorization,
                message: "reentrant contract call detected",
            },
            Self::InvalidState => ErrorInfo {
                category: ErrorCategory::State,
                message: "vault state is internally inconsistent",
            },
            Self::ZeroRewardIncrement => ErrorInfo {
                category: ErrorCategory::Math,
                message: "reward distribution rounded down to zero",
            },
            Self::NoPendingAdmin => ErrorInfo {
                category: ErrorCategory::State,
                message: "no pending admin transfer exists",
            },
            Self::RewardsNotVested => ErrorInfo {
                category: ErrorCategory::Validation,
                message: "rewards are not yet vested",
            },
        }
    }

    pub const fn category(self) -> ErrorCategory {
        self.info().category
    }

    pub const fn message(self) -> &'static str {
        self.info().message
    }
}

impl core::fmt::Display for VaultError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let info = self.info();
        write!(f, "VaultError::{:?}: {}", self, info.message)
    }
}

impl core::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<StateError> for VaultError {
    fn from(error: StateError) -> Self {
        match error {
            StateError::AlreadyInitialized => Self::AlreadyInitialized,
            StateError::NotInitialized => Self::NotInitialized,
            StateError::InvalidState => Self::InvalidState,
            StateError::NoPendingAdmin => Self::NoPendingAdmin,
        }
    }
}

impl From<ValidationError> for VaultError {
    fn from(error: ValidationError) -> Self {
        match error {
            ValidationError::InvalidAmount => Self::InvalidAmount,
            ValidationError::NegativeAmount => Self::NegativeAmount,
            ValidationError::InvalidAddress => Self::InvalidAddress,
            ValidationError::InvalidTokenConfiguration => Self::InvalidTokenConfiguration,
            ValidationError::InsufficientRewardAmount => Self::InsufficientRewardAmount,
        }
    }
}

impl From<BalanceError> for VaultError {
    fn from(error: BalanceError) -> Self {
        match error {
            BalanceError::InsufficientBalance => Self::InsufficientBalance,
            BalanceError::InsufficientContractBalance => Self::InsufficientContractBalance,
            BalanceError::NoDeposits => Self::NoDeposits,
        }
    }
}

impl From<ArithmeticError> for VaultError {
    fn from(error: ArithmeticError) -> Self {
        match error {
            ArithmeticError::Overflow => Self::MathOverflow,
            ArithmeticError::RewardCalculationFailed => Self::RewardCalculationFailed,
            ArithmeticError::ZeroRewardIncrement => Self::ZeroRewardIncrement,
        }
    }
}

impl From<AuthorizationError> for VaultError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Unauthorized => Self::Unauthorized,
            AuthorizationError::ReentrancyDetected => Self::ReentrancyDetected,
            AuthorizationError::UpgradeFailed => Self::UpgradeFailed,
        }
    }
}

impl From<VestingError> for VaultError {
    fn from(error: VestingError) -> Self {
        match error {
            VestingError::RewardsNotVested => Self::RewardsNotVested,
        }
    }
}
