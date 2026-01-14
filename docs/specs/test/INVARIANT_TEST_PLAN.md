# Tempo Transaction Invariant Test Plan

## Overview

This document outlines all invariants that must be tested for the Tempo transaction handler using `vm.executeTransaction`. The tests verify that the handler correctly enforces protocol rules across random sequences of operations.

**Test File:** [TempoTransactionInvariant.t.sol](./TempoTransactionInvariant.t.sol)

---

## 1. Nonce Invariants

### 1.1 Protocol Nonce (nonce_key = 0)

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| N1 | `protocol_nonce_monotonic` | ✅ Implemented | `invariant_N1_protocolNonceMonotonic` |
| N2 | `protocol_nonce_bumps_on_call` | ✅ Implemented | `invariant_N2_protocolNonceMatchesExpected` |
| N3 | `protocol_nonce_bumps_on_create_success` | ✅ Implemented | `invariant_N3_protocolNonceTxsBumpNonce` |
| N4 | `protocol_nonce_bumps_on_create_failure` | ✅ Implemented | `handler_createReverting` |
| N5 | `create_address_uses_protocol_nonce` | ✅ Implemented | `invariant_N5_createAddressUsesProtocolNonce` |

### 1.2 2D Nonces (nonce_key > 0)

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| N6 | `2d_nonce_independent` | ✅ Implemented | `invariant_N6_2dNonceIndependent`, `handler_multipleNonceKeys` |
| N7 | `2d_nonce_monotonic` | ✅ Implemented | `invariant_N7_2dNonceMonotonic` |
| N8 | `2d_nonce_no_protocol_effect` | ✅ Implemented | `invariant_N8_2dNonceNoProtocolEffect` |
| N9 | `2d_nonce_create_still_uses_protocol` | ⏳ TODO | - |
| N10 | `2d_nonce_gas_cold` | ⏳ TODO | - |
| N11 | `2d_nonce_gas_warm` | ⏳ TODO | - |

### 1.3 Replay Protection

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| N12 | `replay_fails_protocol` | ⏳ TODO | - |
| N13 | `replay_fails_2d` | ⏳ TODO | - |
| N14 | `wrong_nonce_too_high` | ⏳ TODO | - |
| N15 | `wrong_nonce_too_low` | ⏳ TODO | - |

---

## 2. Fee Invariants

### 2.1 Fee Collection

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| F1 | `fee_precollected` | ⏳ TODO | - |
| F2 | `fee_equals_gas_times_price` | ⏳ TODO | - |
| F3 | `fee_refund_on_success` | ⏳ TODO | - |
| F4 | `fee_no_refund_on_failure` | ⏳ TODO | - |
| F5 | `fee_paid_even_on_revert` | ⏳ TODO | - |

### 2.2 Fee Token Validation

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| F6 | `fee_token_must_be_tip20` | ⏳ TODO | - |
| F7 | `fee_token_from_tx` | ⏳ TODO | - |
| F8 | `fee_token_fallback` | ⏳ TODO | - |
| F9 | `insufficient_balance_rejected` | ✅ Implemented | `invariant_F9_balanceSumConsistent`, `handler_insufficientBalanceTransfer` |
| F10 | `insufficient_liquidity_rejected` | ⏳ TODO | - |

### 2.3 Subblock Transactions

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| F11 | `subblock_no_fees` | ⏳ TODO | - |
| F12 | `subblock_no_keychain` | ⏳ TODO | - |

---

## 3. Multicall Batch Invariants

### 3.1 Atomicity

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| M1 | `batch_all_or_nothing` | ⏳ TODO | - |
| M2 | `batch_partial_state_reverted` | ⏳ TODO | - |
| M3 | `batch_logs_cleared_on_failure` | ⏳ TODO | - |
| M4 | `batch_logs_preserved_on_success` | ⏳ TODO | - |

### 3.2 Gas Accounting

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| M5 | `batch_gas_accumulated` | ⏳ TODO | - |
| M6 | `batch_intrinsic_per_call` | ⏳ TODO | - |
| M7 | `batch_gas_limit_shared` | ⏳ TODO | - |

