// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable2Step.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

/// @title TempoLightClient
/// @notice Maintains finalized Tempo headers and validator BLS public key
/// @dev For MVP, uses ECDSA threshold signatures. Production would use BLS12-381.
contract TempoLightClient is Ownable2Step {
    /// @notice Domain separator for header signatures
    bytes32 public constant HEADER_DOMAIN = keccak256("TEMPO_HEADER_V1");

    /// @notice Domain separator for key rotation
    bytes32 public constant ROTATION_DOMAIN = keccak256("TEMPO_KEY_ROTATION_V1");

    /// @notice Tempo chain ID
    uint64 public immutable tempoChainId;

    /// @notice Current validator set epoch
    uint64 public currentEpoch;

    /// @notice Aggregated public key (for BLS) or threshold signers (for ECDSA MVP)
    bytes public currentPublicKey;

    /// @notice Latest finalized Tempo block height
    uint64 public latestFinalizedHeight;

    /// @notice Mapping of height to header hash
    mapping(uint64 => bytes32) public headerHashes;

    /// @notice Mapping of height to receipts root
    mapping(uint64 => bytes32) public receiptsRoots;

    /// @notice Threshold for signatures (2/3 of validators)
    uint256 public threshold;

    /// @notice Active validators for ECDSA MVP
    mapping(address => bool) public isValidator;
    address[] public validators;

    event HeaderSubmitted(uint64 indexed height, bytes32 headerHash, bytes32 receiptsRoot);
    event KeyRotated(uint64 indexed newEpoch, bytes newPublicKey);
    event ValidatorAdded(address indexed validator);
    event ValidatorRemoved(address indexed validator);

    error InvalidSignatureCount();
    error InvalidSignature();
    error HeightNotMonotonic();
    error InvalidParentHash();
    error ThresholdNotMet();
    error ValidatorExists();
    error ValidatorNotFound();

    constructor(uint64 _tempoChainId, uint64 _initialEpoch) Ownable(msg.sender) {
        tempoChainId = _tempoChainId;
        currentEpoch = _initialEpoch;
    }

    /// @notice Add a validator (owner only, for MVP)
    function addValidator(address validator) external onlyOwner {
        if (isValidator[validator]) revert ValidatorExists();
        isValidator[validator] = true;
        validators.push(validator);
        _updateThreshold();
        emit ValidatorAdded(validator);
    }

    /// @notice Remove a validator (owner only, for MVP)
    function removeValidator(address validator) external onlyOwner {
        if (!isValidator[validator]) revert ValidatorNotFound();
        isValidator[validator] = false;

        for (uint256 i = 0; i < validators.length; i++) {
            if (validators[i] == validator) {
                validators[i] = validators[validators.length - 1];
                validators.pop();
                break;
            }
        }
        _updateThreshold();
        emit ValidatorRemoved(validator);
    }

    /// @notice Submit a finalized Tempo header with validator signatures
    function submitHeader(
        uint64 height,
        bytes32 parentHash,
        bytes32 stateRoot,
        bytes32 receiptsRoot,
        uint64 epoch,
        bytes[] calldata signatures
    ) external {
        if (height <= latestFinalizedHeight && latestFinalizedHeight > 0) {
            revert HeightNotMonotonic();
        }

        if (latestFinalizedHeight > 0) {
            if (parentHash != headerHashes[latestFinalizedHeight]) {
                revert InvalidParentHash();
            }
        }

        bytes32 headerDigest = keccak256(
            abi.encodePacked(HEADER_DOMAIN, tempoChainId, height, parentHash, stateRoot, receiptsRoot, epoch)
        );

        _verifyThresholdSignatures(headerDigest, signatures);

        bytes32 headerHash = keccak256(abi.encodePacked(height, parentHash, stateRoot, receiptsRoot, epoch));
        headerHashes[height] = headerHash;
        receiptsRoots[height] = receiptsRoot;
        latestFinalizedHeight = height;

        emit HeaderSubmitted(height, headerHash, receiptsRoot);
    }

    /// @notice Submit a key rotation signed by the old validator set
    function submitKeyRotation(uint64 newEpoch, bytes calldata newPublicKey, bytes[] calldata signatures) external {
        require(newEpoch > currentEpoch, "Epoch must increase");

        bytes32 rotationDigest = keccak256(abi.encodePacked(ROTATION_DOMAIN, tempoChainId, newEpoch, newPublicKey));

        _verifyThresholdSignatures(rotationDigest, signatures);

        currentEpoch = newEpoch;
        currentPublicKey = newPublicKey;

        emit KeyRotated(newEpoch, newPublicKey);
    }

    /// @notice Get the receipts root for a height
    function getReceiptsRoot(uint64 height) external view returns (bytes32) {
        return receiptsRoots[height];
    }

    /// @notice Check if a header is finalized
    function isHeaderFinalized(uint64 height) external view returns (bool) {
        return headerHashes[height] != bytes32(0);
    }

    /// @notice Get validator count
    function validatorCount() external view returns (uint256) {
        return validators.length;
    }

    function _verifyThresholdSignatures(bytes32 digest, bytes[] calldata signatures) internal view {
        if (signatures.length < threshold) revert ThresholdNotMet();

        uint256 validCount = 0;
        address lastSigner = address(0);

        for (uint256 i = 0; i < signatures.length; i++) {
            address signer = ECDSA.recover(digest, signatures[i]);

            require(signer > lastSigner, "Signatures not sorted");
            lastSigner = signer;

            if (isValidator[signer]) {
                validCount++;
            }
        }

        if (validCount < threshold) revert ThresholdNotMet();
    }

    function _updateThreshold() internal {
        threshold = (validators.length * 2 + 2) / 3;
    }
}
