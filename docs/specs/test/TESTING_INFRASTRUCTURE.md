# Testing Infrastructure Design

## Overview

This document describes the testing infrastructure needed to implement the Tempo transaction invariant tests.

---

## 1. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    TempoInvariantTest.t.sol                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  Invariants │  │  Handlers   │  │  Ghost State            │  │
│  │  invariant_ │  │  handler_   │  │  ghost_                 │  │
│  └─────────────┘  └──────┬──────┘  └─────────────────────────┘  │
│                          │                                       │
│  ┌───────────────────────┴───────────────────────────────────┐  │
│  │                    TxBuilder Library                       │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │  │
│  │  │ Legacy   │ │ EIP1559  │ │ EIP7702  │ │ Tempo (0x76) │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          │                                       │
│  ┌───────────────────────┴───────────────────────────────────┐  │
│  │                    ActorManager                            │  │
│  │  • 5-10 EOA actors with private keys                      │  │
│  │  • Access keys per actor                                   │  │
│  │  • Token balances                                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                          │                                       │
│  ┌───────────────────────┴───────────────────────────────────┐  │
│  │                    Precompile Mocks/Interfaces             │  │
│  │  • TIP20 tokens    • FeeAMM       • AccountKeychain       │  │
│  │  • NonceManager    • TIP403       • ValidatorConfig       │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Core Components

### 2.1 ActorManager

Manages test actors (accounts) with their keys and state.

```solidity
// test/helpers/ActorManager.sol

struct Actor {
    address addr;
    uint256 privateKey;
    address[] accessKeys;
    mapping(address => uint256) accessKeyPrivateKeys;
}

contract ActorManager is Test {
    Actor[] public actors;
    mapping(address => uint256) public actorIndex;
    
    uint256 constant NUM_ACTORS = 5;
    uint256 constant ACCESS_KEYS_PER_ACTOR = 3;
    
    function _initActors() internal {
        for (uint256 i = 0; i < NUM_ACTORS; i++) {
            (address addr, uint256 pk) = makeAddrAndKey(
                string(abi.encodePacked("actor", vm.toString(i)))
            );
            actors.push();
            actors[i].addr = addr;
            actors[i].privateKey = pk;
            actorIndex[addr] = i;
            
            // Create access keys for each actor
            for (uint256 j = 0; j < ACCESS_KEYS_PER_ACTOR; j++) {
                (address keyAddr, uint256 keyPk) = makeAddrAndKey(
                    string(abi.encodePacked("actor", vm.toString(i), "_key", vm.toString(j)))
                );
                actors[i].accessKeys.push(keyAddr);
                actors[i].accessKeyPrivateKeys[keyAddr] = keyPk;
            }
        }
    }
    
    function _getActor(uint256 seed) internal view returns (Actor storage) {
        return actors[seed % actors.length];
    }
    
    function _getActorKey(uint256 actorSeed, uint256 keySeed) internal view returns (address, uint256) {
        Actor storage actor = _getActor(actorSeed);
        address keyAddr = actor.accessKeys[keySeed % actor.accessKeys.length];
        return (keyAddr, actor.accessKeyPrivateKeys[keyAddr]);
    }
}
```

### 2.2 GhostState

Tracks expected state for invariant verification.

