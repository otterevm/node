# Tempo Transaction Invariant Test Plan

## Overview

This document outlines all invariants that must be tested for the Tempo transaction handler using `vm.executeTransaction`. The tests verify that the handler correctly enforces protocol rules across random sequences of operations.

---

## 1. Nonce Invariants

### 1.1 Protocol Nonce (nonce_key = 0)

| ID | Invariant | Description |
|----|-----------|-------------|
| N1 | `protocol_nonce_monotonic` | Protocol nonce NEVER decreases for any account |
| N2 | `protocol_nonce_bumps_on_call` | Successful CALL always increments protocol nonce by 1 |
| N3 | `protocol_nonce_bumps_on_create_success` | Successful CREATE increments protocol nonce by 1 |
| N4 | `protocol_nonce_bumps_on_create_failure` | Failed CREATE still increments protocol nonce (when using protocol nonce) |
| N5 | `create_address_uses_protocol_nonce` | CREATE address = keccak256(rlp([caller, protocol_nonce])) regardless of 2D nonce |

### 1.2 2D Nonces (nonce_key > 0)

| ID | Invariant | Description |
|----|-----------|-------------|
| N6 | `2d_nonce_independent` | Nonce keys are fully independent (bumping key=1 doesn't affect key=2) |
| N7 | `2d_nonce_monotonic` | 2D nonces NEVER decrease for any (account, nonce_key) pair |
| N8 | `2d_nonce_no_protocol_effect` | Using 2D nonce doesn't affect protocol nonce for CALL operations |
| N9 | `2d_nonce_create_still_uses_protocol` | CREATE with 2D nonce still uses protocol nonce for address derivation |
| N10 | `2d_nonce_gas_cold` | First use of nonce_key costs 22,100 gas (COLD_SLOAD + SSTORE_SET) |
| N11 | `2d_nonce_gas_warm` | Subsequent uses cost 5,000 gas (COLD_SLOAD + WARM_SSTORE_RESET) |

### 1.3 Replay Protection

| ID | Invariant | Description |
|----|-----------|-------------|
| N12 | `replay_fails_protocol` | Same tx with same protocol nonce fails on replay |
| N13 | `replay_fails_2d` | Same tx with same 2D nonce fails on replay |
| N14 | `wrong_nonce_too_high` | Nonce higher than current is rejected |
| N15 | `wrong_nonce_too_low` | Nonce lower than current is rejected |

---

## 2. Fee Invariants

### 2.1 Fee Collection

| ID | Invariant | Description |
|----|-----------|-------------|
| F1 | `fee_precollected` | Fees are locked BEFORE execution begins |
| F2 | `fee_equals_gas_times_price` | Fee = gas_used * effective_gas_price / SCALING_FACTOR |
| F3 | `fee_refund_on_success` | Unused gas refunded only if ALL calls succeed |
| F4 | `fee_no_refund_on_failure` | No refund if any call in batch fails |
| F5 | `fee_paid_even_on_revert` | User pays for gas even when tx reverts |

### 2.2 Fee Token Validation

| ID | Invariant | Description |
|----|-----------|-------------|
| F6 | `fee_token_must_be_tip20` | Non-zero spending requires TIP20 prefix (0x20C0...) |
| F7 | `fee_token_from_tx` | Explicit tx.fee_token takes priority |
| F8 | `fee_token_fallback` | Falls back to user preference → validator preference → default |
| F9 | `insufficient_balance_rejected` | Tx rejected if fee payer has insufficient balance |
| F10 | `insufficient_liquidity_rejected` | Tx rejected if AMM can't swap fee token |

### 2.3 Subblock Transactions

| ID | Invariant | Description |
|----|-----------|-------------|
| F11 | `subblock_no_fees` | Subblock transactions with non-zero fees are rejected |
| F12 | `subblock_no_keychain` | Keychain operations forbidden in subblock transactions |

---

## 3. Multicall Batch Invariants

### 3.1 Atomicity

| ID | Invariant | Description |
|----|-----------|-------------|
| M1 | `batch_all_or_nothing` | Either ALL calls succeed or ALL state changes revert |
| M2 | `batch_partial_state_reverted` | If call N fails, calls 0..N-1 state changes are reverted |
| M3 | `batch_logs_cleared_on_failure` | All logs from successful calls cleared if batch fails |
| M4 | `batch_logs_preserved_on_success` | All logs preserved if batch succeeds |

### 3.2 Gas Accounting

| ID | Invariant | Description |
|----|-----------|-------------|
| M5 | `batch_gas_accumulated` | Gas used = sum of all individual call gas |
| M6 | `batch_intrinsic_per_call` | Each call adds COLD_ACCOUNT_ACCESS + input data gas |
| M7 | `batch_gas_limit_shared` | All calls share single gas_limit from tx |

### 3.3 State Visibility

| ID | Invariant | Description |
|----|-----------|-------------|
| M8 | `batch_state_visible` | State changes from call N visible to call N+1 |
| M9 | `batch_balance_visible` | Balance changes propagate within batch |

---

## 4. CREATE Invariants

### 4.1 Structure Rules

| ID | Invariant | Description |
|----|-----------|-------------|
| C1 | `create_must_be_first` | CREATE only allowed as first call in batch |
| C2 | `create_max_one` | Maximum one CREATE per transaction |
| C3 | `create_no_auth_list` | CREATE forbidden with EIP-7702 authorization list |
| C4 | `create_no_value` | Value transfers forbidden in AA transactions |

### 4.2 Address Derivation

| ID | Invariant | Description |
|----|-----------|-------------|
| C5 | `create_address_deterministic` | address = keccak256(rlp([caller, protocol_nonce]))[12:] |
| C6 | `create_address_uses_pre_nonce` | Uses nonce value BEFORE increment |
| C7 | `create_nonce_burned_on_failure` | Protocol nonce incremented even on CREATE failure |

### 4.3 Initcode Validation

| ID | Invariant | Description |
|----|-----------|-------------|
| C8 | `create_initcode_size_limit` | Initcode must not exceed max_initcode_size (EIP-3860) |
| C9 | `create_initcode_gas` | Initcode costs 2 gas per 32-byte chunk |

---

## 5. Access Key / Session Key Invariants

### 5.1 KeyAuthorization

| ID | Invariant | Description |
|----|-----------|-------------|
| K1 | `key_auth_signed_by_root` | KeyAuthorization MUST be signed by tx.caller (root account) |
| K2 | `key_auth_self_only` | Access key can only authorize itself, not other keys |
| K3 | `key_auth_chain_id_match` | KeyAuthorization chain_id must be 0 (any) or match current |
| K4 | `key_auth_not_expired` | KeyAuthorization expiry must be > block.timestamp |

### 5.2 Keychain Signature

| ID | Invariant | Description |
|----|-----------|-------------|
| K5 | `keychain_key_must_exist` | Access key must be authorized before use (unless same-tx) |
| K6 | `keychain_same_tx_allowed` | Same-tx authorize + use is permitted |
| K7 | `keychain_expired_rejected` | Expired keys cannot sign transactions |
| K8 | `keychain_revoked_rejected` | Revoked keys cannot sign transactions |

### 5.3 Spending Limits

| ID | Invariant | Description |
|----|-----------|-------------|
| K9 | `spending_limit_enforced` | Access key cannot spend more than authorized per token |
| K10 | `spending_limit_per_period` | Limits reset after spending period expires |
| K11 | `spending_limit_none_unlimited` | None = unlimited spending for that token |
| K12 | `spending_limit_empty_zero` | Empty array = zero spending allowed |

### 5.4 Signature Types

| ID | Invariant | Description |
|----|-----------|-------------|
| K13 | `sig_secp256k1_valid` | 65-byte secp256k1 signatures validate correctly |
| K14 | `sig_p256_valid` | 129-byte P256 signatures validate correctly |
| K15 | `sig_webauthn_valid` | WebAuthn signatures with variable authenticator data validate |
| K16 | `sig_wrong_type_rejected` | Signature type mismatch causes rejection |

---

## 6. Time Window Invariants

| ID | Invariant | Description |
|----|-----------|-------------|
| T1 | `valid_after_enforced` | Tx rejected if block.timestamp < validAfter |
| T2 | `valid_before_enforced` | Tx rejected if block.timestamp >= validBefore |
| T3 | `time_window_both` | Both bounds enforced when set |
| T4 | `time_window_open` | No time bounds = always valid |

---

## 7. Transaction Type Invariants

### 7.1 Legacy Transactions

| ID | Invariant | Description |
|----|-----------|-------------|
| TX1 | `legacy_single_call` | Legacy tx = single CALL or CREATE |
| TX2 | `legacy_protocol_nonce` | Legacy uses protocol nonce only |
| TX3 | `legacy_ecdsa_only` | Legacy uses secp256k1 signature only |

### 7.2 EIP-1559 Transactions

| ID | Invariant | Description |
|----|-----------|-------------|
| TX4 | `eip1559_priority_fee` | maxPriorityFeePerGas enforced |
| TX5 | `eip1559_base_fee` | maxFeePerGas >= baseFee |

### 7.3 EIP-7702 Transactions

| ID | Invariant | Description |
|----|-----------|-------------|
| TX6 | `eip7702_auth_applied` | Authorization list applied before execution |
| TX7 | `eip7702_no_create` | CREATE forbidden with authorization list |

### 7.4 Tempo (0x76) Transactions

| ID | Invariant | Description |
|----|-----------|-------------|
| TX8 | `tempo_multicall` | Supports 1+ calls in single tx |
| TX9 | `tempo_2d_nonce` | Supports 2D nonce system |
| TX10 | `tempo_fee_sponsorship` | Supports fee payer signature |
| TX11 | `tempo_access_keys` | Supports access key signatures |
| TX12 | `tempo_time_windows` | Supports validAfter/validBefore |
| TX13 | `tempo_no_value` | Value field must be 0 in all calls |

---

## 8. Gas Invariants

### 8.1 Intrinsic Gas

| ID | Invariant | Description |
|----|-----------|-------------|
| G1 | `intrinsic_base_21k` | Base transaction cost is 21,000 gas |
| G2 | `intrinsic_per_call` | Each call adds COLD_ACCOUNT_ACCESS (2,600 gas) |
| G3 | `intrinsic_calldata` | 16 gas per non-zero byte, 4 gas per zero byte |
| G4 | `intrinsic_create` | CREATE adds 32,000 gas + initcode cost |
| G5 | `intrinsic_access_list` | Access list adds per-address and per-slot gas |

### 8.2 Signature Gas

| ID | Invariant | Description |
|----|-----------|-------------|
| G6 | `sig_gas_secp256k1` | ECRECOVER cost (3,000 gas) |
| G7 | `sig_gas_p256` | ECRECOVER + 5,000 gas |
| G8 | `sig_gas_webauthn` | ECRECOVER + 5,000 + calldata gas |

### 8.3 KeyAuthorization Gas

| ID | Invariant | Description |
|----|-----------|-------------|
| G9 | `key_auth_base_gas` | 27,000 gas base cost |
| G10 | `key_auth_per_limit` | 22,000 gas per spending limit |

---

## 9. Ghost Variables Required

```solidity
// Per-account tracking
mapping(address => uint256) ghost_protocolNonce;
mapping(address => mapping(uint256 => uint256)) ghost_2dNonce;
mapping(address => uint256) ghost_feeTokenBalance;

// Per-tx tracking  
uint256 ghost_totalTxExecuted;
uint256 ghost_totalTxReverted;
uint256 ghost_totalGasUsed;
uint256 ghost_totalFeesCollected;

// Per-access-key tracking
mapping(address => mapping(address => bool)) ghost_keyAuthorized;
mapping(address => mapping(address => uint256)) ghost_keyExpiry;
mapping(address => mapping(address => mapping(address => uint256))) ghost_keySpendingLimit;
mapping(address => mapping(address => mapping(address => uint256))) ghost_keySpentAmount;

// CREATE tracking
mapping(address => uint256) ghost_createCount;
mapping(bytes32 => address) ghost_createAddresses; // hash(caller, nonce) => deployed address
```

---

## 10. Handler Functions Required

```solidity
// Transaction execution handlers
handler_legacyTransfer(actorSeed, recipientSeed, amount)
handler_legacyCreate(actorSeed, initcode)
handler_eip1559Transfer(actorSeed, recipientSeed, amount, priorityFee)
handler_tempoSingleCall(actorSeed, recipientSeed, amount, nonceKey)
handler_tempoMultiCall(actorSeed, callCount, amounts[])
handler_tempoCreate(actorSeed, initcode, nonceKey)
handler_tempoCreateAndCall(actorSeed, initcode, callData)

// Access key handlers
handler_authorizeKey(ownerSeed, keySeed, expiry, limits[])
handler_useAccessKey(ownerSeed, keySeed, recipientSeed, amount)
handler_revokeKey(ownerSeed, keySeed)

// Time-bound handlers
handler_timeBoundTx(actorSeed, validAfter, validBefore, amount)

// Failure handlers (to test revert behavior)
handler_transferInsufficientBalance(actorSeed)
handler_createWithAuthList(actorSeed)
handler_createNotFirst(actorSeed)
handler_expiredKeyTx(ownerSeed, keySeed)
```

---

## 11. Test Phases

### Phase 1: Core Nonce Invariants (Current)
- ✅ N1, N2 - Protocol nonce monotonicity and bumping
- N3-N5 - CREATE with protocol nonce
- N6-N11 - 2D nonce independence and gas

### Phase 2: Fee Invariants
- F1-F5 - Fee collection mechanics
- F6-F10 - Fee token validation

### Phase 3: Multicall Invariants
- M1-M4 - Atomicity guarantees
- M5-M9 - Gas and state visibility

### Phase 4: CREATE Invariants
- C1-C9 - Structure, address derivation, validation

### Phase 5: Access Key Invariants
- K1-K16 - Authorization, keychain, spending limits

### Phase 6: Cross-Type Invariants
- TX1-TX13 - All transaction types working together

---

## 12. Dependencies

- `tempo-std` TempoTransactionLib encoding fix (blocked for Tempo 0x76 tests)
- `vm.executeTransaction` nonce validation (currently not enforced for Legacy)
- Access to FeeAMM and AccountKeychain precompiles

---

## 13. Priority Order

1. **Critical**: N1-N5, F1-F5, M1-M4 (core protocol safety)
2. **High**: C1-C7, K1-K8 (CREATE and key authorization)
3. **Medium**: N6-N15, F6-F12, K9-K16 (2D nonces, fee tokens, spending limits)
4. **Low**: G1-G10, TX1-TX13 (gas accounting, tx type coverage)