### 3.3 State Visibility

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| M8 | `batch_state_visible` | ⏳ TODO | - |
| M9 | `batch_balance_visible` | ⏳ TODO | - |

---

## 4. CREATE Invariants

### 4.1 Structure Rules

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| C1 | `create_must_be_first` | ⏳ TODO | - |
| C2 | `create_max_one` | ⏳ TODO | - |
| C3 | `create_no_auth_list` | ⏳ TODO | - |
| C4 | `create_no_value` | ⏳ TODO | - |

### 4.2 Address Derivation

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| C5 | `create_address_deterministic` | ✅ Implemented | `invariant_C5_createAddressDeterministic` |
| C6 | `create_address_uses_pre_nonce` | ✅ Implemented | `handler_create` |
| C7 | `create_nonce_burned_on_failure` | ✅ Implemented | `handler_createReverting` |

### 4.3 Initcode Validation

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| C8 | `create_initcode_size_limit` | ⏳ TODO | - |
| C9 | `create_initcode_gas` | ⏳ TODO | - |

---

## 5. Access Key / Session Key Invariants

### 5.1 KeyAuthorization

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| K1 | `key_auth_signed_by_root` | ⏳ TODO | - |
| K2 | `key_auth_self_only` | ⏳ TODO | - |
| K3 | `key_auth_chain_id_match` | ⏳ TODO | - |
| K4 | `key_auth_not_expired` | ✅ Implemented | `handler_authorizeKey`, `handler_useAccessKey` |

### 5.2 Keychain Signature

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| K5 | `keychain_key_must_exist` | ✅ Implemented | `invariant_K5_keyAuthorizationConsistent` |
| K6 | `keychain_same_tx_allowed` | ⏳ TODO | - |
| K7 | `keychain_expired_rejected` | ✅ Implemented | `handler_useAccessKey` (skips expired keys) |
| K8 | `keychain_revoked_rejected` | ✅ Implemented | `handler_revokeKey`, `handler_useAccessKey` (skips revoked) |

### 5.3 Spending Limits

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| K9 | `spending_limit_enforced` | ✅ Implemented | `invariant_K9_spendingLimitEnforced` |
| K10 | `spending_limit_per_period` | ⏳ TODO | - |
| K11 | `spending_limit_none_unlimited` | ⏳ TODO | - |
| K12 | `spending_limit_empty_zero` | ⏳ TODO | - |

### 5.4 Signature Types

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| K13 | `sig_secp256k1_valid` | ✅ Implemented | All handlers support Secp256k1 |
| K14 | `sig_p256_valid` | ✅ Implemented | All handlers support P256 via `_getRandomSignatureType` |
| K15 | `sig_webauthn_valid` | ✅ Implemented | All handlers support WebAuthn via `_getRandomSignatureType` |
| K16 | `sig_wrong_type_rejected` | ⏳ TODO | - |

---

## 6. Time Window Invariants

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| T1 | `valid_after_enforced` | ⏳ TODO | - |
| T2 | `valid_before_enforced` | ⏳ TODO | - |
| T3 | `time_window_both` | ⏳ TODO | - |
| T4 | `time_window_open` | ⏳ TODO | - |

---

## 7. Transaction Type Invariants

### 7.1 Legacy Transactions

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| TX1 | `legacy_single_call` | ✅ Implemented | `handler_transfer`, `handler_create` |
| TX2 | `legacy_protocol_nonce` | ✅ Implemented | All legacy handlers |
| TX3 | `legacy_ecdsa_only` | ✅ Implemented | Secp256k1, P256, WebAuthn all tested |

### 7.2 EIP-1559 Transactions

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| TX4 | `eip1559_priority_fee` | ⏳ TODO | - |
| TX5 | `eip1559_base_fee` | ⏳ TODO | - |

### 7.3 EIP-7702 Transactions

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| TX6 | `eip7702_auth_applied` | ⏳ TODO | - |
| TX7 | `eip7702_no_create` | ⏳ TODO | - |