```solidity
// test/helpers/GhostState.sol

contract GhostState {
    // ============ Nonce Tracking ============
    
    /// @dev Expected protocol nonce per account
    mapping(address => uint256) public ghost_protocolNonce;
    
    /// @dev Expected 2D nonce per (account, nonceKey)
    mapping(address => mapping(uint256 => uint256)) public ghost_2dNonce;
    
    /// @dev Whether this is first use of a 2D nonce key (for gas calculation)
    mapping(address => mapping(uint256 => bool)) public ghost_2dNonceUsed;
    
    // ============ Fee Tracking ============
    
    /// @dev Total fees collected across all txs
    uint256 public ghost_totalFeesCollected;
    
    /// @dev Fees collected per validator
    mapping(address => uint256) public ghost_validatorFees;
    
    /// @dev Fee token balance per account (tracked separately from actual)
    mapping(address => uint256) public ghost_feeTokenBalance;
    
    // ============ Transaction Tracking ============
    
    uint256 public ghost_totalTxExecuted;
    uint256 public ghost_totalTxReverted;
    uint256 public ghost_totalCallsExecuted;
    uint256 public ghost_totalCreatesExecuted;
    uint256 public ghost_totalCreatesReverted;
    
    // ============ CREATE Tracking ============
    
    /// @dev Maps (caller, nonce) hash to deployed address
    mapping(bytes32 => address) public ghost_createAddresses;
    
    /// @dev Number of CREATEs per account
    mapping(address => uint256) public ghost_createCount;
    
    // ============ Access Key Tracking ============
    
    /// @dev Whether key is authorized for owner
    mapping(address => mapping(address => bool)) public ghost_keyAuthorized;
    
    /// @dev Key expiry timestamp
    mapping(address => mapping(address => uint256)) public ghost_keyExpiry;
    
    /// @dev Spending limit per (owner, key, token)
    mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpendingLimit;
    
    /// @dev Amount spent per (owner, key, token)
    mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpentAmount;
    
    // ============ Multicall Tracking ============
    
    /// @dev Number of successful batches
    uint256 public ghost_batchesSucceeded;
    
    /// @dev Number of failed batches
    uint256 public ghost_batchesFailed;
    
    /// @dev Total calls in successful batches
    uint256 public ghost_callsInSuccessfulBatches;
    
    // ============ Update Functions ============
    
    function _updateProtocolNonce(address account) internal {
        ghost_protocolNonce[account]++;
    }
    
    function _update2dNonce(address account, uint256 nonceKey) internal {
        ghost_2dNonce[account][nonceKey]++;
        ghost_2dNonceUsed[account][nonceKey] = true;
    }
    
    function _recordCreate(address caller, uint256 nonce, address deployed) internal {
        bytes32 key = keccak256(abi.encodePacked(caller, nonce));
        ghost_createAddresses[key] = deployed;
        ghost_createCount[caller]++;
    }
    
    function _authorizeKey(
        address owner,
        address key,
        uint256 expiry,
        address[] memory tokens,
        uint256[] memory limits
    ) internal {
        ghost_keyAuthorized[owner][key] = true;
        ghost_keyExpiry[owner][key] = expiry;
        for (uint256 i = 0; i < tokens.length; i++) {
            ghost_keySpendingLimit[owner][key][tokens[i]] = limits[i];
        }
    }
    
    function _revokeKey(address owner, address key) internal {
        ghost_keyAuthorized[owner][key] = false;
        ghost_keyExpiry[owner][key] = 0;
    }
}
```

### 2.3 TxBuilder Library

Builds and signs transactions of all types.

