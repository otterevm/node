pub use IRecoveryGuardian::{
    IRecoveryGuardianErrors as RecoveryGuardianError,
    IRecoveryGuardianEvents as RecoveryGuardianEvent,
};

crate::sol! {
    /// RecoveryGuardian interface for social recovery of accounts
    ///
    /// This precompile enables account recovery through trusted guardians.
    /// Recovery requires threshold guardian approval and includes a timelock
    /// period before execution to allow the original owner to cancel.
    #[derive(Debug, PartialEq, Eq)]
    #[sol(abi)]
    interface IRecoveryGuardian {
        /// Recovery configuration for an account
        struct RecoveryConfig {
            uint8 threshold;           // Number of guardian approvals required
            uint64 recoveryDelay;      // Seconds to wait before recovery can execute
            address[] guardians;       // List of guardian addresses
        }

        /// Pending recovery request
        struct RecoveryRequest {
            address newOwner;          // The proposed new owner
            uint64 executeAfter;       // Timestamp after which recovery can execute
            uint8 approvalCount;       // Number of guardian approvals received
        }

        /// Initialize recovery configuration for an account
        /// Can only be called by the account itself (via root key)
        ///
        /// @param keyHash The key hash for this recovery config
        /// @param threshold Number of guardian approvals required
        /// @param recoveryDelay Seconds to wait before recovery executes
        /// @param guardians List of guardian addresses
        function initConfig(
            bytes32 keyHash,
            uint8 threshold,
            uint64 recoveryDelay,
            address[] calldata guardians
        ) external;

        /// Initiate a recovery request
        /// Called by a guardian to propose a new owner
        ///
        /// @param account The account to recover
        /// @param keyHash The recovery config keyHash
        /// @param newOwner The proposed new owner address
        function initiateRecovery(
            address account,
            bytes32 keyHash,
            address newOwner
        ) external;

        /// Approve a pending recovery request
        /// Called by guardians to approve the current recovery request
        ///
        /// @param account The account being recovered
        /// @param keyHash The recovery config keyHash
        function approveRecovery(
            address account,
            bytes32 keyHash
        ) external;

        /// Cancel a pending recovery request
        /// Can only be called by the account owner (proves they still have access)
        ///
        /// @param keyHash The recovery config keyHash
        function cancelRecovery(bytes32 keyHash) external;

        /// Execute a recovery after the timelock has passed
        /// Anyone can call this once threshold is met and delay has passed
        ///
        /// @param account The account to recover
        /// @param keyHash The recovery config keyHash
        /// @return newOwner The new owner address that was set
        function executeRecovery(
            address account,
            bytes32 keyHash
        ) external returns (address newOwner);

        /// Get the recovery configuration for an account
        ///
        /// @param account The account address
        /// @param keyHash The recovery config keyHash
        /// @return config The recovery configuration
        function getConfig(
            address account,
            bytes32 keyHash
        ) external view returns (RecoveryConfig memory config);

        /// Get a pending recovery request
        ///
        /// @param account The account address
        /// @param keyHash The recovery config keyHash
        /// @return request The pending recovery request (zero values if none)
        function getRecoveryRequest(
            address account,
            bytes32 keyHash
        ) external view returns (RecoveryRequest memory request);

        /// Check if an address has approved the current recovery request
        ///
        /// @param account The account being recovered
        /// @param keyHash The recovery config keyHash
        /// @param guardian The guardian address to check
        /// @return hasApproved True if the guardian has approved
        function hasApproved(
            address account,
            bytes32 keyHash,
            address guardian
        ) external view returns (bool hasApproved);

        /// Validate recovery authorization (implements ITempoSigner)
        /// Used by AccountKeychain to validate recovery execution
        ///
        /// @param account The account being recovered
        /// @param digest The recovery digest
        /// @param keyHash The recovery config keyHash
        /// @param signature The recovery proof (encoded guardian signatures)
        /// @return magicValue 0x1626ba7e if valid
        function isValidSignatureWithKeyHash(
            address account,
            bytes32 digest,
            bytes32 keyHash,
            bytes calldata signature
        ) external view returns (bytes4 magicValue);

        // Events
        event RecoveryConfigured(address indexed account, bytes32 indexed keyHash, uint8 threshold, uint64 delay);
        event RecoveryInitiated(address indexed account, bytes32 indexed keyHash, address indexed newOwner, uint64 executeAfter);
        event RecoveryApproved(address indexed account, bytes32 indexed keyHash, address indexed guardian);
        event RecoveryCancelled(address indexed account, bytes32 indexed keyHash);
        event RecoveryExecuted(address indexed account, bytes32 indexed keyHash, address indexed newOwner);

        // Errors
        error ConfigAlreadyExists();
        error ConfigNotFound();
        error InvalidThreshold();
        error InvalidDelay();
        error NotGuardian();
        error RecoveryAlreadyPending();
        error NoRecoveryPending();
        error AlreadyApproved();
        error ThresholdNotMet();
        error RecoveryNotReady();
        error RecoveryDelayNotPassed();
        error InvalidNewOwner();
        error UnauthorizedCaller();
    }
}

impl RecoveryGuardianError {
    pub const fn config_already_exists() -> Self {
        Self::ConfigAlreadyExists(IRecoveryGuardian::ConfigAlreadyExists {})
    }

    pub const fn config_not_found() -> Self {
        Self::ConfigNotFound(IRecoveryGuardian::ConfigNotFound {})
    }

    pub const fn invalid_threshold() -> Self {
        Self::InvalidThreshold(IRecoveryGuardian::InvalidThreshold {})
    }

    pub const fn invalid_delay() -> Self {
        Self::InvalidDelay(IRecoveryGuardian::InvalidDelay {})
    }

    pub const fn not_guardian() -> Self {
        Self::NotGuardian(IRecoveryGuardian::NotGuardian {})
    }

    pub const fn recovery_already_pending() -> Self {
        Self::RecoveryAlreadyPending(IRecoveryGuardian::RecoveryAlreadyPending {})
    }

    pub const fn no_recovery_pending() -> Self {
        Self::NoRecoveryPending(IRecoveryGuardian::NoRecoveryPending {})
    }

    pub const fn already_approved() -> Self {
        Self::AlreadyApproved(IRecoveryGuardian::AlreadyApproved {})
    }

    pub const fn threshold_not_met() -> Self {
        Self::ThresholdNotMet(IRecoveryGuardian::ThresholdNotMet {})
    }

    pub const fn recovery_not_ready() -> Self {
        Self::RecoveryNotReady(IRecoveryGuardian::RecoveryNotReady {})
    }

    pub const fn recovery_delay_not_passed() -> Self {
        Self::RecoveryDelayNotPassed(IRecoveryGuardian::RecoveryDelayNotPassed {})
    }

    pub const fn invalid_new_owner() -> Self {
        Self::InvalidNewOwner(IRecoveryGuardian::InvalidNewOwner {})
    }

    pub const fn unauthorized_caller() -> Self {
        Self::UnauthorizedCaller(IRecoveryGuardian::UnauthorizedCaller {})
    }
}
