// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

/// @title GhostState - Ghost Variable Tracking for Invariant Tests
/// @dev Ghost variables mirror what we expect on-chain state to be
abstract contract GhostState {
    // ============ Nonce Tracking ============

    mapping(address => uint256) public ghost_protocolNonce;
    mapping(address => mapping(uint256 => uint256)) public ghost_2dNonce;
    mapping(address => mapping(uint256 => bool)) public ghost_2dNonceUsed;

    // ============ Transaction Tracking ============

    uint256 public ghost_totalTxExecuted;
    uint256 public ghost_totalTxReverted;
    uint256 public ghost_totalCallsExecuted;
    uint256 public ghost_totalCreatesExecuted;
    uint256 public ghost_totalProtocolNonceTxs;
    uint256 public ghost_total2dNonceTxs;

    // ============ CREATE Tracking ============

    mapping(bytes32 => address) public ghost_createAddresses;
    mapping(address => uint256) public ghost_createCount;

    // ============ Fee Tracking ============

    mapping(address => uint256) public ghost_feeTokenBalance;

    // ============ Access Key Tracking ============

    mapping(address => mapping(address => bool)) public ghost_keyAuthorized;
    mapping(address => mapping(address => uint256)) public ghost_keyExpiry;
    mapping(address => mapping(address => bool)) public ghost_keyEnforceLimits;
    mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpendingLimit;
    mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpentAmount;

    // ============ Update Functions ============

    function _updateProtocolNonce(address account) internal {
        ghost_protocolNonce[account]++;
    }

    function _update2dNonce(address account, uint256 nonceKey) internal {
        ghost_2dNonce[account][nonceKey]++;
        ghost_2dNonceUsed[account][nonceKey] = true;
    }

    function _recordTxSuccess() internal {
        ghost_totalTxExecuted++;
    }

    function _recordTxRevert() internal {
        ghost_totalTxReverted++;
    }

    function _recordCallSuccess() internal {
        ghost_totalCallsExecuted++;
    }

    function _recordCreateSuccess(address caller, uint256 protocolNonce, address deployed) internal {
        bytes32 key = keccak256(abi.encodePacked(caller, protocolNonce));
        ghost_createAddresses[key] = deployed;
        ghost_createCount[caller]++;
        ghost_totalCreatesExecuted++;
    }

    function _authorizeKey(
        address owner,
        address keyId,
        uint256 expiry,
        bool enforceLimits,
        address[] memory tokens,
        uint256[] memory limits
    ) internal {
        ghost_keyAuthorized[owner][keyId] = true;
        ghost_keyExpiry[owner][keyId] = expiry;
        ghost_keyEnforceLimits[owner][keyId] = enforceLimits;
        for (uint256 i = 0; i < tokens.length; i++) {
            ghost_keySpendingLimit[owner][keyId][tokens[i]] = limits[i];
        }
    }

    function _revokeKey(address owner, address keyId) internal {
        ghost_keyAuthorized[owner][keyId] = false;
        ghost_keyExpiry[owner][keyId] = 0;
    }

    function _recordKeySpending(address owner, address keyId, address token, uint256 amount) internal {
        ghost_keySpentAmount[owner][keyId][token] += amount;
    }
}