### 7.4 Tempo (0x76) Transactions

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| TX8 | `tempo_multicall` | ⏳ TODO | - |
| TX9 | `tempo_2d_nonce` | ✅ Implemented | `handler_tempoTransfer` |
| TX10 | `tempo_fee_sponsorship` | ⏳ TODO | - |
| TX11 | `tempo_access_keys` | ✅ Implemented | `handler_tempoUseAccessKey`, `handler_tempoUseP256AccessKey` |
| TX12 | `tempo_time_windows` | ⏳ TODO | - |
| TX13 | `tempo_no_value` | ✅ Implemented | All Tempo calls use `value: 0` |

---

## 8. Gas Invariants

### 8.1 Intrinsic Gas

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| G1 | `intrinsic_base_21k` | ⏳ TODO | - |
| G2 | `intrinsic_per_call` | ⏳ TODO | - |
| G3 | `intrinsic_calldata` | ⏳ TODO | - |
| G4 | `intrinsic_create` | ⏳ TODO | - |
| G5 | `intrinsic_access_list` | ⏳ TODO | - |

### 8.2 Signature Gas

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| G6 | `sig_gas_secp256k1` | ⏳ TODO | - |
| G7 | `sig_gas_p256` | ⏳ TODO | - |
| G8 | `sig_gas_webauthn` | ⏳ TODO | - |

### 8.3 KeyAuthorization Gas

| ID | Invariant | Status | Handler(s) |
|----|-----------|--------|------------|
| G9 | `key_auth_base_gas` | ⏳ TODO | - |
| G10 | `key_auth_per_limit` | ⏳ TODO | - |

---

## 9. Ghost Variables

**File:** [helpers/GhostState.sol](./helpers/GhostState.sol)

```solidity
// Nonce Tracking
mapping(address => uint256) public ghost_protocolNonce;
mapping(address => mapping(uint256 => uint256)) public ghost_2dNonce;
mapping(address => mapping(uint256 => bool)) public ghost_2dNonceUsed;

// Transaction Tracking
uint256 public ghost_totalTxExecuted;
uint256 public ghost_totalTxReverted;
uint256 public ghost_totalCallsExecuted;
uint256 public ghost_totalCreatesExecuted;
uint256 public ghost_totalProtocolNonceTxs;
uint256 public ghost_total2dNonceTxs;

// CREATE Tracking
mapping(bytes32 => address) public ghost_createAddresses; // hash(caller, nonce) => deployed
mapping(address => uint256) public ghost_createCount;

// Fee Tracking
mapping(address => uint256) public ghost_feeTokenBalance;

// Access Key Tracking
mapping(address => mapping(address => bool)) public ghost_keyAuthorized;
mapping(address => mapping(address => uint256)) public ghost_keyExpiry;
mapping(address => mapping(address => bool)) public ghost_keyEnforceLimits;
mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpendingLimit;
mapping(address => mapping(address => mapping(address => uint256))) public ghost_keySpentAmount;
```

**Additional State in Test Contract:**

```solidity
// Per-handler previous nonce tracking (for monotonicity checks)
mapping(address => uint256) public ghost_previousProtocolNonce;
mapping(address => mapping(uint256 => uint256)) public ghost_previous2dNonce;
```

---

## 10. Handler Functions

**File:** [TempoTransactionInvariant.t.sol](./TempoTransactionInvariant.t.sol)

### Legacy Transaction Handlers

| Handler | Description | Invariants Tested |
|---------|-------------|-------------------|
| `handler_transfer` | Random transfer with random sig type | N1, N2, TX1-TX3, K13-K15 |
| `handler_sequentialTransfers` | Multiple sequential transfers | N1, N2 |
| `handler_create` | Deploy contract with random sig type | N3, C5-C6, K13-K15 |
| `handler_createReverting` | Deploy reverting contract | N4, C7 |

### 2D Nonce Handlers

