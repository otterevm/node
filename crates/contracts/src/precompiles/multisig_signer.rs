pub use IMultiSigSigner::{
    IMultiSigSignerErrors as MultiSigSignerError, IMultiSigSignerEvents as MultiSigSignerEvent,
};

crate::sol! {
    /// MultiSigSigner interface for threshold-based multisig access keys
    ///
    /// This precompile implements ITempoSigner and provides M-of-N threshold
    /// signature verification. Each account can have multiple configurations
    /// identified by keyHash.
    #[derive(Debug, PartialEq, Eq)]
    #[sol(abi)]
    interface IMultiSigSigner {
        /// Multisig configuration
        struct MultisigConfig {
            uint8 threshold;
            address[] owners;
        }

        /// Initialize a multisig configuration for a keyHash
        /// Can only be called once per (account, keyHash) pair
        ///
        /// @param keyHash The key hash identifying this configuration
        /// @param threshold Minimum number of signatures required (1 <= threshold <= owners.length)
        /// @param owners List of owner addresses that can sign
        function initConfig(
            bytes32 keyHash,
            uint8 threshold,
            address[] calldata owners
        ) external;

        /// Update the threshold for an existing configuration
        /// Requires current threshold of signatures to authorize
        ///
        /// @param keyHash The key hash identifying this configuration
        /// @param newThreshold The new threshold value
        /// @param signatures Signatures from current owners authorizing the change
        function setThreshold(
            bytes32 keyHash,
            uint8 newThreshold,
            bytes calldata signatures
        ) external;

        /// Add a new owner to the configuration
        /// Requires current threshold of signatures to authorize
        ///
        /// @param keyHash The key hash identifying this configuration
        /// @param newOwner The address of the new owner
        /// @param signatures Signatures from current owners authorizing the change
        function addOwner(
            bytes32 keyHash,
            address newOwner,
            bytes calldata signatures
        ) external;

        /// Remove an owner from the configuration
        /// Requires current threshold of signatures to authorize
        /// Threshold is automatically reduced if it exceeds new owner count
        ///
        /// @param keyHash The key hash identifying this configuration
        /// @param owner The address of the owner to remove
        /// @param signatures Signatures from current owners authorizing the change
        function removeOwner(
            bytes32 keyHash,
            address owner,
            bytes calldata signatures
        ) external;

        /// Get the configuration for an account and keyHash
        ///
        /// @param account The account address
        /// @param keyHash The key hash
        /// @return config The multisig configuration
        function getConfig(
            address account,
            bytes32 keyHash
        ) external view returns (MultisigConfig memory config);

        /// Validate signatures for contract-based access keys (implements ITempoSigner)
        ///
        /// @param account The Tempo account being authorized
        /// @param digest The bound digest to validate
        /// @param keyHash Identifies the multisig configuration
        /// @param signature ABI-encoded (address[] signers, bytes[] signatures)
        /// @return magicValue 0x1626ba7e if valid
        function isValidSignatureWithKeyHash(
            address account,
            bytes32 digest,
            bytes32 keyHash,
            bytes calldata signature
        ) external view returns (bytes4 magicValue);

        // Events
        event ConfigInitialized(address indexed account, bytes32 indexed keyHash, uint8 threshold, address[] owners);
        event ThresholdUpdated(address indexed account, bytes32 indexed keyHash, uint8 newThreshold);
        event OwnerAdded(address indexed account, bytes32 indexed keyHash, address indexed owner);
        event OwnerRemoved(address indexed account, bytes32 indexed keyHash, address indexed owner);

        // Errors
        error ConfigAlreadyExists();
        error ConfigNotFound();
        error InvalidThreshold();
        error BelowThreshold();
        error DuplicateOwner();
        error OwnerNotFound();
        error InvalidSignerOrder();
        error SignerNotOwner();
        error TooFewOwners();
    }
}

impl MultiSigSignerError {
    pub const fn config_already_exists() -> Self {
        Self::ConfigAlreadyExists(IMultiSigSigner::ConfigAlreadyExists {})
    }

    pub const fn config_not_found() -> Self {
        Self::ConfigNotFound(IMultiSigSigner::ConfigNotFound {})
    }

    pub const fn invalid_threshold() -> Self {
        Self::InvalidThreshold(IMultiSigSigner::InvalidThreshold {})
    }

    pub const fn below_threshold() -> Self {
        Self::BelowThreshold(IMultiSigSigner::BelowThreshold {})
    }

    pub const fn duplicate_owner() -> Self {
        Self::DuplicateOwner(IMultiSigSigner::DuplicateOwner {})
    }

    pub const fn owner_not_found() -> Self {
        Self::OwnerNotFound(IMultiSigSigner::OwnerNotFound {})
    }

    pub const fn invalid_signer_order() -> Self {
        Self::InvalidSignerOrder(IMultiSigSigner::InvalidSignerOrder {})
    }

    pub const fn signer_not_owner() -> Self {
        Self::SignerNotOwner(IMultiSigSigner::SignerNotOwner {})
    }

    pub const fn too_few_owners() -> Self {
        Self::TooFewOwners(IMultiSigSigner::TooFewOwners {})
    }
}