```solidity
// test/helpers/TxBuilder.sol

import {VmRlp, VmExecuteTransaction} from "tempo-std/StdVm.sol";
import {LegacyTransaction, LegacyTransactionLib} from "tempo-std/tx/LegacyTransactionLib.sol";
import {Eip1559Transaction, Eip1559TransactionLib} from "tempo-std/tx/Eip1559TransactionLib.sol";
import {TempoTransaction, TempoCall, TempoTransactionLib} from "tempo-std/tx/TempoTransactionLib.sol";

library TxBuilder {
    using LegacyTransactionLib for LegacyTransaction;
    using Eip1559TransactionLib for Eip1559Transaction;
    using TempoTransactionLib for TempoTransaction;
    
    uint64 constant CHAIN_ID = 98985;
    uint64 constant DEFAULT_GAS_LIMIT = 100_000;
    uint256 constant DEFAULT_GAS_PRICE = 100;
    
    // ============ Legacy Transactions ============
    
    function buildLegacyCall(
        VmRlp vmRlp,
        Vm vm,
        address to,
        bytes memory data,
        uint64 nonce,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        LegacyTransaction memory tx_ = LegacyTransactionLib
            .create()
            .withNonce(nonce)
            .withGasPrice(DEFAULT_GAS_PRICE)
            .withGasLimit(DEFAULT_GAS_LIMIT)
            .withTo(to)
            .withData(data);
        
        return _signLegacy(vmRlp, vm, tx_, privateKey);
    }
    
    function buildLegacyCreate(
        VmRlp vmRlp,
        Vm vm,
        bytes memory initcode,
        uint64 nonce,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        LegacyTransaction memory tx_ = LegacyTransactionLib
            .create()
            .withNonce(nonce)
            .withGasPrice(DEFAULT_GAS_PRICE)
            .withGasLimit(500_000) // Higher for CREATE
            .withData(initcode);
        // Note: withTo not called = CREATE
        
        return _signLegacy(vmRlp, vm, tx_, privateKey);
    }
    
    function _signLegacy(
        VmRlp vmRlp,
        Vm vm,
        LegacyTransaction memory tx_,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        bytes memory unsigned = tx_.encode(vmRlp);
        bytes32 hash = keccak256(unsigned);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, hash);
        return tx_.encodeWithSignature(vmRlp, v, r, s);
    }
    
    // ============ EIP-1559 Transactions ============
    
    function buildEip1559Call(
        VmRlp vmRlp,
        Vm vm,
        address to,
        bytes memory data,
        uint64 nonce,
        uint256 maxFeePerGas,
        uint256 maxPriorityFee,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        Eip1559Transaction memory tx_ = Eip1559TransactionLib
            .create()
            .withChainId(CHAIN_ID)
            .withNonce(nonce)
            .withMaxFeePerGas(maxFeePerGas)
            .withMaxPriorityFeePerGas(maxPriorityFee)
            .withGasLimit(DEFAULT_GAS_LIMIT)
            .withTo(to)
            .withData(data);
        
        bytes memory unsigned = tx_.encode(vmRlp);
        bytes32 hash = keccak256(unsigned);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, hash);
        return tx_.encodeWithSignature(vmRlp, v, r, s);
    }
    
    // ============ Tempo Transactions ============
    
    function buildTempoSingleCall(
        VmRlp vmRlp,
        Vm vm,
        address to,
        bytes memory data,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        TempoCall[] memory calls = new TempoCall[](1);
        calls[0] = TempoCall({to: to, value: 0, data: data});
        
        return _buildAndSignTempo(vmRlp, vm, calls, nonceKey, nonce, feeToken, 0, 0, privateKey);
    }
    
    function buildTempoMultiCall(
        VmRlp vmRlp,
        Vm vm,
        TempoCall[] memory calls,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        return _buildAndSignTempo(vmRlp, vm, calls, nonceKey, nonce, feeToken, 0, 0, privateKey);
    }
    
    function buildTempoTimeBound(
        VmRlp vmRlp,
        Vm vm,
        TempoCall[] memory calls,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint64 validAfter,
        uint64 validBefore,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        return _buildAndSignTempo(vmRlp, vm, calls, nonceKey, nonce, feeToken, validAfter, validBefore, privateKey);
    }
    
    function buildTempoCreate(
        VmRlp vmRlp,
        Vm vm,
        bytes memory initcode,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        TempoCall[] memory calls = new TempoCall[](1);
        calls[0] = TempoCall({
            to: address(0), // CREATE indicator
            value: 0,
            data: initcode
        });
        
        return _buildAndSignTempo(vmRlp, vm, calls, nonceKey, nonce, feeToken, 0, 0, privateKey);
    }
    
    function buildTempoCreateAndCalls(
        VmRlp vmRlp,
        Vm vm,
        bytes memory initcode,
        TempoCall[] memory additionalCalls,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        TempoCall[] memory calls = new TempoCall[](1 + additionalCalls.length);
        calls[0] = TempoCall({to: address(0), value: 0, data: initcode});
        for (uint256 i = 0; i < additionalCalls.length; i++) {
            calls[i + 1] = additionalCalls[i];
        }
        
        return _buildAndSignTempo(vmRlp, vm, calls, nonceKey, nonce, feeToken, 0, 0, privateKey);
    }
    
    function _buildAndSignTempo(
        VmRlp vmRlp,
        Vm vm,
        TempoCall[] memory calls,
        uint256 nonceKey,
        uint64 nonce,
        address feeToken,
        uint64 validAfter,
        uint64 validBefore,
        uint256 privateKey
    ) internal view returns (bytes memory) {
        TempoTransaction memory tx_ = TempoTransactionLib
            .create()
            .withChainId(CHAIN_ID)
            .withMaxFeePerGas(1e6)
            .withGasLimit(uint64(100_000 * calls.length))
            .withCalls(calls)
            .withFeeToken(feeToken)
            .withNonceKey(nonceKey)
            .withNonce(nonce);
        
        if (validAfter > 0) {
            tx_ = tx_.withValidAfter(validAfter);
        }
        if (validBefore > 0) {
            tx_ = tx_.withValidBefore(validBefore);
        }
        
        bytes memory unsigned = tx_.encode(vmRlp);
        bytes32 hash = keccak256(unsigned);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, hash);
        return tx_.encodeWithSignature(vmRlp, v, r, s);
    }
    
    // ============ CREATE Address Calculation ============
    
    function computeCreateAddress(address deployer, uint256 nonce) internal pure returns (address) {
        bytes memory rlp;
        if (nonce == 0) {
            rlp = abi.encodePacked(bytes1(0xd6), bytes1(0x94), deployer, bytes1(0x80));
        } else if (nonce <= 0x7f) {
            rlp = abi.encodePacked(bytes1(0xd6), bytes1(0x94), deployer, uint8(nonce));
        } else if (nonce <= 0xff) {
            rlp = abi.encodePacked(bytes1(0xd7), bytes1(0x94), deployer, bytes1(0x81), uint8(nonce));
        } else if (nonce <= 0xffff) {
            rlp = abi.encodePacked(bytes1(0xd8), bytes1(0x94), deployer, bytes1(0x82), uint16(nonce));
        } else {
            revert("Nonce too large");
        }
        return address(uint160(uint256(keccak256(rlp))));
    }
}
```