| Handler | Description | Invariants Tested |
|---------|-------------|-------------------|
| `handler_2dNonceIncrement` | Increment single 2D nonce key | N6, N7 |
| `handler_multipleNonceKeys` | Increment multiple keys for same actor | N6 |

### Tempo Transaction Handlers

| Handler | Description | Invariants Tested |
|---------|-------------|-------------------|
| `handler_tempoTransfer` | Tempo transfer with 2D nonce | TX9, N6-N8 |
| `handler_tempoTransferProtocolNonce` | Tempo transfer with protocol nonce | N1-N3 |
| `handler_tempoUseAccessKey` | Tempo tx signed by secp256k1 access key | K5, K9, TX11 |
| `handler_tempoUseP256AccessKey` | Tempo tx signed by P256 access key | K5, K9, TX11, K14 |

### Access Key Handlers

| Handler | Description | Invariants Tested |
|---------|-------------|-------------------|
| `handler_authorizeKey` | Authorize secp256k1 or P256 access key | K1-K4 |
| `handler_revokeKey` | Revoke an access key | K7-K8 |
| `handler_useAccessKey` | Use access key for legacy transfer | K5, K9 |

### Failure Handlers

| Handler | Description | Invariants Tested |
|---------|-------------|-------------------|
| `handler_insufficientBalanceTransfer` | Transfer exceeding balance | F9 |

---

## 11. Invariant Functions

| Function | Invariant IDs | Description |
|----------|---------------|-------------|
| `invariant_N1_protocolNonceMonotonic` | N1 | Protocol nonce never decreases |
| `invariant_N2_protocolNonceMatchesExpected` | N2 | On-chain nonce matches ghost state |
| `invariant_N3_protocolNonceTxsBumpNonce` | N3 | Sum of nonces equals protocol tx count |
| `invariant_N5_createAddressUsesProtocolNonce` | N5 | CREATE uses correct nonce for address |
| `invariant_N6_2dNonceIndependent` | N6 | 2D nonce keys are independent |
| `invariant_N7_2dNonceMonotonic` | N7 | 2D nonces never decrease |
| `invariant_N8_2dNonceNoProtocolEffect` | N8 | 2D nonces don't affect protocol nonce |
| `invariant_2dNonceMatchesExpected` | - | 2D nonce on-chain matches ghost state |
| `invariant_C5_createAddressDeterministic` | C5 | Deployed addresses match computation |
| `invariant_K5_keyAuthorizationConsistent` | K5 | Ghost auth matches on-chain state |
| `invariant_K9_spendingLimitEnforced` | K9 | Spent never exceeds limit |
| `invariant_F9_balanceSumConsistent` | F9 | Total supply >= actor balances |
| `invariant_tokenConservation` | - | Tokens are conserved |
| `invariant_createCountConsistent` | - | CREATE count matches ghost |
| `invariant_callsAndCreatesEqualTotal` | - | Calls + Creates = Total |
| `invariant_nonceTypePartition` | - | Protocol + 2D txs = Total |
| `invariant_P256NoncesTracked` | - | P256 addresses tracked correctly |

---

## 12. Test Phases

### Phase 1: Core Nonce Invariants ✅ Complete
- ✅ N1, N2 - Protocol nonce monotonicity and bumping
- ✅ N3-N5 - CREATE with protocol nonce
- ✅ N6-N8 - 2D nonce independence and protocol isolation

### Phase 2: Access Key Invariants ✅ Partial
- ✅ K4-K5, K7-K9 - Authorization, expiry, revocation, spending limits
- ✅ K13-K15 - Signature types (Secp256k1, P256, WebAuthn)
- ⏳ K1-K3, K6, K10-K12, K16 - Remaining key invariants

### Phase 3: CREATE Invariants ✅ Partial
- ✅ C5-C7 - Address derivation and nonce burn on failure
- ⏳ C1-C4, C8-C9 - Structure rules and initcode validation

### Phase 4: Fee Invariants ⏳ In Progress
- ✅ F9 - Balance consistency
- ⏳ F1-F8, F10-F12 - Fee collection mechanics and validation

