// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/access/Ownable2Step.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/// @title StablecoinEscrow
/// @notice Escrows stablecoins for bridging to Tempo
contract StablecoinEscrow is Ownable2Step, ReentrancyGuard {
    using SafeERC20 for IERC20;

    /// @notice The Tempo light client for header verification
    address public immutable lightClient;

    /// @notice Tempo chain ID
    uint64 public immutable tempoChainId;

    /// @notice Mapping of supported tokens
    mapping(address => bool) public supportedTokens;

    /// @notice Mapping of deposit nonces per user
    mapping(address => uint64) public depositNonces;

    /// @notice Mapping of spent burn IDs (prevents replay)
    mapping(bytes32 => bool) public spentBurnIds;

    /// @notice Bridge precompile address on Tempo
    address public constant TEMPO_BRIDGE = 0xBBBB000000000000000000000000000000000000;

    /// @notice Burn event signature from Tempo bridge
    bytes32 public constant BURN_EVENT_SIGNATURE =
        keccak256("BurnInitiated(bytes32,uint64,address,address,uint64,uint64,uint64)");

    event Deposited(
        bytes32 indexed depositId,
        address indexed token,
        address indexed depositor,
        uint64 amount,
        address tempoRecipient,
        uint64 nonce
    );

    event Unlocked(bytes32 indexed burnId, address indexed token, address indexed recipient, uint64 amount);

    event TokenAdded(address indexed token);
    event TokenRemoved(address indexed token);

    error TokenNotSupported();
    error ZeroAmount();
    error InvalidRecipient();
    error BurnAlreadySpent();
    error HeaderNotFinalized();
    error InvalidReceiptProof();
    error InvalidBurnEvent();

    constructor(address _lightClient, uint64 _tempoChainId) Ownable(msg.sender) {
        lightClient = _lightClient;
        tempoChainId = _tempoChainId;
    }

    /// @notice Add a supported token
    function addToken(address token) external onlyOwner {
        supportedTokens[token] = true;
        emit TokenAdded(token);
    }

    /// @notice Remove a supported token
    function removeToken(address token) external onlyOwner {
        supportedTokens[token] = false;
        emit TokenRemoved(token);
    }

    /// @notice Deposit tokens to bridge to Tempo
    /// @param token The ERC20 token to deposit
    /// @param amount Amount in token's native decimals (will be normalized to 6)
    /// @param tempoRecipient Recipient address on Tempo
    function deposit(address token, uint256 amount, address tempoRecipient)
        external
        nonReentrant
        returns (bytes32 depositId)
    {
        if (!supportedTokens[token]) revert TokenNotSupported();
        if (amount == 0) revert ZeroAmount();
        if (tempoRecipient == address(0)) revert InvalidRecipient();

        uint64 nonce = depositNonces[msg.sender]++;

        uint64 normalizedAmount = _normalizeAmount(token, amount);

        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);

        depositId = keccak256(
            abi.encodePacked(block.chainid, address(this), token, msg.sender, normalizedAmount, tempoRecipient, nonce)
        );

        emit Deposited(depositId, token, msg.sender, normalizedAmount, tempoRecipient, nonce);
    }

    /// @notice Unlock tokens based on Tempo burn proof
    /// @param tempoHeight The Tempo block height containing the burn
    /// @param receiptRlp RLP-encoded receipt
    /// @param receiptProof MPT proof for receipt inclusion
    /// @param logIndex Index of the burn event in the receipt
    function unlock(uint64 tempoHeight, bytes calldata receiptRlp, bytes[] calldata receiptProof, uint256 logIndex)
        external
        nonReentrant
    {
        ITempoLightClient lc = ITempoLightClient(lightClient);
        if (!lc.isHeaderFinalized(tempoHeight)) revert HeaderNotFinalized();

        bytes32 receiptsRoot = lc.getReceiptsRoot(tempoHeight);

        bytes32 receiptHash = keccak256(receiptRlp);
        if (!_verifyReceiptProof(receiptHash, receiptsRoot, receiptProof)) {
            revert InvalidReceiptProof();
        }

        (bytes32 burnId, uint64 originChainId, address originToken, address originRecipient, uint64 amount) =
            _decodeBurnEvent(receiptRlp, logIndex);

        if (originChainId != uint64(block.chainid)) revert InvalidBurnEvent();

        if (spentBurnIds[burnId]) revert BurnAlreadySpent();
        spentBurnIds[burnId] = true;

        uint256 denormalizedAmount = _denormalizeAmount(originToken, amount);

        IERC20(originToken).safeTransfer(originRecipient, denormalizedAmount);

        emit Unlocked(burnId, originToken, originRecipient, amount);
    }

    /// @notice Check if a burn ID has been spent
    function isBurnSpent(bytes32 burnId) external view returns (bool) {
        return spentBurnIds[burnId];
    }

    function _normalizeAmount(address token, uint256 amount) internal view returns (uint64) {
        uint8 decimals = _getDecimals(token);
        if (decimals > 6) {
            return uint64(amount / (10 ** (decimals - 6)));
        } else if (decimals < 6) {
            return uint64(amount * (10 ** (6 - decimals)));
        }
        return uint64(amount);
    }

    function _denormalizeAmount(address token, uint64 amount) internal view returns (uint256) {
        uint8 decimals = _getDecimals(token);
        if (decimals > 6) {
            return uint256(amount) * (10 ** (decimals - 6));
        } else if (decimals < 6) {
            return uint256(amount) / (10 ** (6 - decimals));
        }
        return uint256(amount);
    }

    function _getDecimals(address token) internal view returns (uint8) {
        try IERC20Metadata(token).decimals() returns (uint8 decimals) {
            return decimals;
        } catch {
            return 18;
        }
    }

    function _verifyReceiptProof(bytes32 receiptHash, bytes32 receiptsRoot, bytes[] calldata proof)
        internal
        pure
        returns (bool)
    {
        bytes32 computedRoot = receiptHash;
        for (uint256 i = 0; i < proof.length; i++) {
            computedRoot = keccak256(abi.encodePacked(computedRoot, proof[i]));
        }
        return computedRoot == receiptsRoot;
    }

    function _decodeBurnEvent(bytes calldata receiptRlp, uint256 logIndex)
        internal
        pure
        returns (bytes32 burnId, uint64 originChainId, address originToken, address originRecipient, uint64 amount)
    {
        // Simplified event decoding for MVP
        // Production needs full RLP decoding
        // This is a placeholder that assumes the data is passed correctly
        // In production, parse receiptRlp to extract logs[logIndex]

        // Suppress unused variable warnings
        receiptRlp;
        logIndex;

        revert("Not implemented - use test mock");
    }
}

interface ITempoLightClient {
    function isHeaderFinalized(uint64 height) external view returns (bool);
    function getReceiptsRoot(uint64 height) external view returns (bytes32);
}

interface IERC20Metadata is IERC20 {
    function decimals() external view returns (uint8);
}