### 2.4 TestContracts

Simple contracts for testing CREATE and calls.

```solidity
// test/helpers/TestContracts.sol

/// @notice Simple counter contract for testing
contract Counter {
    uint256 public count;
    
    function increment() external {
        count++;
    }
    
    function incrementBy(uint256 amount) external {
        count += amount;
    }
    
    function decrement() external {
        require(count > 0, "Counter: underflow");
        count--;
    }
    
    function reset() external {
        count = 0;
    }
    
    function fail() external pure {
        revert("Counter: intentional failure");
    }
}

/// @notice Returns initcode for Counter
function getCounterInitcode() pure returns (bytes memory) {
    return type(Counter).creationCode;
}

/// @notice Contract that always reverts
contract AlwaysReverts {
    constructor() {
        revert("AlwaysReverts: constructor revert");
    }
}

/// @notice Contract that reverts on specific calls
contract SelectiveRevert {
    bool public shouldRevert;
    
    function setShouldRevert(bool _shouldRevert) external {
        shouldRevert = _shouldRevert;
    }
    
    function maybeRevert() external view {
        if (shouldRevert) {
            revert("SelectiveRevert: triggered");
        }
    }
}

/// @notice Contract that consumes gas
contract GasConsumer {
    uint256[] public data;
    
    function consumeGas(uint256 iterations) external {
        for (uint256 i = 0; i < iterations; i++) {
            data.push(i);
        }
    }
}
```

---

## 3. Handler Design

### 3.1 Core Handlers