### Phase 5: Multicall Invariants ⏳ TODO
- ⏳ M1-M9 - Atomicity, gas, state visibility

### Phase 6: Time & Transaction Type Invariants ⏳ Partial
- ✅ TX1-TX3, TX9, TX11, TX13 - Legacy and Tempo basics
- ⏳ T1-T4 - Time windows
- ⏳ TX4-TX8, TX10, TX12 - EIP-1559, EIP-7702, Tempo advanced

### Phase 7: Gas Invariants ⏳ TODO
- ⏳ G1-G10 - Intrinsic, signature, and key auth gas

---

## 13. Dependencies

- ✅ `tempo-std` TempoTransactionLib encoding
- ✅ `vm.executeTransaction` for Legacy transactions
- ✅ `vm.executeTransaction` for Tempo (0x76) transactions
- ✅ AccountKeychain precompile access
- ✅ Nonce precompile access
- ✅ FeeAMM liquidity setup

---

## 13.1 Handler Best Practices

### Ghost State Update Pattern

**IMPORTANT**: Handlers must verify on-chain state before updating ghost state. The pattern below prevents ghost/on-chain mismatches:

```solidity
// ❌ WRONG: Unconditionally update ghost (can cause invariant failures)
try vmExec.executeTransaction(signedTx) {
    ghost_2dNonce[sender][nonceKey]++;  // Bug: assumes nonce always increments
    ghost_totalTxExecuted++;
}

// ✅ CORRECT: Verify on-chain state before updating ghost
try vmExec.executeTransaction(signedTx) {
    uint64 actualNonce = nonce.getNonce(sender, nonceKey);
    if (actualNonce > currentNonce) {
        ghost_2dNonce[sender][nonceKey] = actualNonce;
        ghost_2dNonceUsed[sender][nonceKey] = true;
        ghost_totalTxExecuted++;
    }
}
```

### Why This Matters

1. `vmExec.executeTransaction` may return success even if inner calls revert
2. 2D nonce may not increment if tx validation fails
3. Ghost state must reflect actual on-chain state, not expected state

### Invariant Rules

1. **Invariants must NOT mutate ghost state** - use `public view` and only assert
2. **No "defensive sync"** - if ghost != on-chain, the invariant should FAIL
3. **Cross-check ghost vs on-chain** - iterate actors and verify consistency

---

## 14. Progress Summary

| Category | Implemented | Total | Progress |
|----------|-------------|-------|----------|
| Nonce (N1-N15) | 8 | 15 | 53% |
| Fee (F1-F12) | 1 | 12 | 8% |
| Multicall (M1-M9) | 0 | 9 | 0% |
| CREATE (C1-C9) | 3 | 9 | 33% |
| Access Key (K1-K16) | 8 | 16 | 50% |
| Time (T1-T4) | 0 | 4 | 0% |
| Transaction (TX1-TX13) | 6 | 13 | 46% |
| Gas (G1-G10) | 0 | 10 | 0% |
| **Total** | **26** | **88** | **30%** |

---

## 15. Priority Order

1. **Critical** (Core protocol safety):
   - ✅ N1-N5 - Protocol nonce safety
   - ⏳ F1-F5 - Fee collection mechanics
   - ⏳ M1-M4 - Batch atomicity

2. **High** (CREATE and key authorization):
   - ✅ C5-C7 - CREATE address derivation
   - ⏳ C1-C4 - CREATE structure rules
   - ✅ K4-K5, K7-K9 - Key authorization basics
   - ⏳ K1-K3 - Key auth signing rules

3. **Medium** (2D nonces, fee tokens, spending limits):
   - ✅ N6-N8 - 2D nonce independence
   - ⏳ N9-N15 - 2D nonce gas and replay
   - ⏳ F6-F12 - Fee token validation
   - ⏳ K10-K12 - Spending limit periods

4. **Low** (Gas accounting, tx type coverage):
   - ⏳ G1-G10 - Gas accounting
   - ⏳ TX4-TX13 - Transaction type coverage