```solidity
// ============ CALL Handlers ============

/// @notice Execute a simple token transfer
function handler_transfer(
    uint256 actorSeed,
    uint256 recipientSeed,
    uint256 amount
) external {
    Actor storage sender = _getActor(actorSeed);
    Actor storage recipient = _getActor(recipientSeed);
    if (sender.addr == recipient.addr) return;
    
    amount = bound(amount, 1e6, 100e6);
    if (feeToken.balanceOf(sender.addr) < amount) return;
    
    uint64 nonce = uint64(ghost_protocolNonce[sender.addr]);
    bytes memory signedTx = TxBuilder.buildLegacyCall(
        vmRlp, vm,
        address(feeToken),
        abi.encodeCall(ITIP20.transfer, (recipient.addr, amount)),
        nonce,
        sender.privateKey
    );
    
    _executeAndTrack(sender.addr, signedTx, false, 0);
}

/// @notice Execute transfer with 2D nonce
function handler_transfer2dNonce(
    uint256 actorSeed,
    uint256 recipientSeed,
    uint256 amount,
    uint256 nonceKey
) external {
    // TODO: Requires Tempo tx type
    nonceKey = bound(nonceKey, 1, 100); // Ensure > 0 for 2D nonce
    // ...
}

// ============ CREATE Handlers ============

/// @notice Deploy a simple contract
function handler_create(uint256 actorSeed) external {
    Actor storage deployer = _getActor(actorSeed);
    
    uint64 nonce = uint64(ghost_protocolNonce[deployer.addr]);
    address expectedAddr = TxBuilder.computeCreateAddress(deployer.addr, nonce);
    
    bytes memory signedTx = TxBuilder.buildLegacyCreate(
        vmRlp, vm,
        getCounterInitcode(),
        nonce,
        deployer.privateKey
    );
    
    bool success = _executeAndTrack(deployer.addr, signedTx, true, nonce);
    
    if (success) {
        ghost_createAddresses[keccak256(abi.encodePacked(deployer.addr, nonce))] = expectedAddr;
        ghost_totalCreatesExecuted++;
    } else {
        ghost_totalCreatesReverted++;
    }
}

/// @notice Deploy contract that reverts in constructor
function handler_createReverting(uint256 actorSeed) external {
    Actor storage deployer = _getActor(actorSeed);
    
    uint64 nonce = uint64(ghost_protocolNonce[deployer.addr]);
    
    bytes memory signedTx = TxBuilder.buildLegacyCreate(
        vmRlp, vm,
        type(AlwaysReverts).creationCode,
        nonce,
        deployer.privateKey
    );
    
    _executeAndTrack(deployer.addr, signedTx, true, nonce);
    ghost_totalCreatesReverted++;
}

// ============ Multicall Handlers ============

/// @notice Execute batch of transfers
function handler_batchTransfer(
    uint256 actorSeed,
    uint256 callCount,
    uint256 amountSeed
) external {
    // TODO: Requires Tempo tx type
    callCount = bound(callCount, 2, 5);
    // ...
}

/// @notice Execute batch where one call reverts
function handler_batchWithRevert(
    uint256 actorSeed,
    uint256 revertIndex
) external {
    // TODO: Requires Tempo tx type
    // Build batch where call at revertIndex calls SelectiveRevert.maybeRevert()
}

// ============ Access Key Handlers ============

/// @notice Authorize a new access key
function handler_authorizeKey(
    uint256 ownerSeed,
    uint256 keySeed,
    uint256 expirySeed,
    uint256 limitSeed
) external {
    Actor storage owner = _getActor(ownerSeed);
    (address keyAddr,) = _getActorKey(ownerSeed, keySeed);
    
    uint256 expiry = block.timestamp + bound(expirySeed, 1 hours, 30 days);
    uint256 limit = bound(limitSeed, 100e6, 10_000e6);
    
    // Call keychain.authorizeKey()
    vm.prank(owner.addr);
    try keychain.authorizeKey(
        keyAddr,
        IAccountKeychain.SignatureType.Secp256k1,
        uint64(expiry),
        true,
        _createTokenLimits(limit)
    ) {
        address[] memory tokens = new address[](1);
        tokens[0] = address(feeToken);
        uint256[] memory limits = new uint256[](1);
        limits[0] = limit;
        _authorizeKey(owner.addr, keyAddr, expiry, tokens, limits);
    } catch {}
}

/// @notice Use access key to sign transaction
function handler_useAccessKey(
    uint256 ownerSeed,
    uint256 keySeed,
    uint256 recipientSeed,
    uint256 amount
) external {
    // TODO: Requires Tempo tx type with keyAuthorization
}

/// @notice Revoke an access key
function handler_revokeKey(
    uint256 ownerSeed,
    uint256 keySeed
) external {
    Actor storage owner = _getActor(ownerSeed);
    (address keyAddr,) = _getActorKey(ownerSeed, keySeed);
    
    if (!ghost_keyAuthorized[owner.addr][keyAddr]) return;
    
    vm.prank(owner.addr);
    try keychain.revokeKey(keyAddr) {
        _revokeKey(owner.addr, keyAddr);
    } catch {}
}

// ============ Time-Bound Handlers ============

/// @notice Execute transaction with time bounds
function handler_timeBoundTx(
    uint256 actorSeed,
    uint256 validAfterOffset,
    uint256 validBeforeOffset
) external {
    // TODO: Requires Tempo tx type
    // validAfter = block.timestamp - bound(validAfterOffset, 0, 1 hours)
    // validBefore = block.timestamp + bound(validBeforeOffset, 1 minutes, 1 hours)
}

/// @notice Execute transaction that should fail due to time
function handler_expiredTx(uint256 actorSeed) external {
    // TODO: validBefore = block.timestamp - 1
}

// ============ Execution Helper ============

function _executeAndTrack(
    address sender,
    bytes memory signedTx,
    bool isCreate,
    uint256 createNonce
) internal returns (bool success) {
    vm.coinbase(validator);
    
    try vmExec.executeTransaction(signedTx) {
        ghost_protocolNonce[sender]++;
        ghost_totalTxExecuted++;
        return true;
    } catch {
        // CREATE failure still burns nonce
        if (isCreate) {
            ghost_protocolNonce[sender]++;
        }
        ghost_totalTxReverted++;
        return false;
    }
}
```

---

## 4. Invariant Functions

```solidity
// ============ Nonce Invariants ============

function invariant_protocolNonceMatchesActual() public view {
    for (uint256 i = 0; i < actors.length; i++) {
        address actor = actors[i].addr;
        assertEq(
            vm.getNonce(actor),
            ghost_protocolNonce[actor],
            "Protocol nonce mismatch"
        );
    }
}

function invariant_2dNonceMatchesActual() public view {
    for (uint256 i = 0; i < actors.length; i++) {
        address actor = actors[i].addr;
        // Check commonly used nonce keys
        for (uint256 key = 1; key <= 10; key++) {
            if (ghost_2dNonceUsed[actor][key]) {
                assertEq(
                    nonce.getNonce(actor, key),
                    ghost_2dNonce[actor][key],
                    "2D nonce mismatch"
                );
            }
        }
    }
}

function invariant_createAddressCorrect() public view {
    for (uint256 i = 0; i < actors.length; i++) {
        address actor = actors[i].addr;
        for (uint256 n = 0; n < ghost_createCount[actor]; n++) {
            bytes32 key = keccak256(abi.encodePacked(actor, n));
            address expected = ghost_createAddresses[key];
            if (expected != address(0)) {
                address computed = TxBuilder.computeCreateAddress(actor, n);
                assertEq(expected, computed, "CREATE address mismatch");
            }
        }
    }
}

// ============ Fee Invariants ============

function invariant_feesAlwaysCollected() public view {
    // Sum of all validator fees should equal total collected
    uint256 totalValidatorFees = 0;
    // ... iterate validators
    assertEq(totalValidatorFees, ghost_totalFeesCollected, "Fee accounting mismatch");
}

// ============ Multicall Invariants ============

function invariant_batchAtomicity() public view {
    // All calls in successful batches should have executed
    // This is tracked by ghost_callsInSuccessfulBatches
}

// ============ Access Key Invariants ============

function invariant_keyAuthorizationConsistent() public view {
    for (uint256 i = 0; i < actors.length; i++) {
        address owner = actors[i].addr;
        for (uint256 j = 0; j < actors[i].accessKeys.length; j++) {
            address key = actors[i].accessKeys[j];
            
            bool ghostAuth = ghost_keyAuthorized[owner][key];
            
            try keychain.getKey(owner, key) returns (IAccountKeychain.KeyInfo memory info) {
                if (ghostAuth && info.expiry > block.timestamp) {
                    assertTrue(true, "Key should be authorized");
                }
            } catch {}
        }
    }
}

// ============ Counting Invariants ============

function invariant_txCountingConsistent() public view {
    uint256 sumOfNonces = 0;
    for (uint256 i = 0; i < actors.length; i++) {
        sumOfNonces += ghost_protocolNonce[actors[i].addr];
    }
    assertEq(
        sumOfNonces,
        ghost_totalTxExecuted + ghost_totalCreatesReverted,
        "Tx counting mismatch"
    );
}
```

---

## 5. File Structure

```
test/
├── invariants/
│   ├── TempoTransactionInvariant.t.sol    # Main invariant test
│   ├── NonceInvariant.t.sol               # Nonce-focused tests
│   ├── FeeInvariant.t.sol                 # Fee-focused tests
│   ├── MulticallInvariant.t.sol           # Multicall-focused tests
│   ├── CreateInvariant.t.sol              # CREATE-focused tests
│   └── AccessKeyInvariant.t.sol           # Access key-focused tests
│
├── helpers/
│   ├── ActorManager.sol                   # Actor/account management
│   ├── GhostState.sol                     # Ghost variable tracking
│   ├── TxBuilder.sol                      # Transaction building library
│   ├── TestContracts.sol                  # Simple test contracts
│   └── InvariantBase.sol                  # Base contract combining all helpers
│
├── INVARIANT_TEST_PLAN.md                 # Test plan document
└── TESTING_INFRASTRUCTURE.md              # This document
```

---

## 6. InvariantBase Contract

Combines all helpers into a single base contract:

```solidity
// test/helpers/InvariantBase.sol

import {BaseTest} from "../BaseTest.t.sol";
import {ActorManager} from "./ActorManager.sol";
import {GhostState} from "./GhostState.sol";
import {TxBuilder} from "./TxBuilder.sol";

abstract contract InvariantBase is BaseTest, ActorManager, GhostState {
    using TxBuilder for *;
    
    VmRlp internal vmRlp = VmRlp(address(vm));
    VmExecuteTransaction internal vmExec = VmExecuteTransaction(address(vm));
    
    TIP20 public feeToken;
    address public validator;
    
    function setUp() public virtual override {
        super.setUp();
        
        // Initialize fee token
        feeToken = TIP20(
            factory.createToken("Fee Token", "FEE", "USD", pathUSD, admin, bytes32("feetoken"))
        );
        
        // Initialize actors
        _initActors();
        
        // Fund actors
        vm.startPrank(admin);
        feeToken.grantRole(_ISSUER_ROLE, admin);
        for (uint256 i = 0; i < actors.length; i++) {
            feeToken.mint(actors[i].addr, 100_000_000e6);
        }
        vm.stopPrank();
        
        // Setup validator
        validator = makeAddr("validator");
        
        // Setup AMM liquidity
        _setupAmmLiquidity();
        
        // Target this contract for fuzzing
        targetContract(address(this));
    }
    
    function _setupAmmLiquidity() internal {
        vm.startPrank(admin);
        feeToken.mint(address(amm), 100_000_000e6);
        pathUSD.mint(address(amm), 100_000_000e6);
        feeToken.approve(address(amm), type(uint256).max);
        pathUSD.approve(address(amm), type(uint256).max);
        amm.mint(address(feeToken), address(pathUSD), 50_000_000e6, admin);
        vm.stopPrank();
        
        vm.prank(validator);
        amm.setValidatorToken(address(feeToken));
    }
}
```

---

## 7. Implementation Priority

1. **Core Infrastructure** (Week 1)
   - ActorManager
   - GhostState  
   - TxBuilder (Legacy only initially)
   - InvariantBase

2. **Nonce Handlers** (Week 1)
   - handler_transfer
   - handler_create
   - handler_createReverting
   - invariant_protocolNonceMatchesActual

3. **CREATE Handlers** (Week 2)
   - handler_createAndCall (needs Tempo tx)
   - invariant_createAddressCorrect
   - TestContracts

4. **Multicall Handlers** (Week 2)
   - handler_batchTransfer
   - handler_batchWithRevert
   - invariant_batchAtomicity

5. **Access Key Handlers** (Week 3)
   - handler_authorizeKey
   - handler_useAccessKey
   - handler_revokeKey
   - invariant_keyAuthorizationConsistent

6. **Fee Handlers** (Week 3)
   - handler_insufficientBalance
   - invariant_feesAlwaysCollected
