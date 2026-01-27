---
id: TIP-XXXX
title: Account-Level Transfer Policies
description: Extends TIP-403 to allow individual accounts to set their own send and receive policies, enabling regulated entities to enforce account-level compliance controls.
authors: Mallesh Pai @malleshpai
status: Draft
related: TIP-403
---

# TIP-XXXX: Account-Level Transfer Policies

## Abstract

This TIP extends the TIP-403 transfer policy system to support account-level policies. Currently, TIP-403 policies are set at the token level—each TIP-20 token has a single `transferPolicyId` that governs all transfers. This proposal adds the ability for individual accounts to set their own send and receive policies, enabling regulated entities (banks, exchanges, etc.) to enforce compliance controls at the account level, independent of and in addition to token-level policies.

## Motivation

Regulated entities operating on Tempo need the ability to control who they transact with, regardless of what policies the token issuer has set. For example:

- A bank may want to only receive funds from KYC'd addresses (whitelist on receives)
- An exchange may need to block sends to sanctioned addresses (blacklist on sends)
- A custodian may require that all incoming and outgoing transfers pass through approved counterparties

The current TIP-403 system only supports token-level policies: the token issuer sets a policy, and all transfers of that token must satisfy it. This does not allow individual account holders to impose their own restrictions.

### Design Goals

1. **Minimal changes**: Reuse the existing TIP-403 policy infrastructure (policy creation, membership management, isAuthorized logic)
2. **Composable**: Account-level policies AND token-level policies must both pass (neither can override the other)
3. **Opt-in**: Accounts without policies have zero overhead beyond two storage reads
4. **Gas-efficient**: Minimize hot-path gas cost for transfers
5. **State-efficient**: Minimize storage overhead per opted-in account

### Alternatives Considered

1. **Per-token account policies**: Store account policies per token. Rejected due to state bloat (N accounts × M tokens).
2. **Merkle tree policies**: Store policy membership as merkle roots, require proofs at transfer time. Rejected due to UX burden (users must obtain and submit proofs).
3. **Bloom filter policies**: Use probabilistic data structures. Rejected due to false positive risk (security hole for whitelists).

---

# Specification

## Overview

The TIP-403 Registry is extended with a new mapping that allows any account to set its own send and receive policies. These policies are checked on every TIP-20 transfer in addition to the existing token-level policy check.

## Storage Layout

### New Storage

```solidity
mapping(address => uint128) public accountPolicies;
```

The `accountPolicies` mapping packs two `uint64` policy IDs into a single storage slot:

- **Bits 0–63**: `sendPolicyId` — policy checked when this account is the sender
- **Bits 64–127**: `receivePolicyId` — policy checked when this account is the receiver

A policy ID of `0` means "no policy" (always authorized). This matches the existing TIP-403 semantics where policy ID 0 is the "always-reject" policy, but for account-level policies we repurpose ID 0 as "no account policy set" (i.e., defer to token policy only).

### Encoding

```solidity
function encodeAccountPolicies(uint64 sendPolicyId, uint64 receivePolicyId) 
    internal pure returns (uint128) 
{
    return uint128(sendPolicyId) | (uint128(receivePolicyId) << 64);
}

function decodeAccountPolicies(uint128 packed) 
    internal pure returns (uint64 sendPolicyId, uint64 receivePolicyId) 
{
    sendPolicyId = uint64(packed);
    receivePolicyId = uint64(packed >> 64);
}
```

## Interface Extensions

The following functions are added to the TIP-403 Registry:

```solidity
/// @notice Sets the send and receive policies for the caller's account
/// @param sendPolicyId Policy to check when caller sends (0 = no policy)
/// @param receivePolicyId Policy to check when caller receives (0 = no policy)
/// @dev Both policies must exist (or be 0). Caller can only set their own policies.
function setAccountPolicies(uint64 sendPolicyId, uint64 receivePolicyId) external;

/// @notice Returns the send and receive policies for an account
/// @param account The account to query
/// @return sendPolicyId The policy checked when account sends (0 = no policy)
/// @return receivePolicyId The policy checked when account receives (0 = no policy)
function getAccountPolicies(address account) 
    external view returns (uint64 sendPolicyId, uint64 receivePolicyId);

/// @notice Checks if a transfer is authorized under account-level policies
/// @param from The sender address
/// @param to The receiver address
/// @return True if the transfer is authorized under both accounts' policies
/// @dev Returns true if:
///      - from's sendPolicy is 0 OR to is authorized under from's sendPolicy
///      - to's receivePolicy is 0 OR from is authorized under to's receivePolicy
function isTransferAuthorized(address from, address to) external view returns (bool);
```

### Events

```solidity
/// @notice Emitted when an account updates its policies
/// @param account The account that updated its policies
/// @param sendPolicyId The new send policy (0 = no policy)
/// @param receivePolicyId The new receive policy (0 = no policy)
event AccountPoliciesUpdated(
    address indexed account, 
    uint64 sendPolicyId, 
    uint64 receivePolicyId
);
```

### Errors

```solidity
/// @notice Error when setting a policy that does not exist
error PolicyNotFound();
```

## Authorization Logic

### setAccountPolicies

```solidity
function setAccountPolicies(uint64 sendPolicyId, uint64 receivePolicyId) external {
    // Validate policies exist (0 is always valid as "no policy")
    if (sendPolicyId != 0 && !policyExists(sendPolicyId)) {
        revert PolicyNotFound();
    }
    if (receivePolicyId != 0 && !policyExists(receivePolicyId)) {
        revert PolicyNotFound();
    }
    
    accountPolicies[msg.sender] = encodeAccountPolicies(sendPolicyId, receivePolicyId);
    
    emit AccountPoliciesUpdated(msg.sender, sendPolicyId, receivePolicyId);
}
```

### getAccountPolicies

```solidity
function getAccountPolicies(address account) 
    external view returns (uint64 sendPolicyId, uint64 receivePolicyId) 
{
    return decodeAccountPolicies(accountPolicies[account]);
}
```

### isTransferAuthorized

```solidity
function isTransferAuthorized(address from, address to) external view returns (bool) {
    // Decode sender's policies
    (uint64 fromSendPolicy, ) = decodeAccountPolicies(accountPolicies[from]);
    
    // Check sender's send policy: "who can I send to?"
    // If sendPolicy is 0, no restriction. Otherwise, `to` must be authorized.
    if (fromSendPolicy != 0) {
        if (!isAuthorized(fromSendPolicy, to)) {
            return false;
        }
    }
    
    // Decode receiver's policies
    (, uint64 toReceivePolicy) = decodeAccountPolicies(accountPolicies[to]);
    
    // Check receiver's receive policy: "who can send to me?"
    // If receivePolicy is 0, no restriction. Otherwise, `from` must be authorized.
    if (toReceivePolicy != 0) {
        if (!isAuthorized(toReceivePolicy, from)) {
            return false;
        }
    }
    
    return true;
}
```

## Integration with TIP-20

The `transferAuthorized` modifier in TIP-20 is updated to check both token-level and account-level policies:

```solidity
modifier transferAuthorized(address from, address to) {
    // Token-level policy check (existing behavior)
    if (
        !TIP403_REGISTRY.isAuthorized(transferPolicyId, from)
            || !TIP403_REGISTRY.isAuthorized(transferPolicyId, to)
    ) revert PolicyForbids();
    
    // Account-level policy check (new)
    if (!TIP403_REGISTRY.isTransferAuthorized(from, to)) {
        revert PolicyForbids();
    }
    _;
}
```

### Affected Functions

The following TIP-20 functions use the `transferAuthorized` modifier and will now also check account-level policies:

- `transfer(address to, uint256 amount)`
- `transferFrom(address from, address to, uint256 amount)`
- `transferWithMemo(address to, uint256 amount, bytes32 memo)`
- `transferFromWithMemo(address from, address to, uint256 amount, bytes32 memo)`
- `systemTransferFrom(address from, address to, uint256 amount)`

### Mint Behavior

Minting checks the token-level policy on the recipient but does NOT check account-level policies. Rationale: minting is an issuer-controlled operation, and the issuer's token-level policy should be sufficient. The recipient's receive policy is intended to restrict peer-to-peer transfers, not issuer mints.

```solidity
function _mint(address to, uint256 amount) internal {
    // Token-level policy check (existing, unchanged)
    if (!TIP403_REGISTRY.isAuthorized(transferPolicyId, to)) {
        revert PolicyForbids();
    }
    // Account-level policy NOT checked for mints
    // ... rest of mint logic
}
```

### Burn Behavior

Burning does not involve a counterparty, so account-level policies are not applicable. The existing behavior is unchanged.

### Fee Transfers

Fee transfers via `transferFeePreTx` and `transferFeePostTx` do NOT check account-level policies. Rationale: fee transfers are system operations between the user and the FeeManager precompile, and should not be blocked by user-configured policies.

## Gas Cost Analysis

### Per-Transfer Overhead (Incremental)

| Scenario | Additional Gas |
|----------|----------------|
| Neither account has policies set | ~4,200 gas (2 SLOADs for accountPolicies) |
| Sender has send policy | ~6,300 gas (+1 isAuthorized call) |
| Receiver has receive policy | ~6,300 gas (+1 isAuthorized call) |
| Both have policies | ~8,400 gas (+2 isAuthorized calls) |

### Breakdown

1. `accountPolicies[from]` SLOAD: ~2,100 gas
2. `accountPolicies[to]` SLOAD: ~2,100 gas
3. `isAuthorized(sendPolicy, to)` if sendPolicy != 0:
   - `policyData[policyId]` SLOAD: ~2,100 gas (to get policy type)
   - `policySet[policyId][to]` SLOAD: ~2,100 gas
4. `isAuthorized(receivePolicy, from)` if receivePolicy != 0:
   - `policyData[policyId]` SLOAD: ~2,100 gas
   - `policySet[policyId][from]` SLOAD: ~2,100 gas

Note: The baseline ~4,200 gas is incurred on ALL transfers, even when no accounts have policies set. This is the cost of reading the two `accountPolicies` slots to check if policies exist.

### State Creation Costs

| Operation | Gas Cost |
|-----------|----------|
| First call to `setAccountPolicies` (creates slot) | 250,000 gas |
| Subsequent updates to account policies | 5,000 gas |
| Adding address to policy membership | 250,000 gas (first time) |
| Updating address in policy membership | 5,000 gas |

## Example Usage

### Regulated Entity: Whitelist Receives

A bank wants to only receive funds from KYC'd addresses:

```solidity
// 1. Bank creates a whitelist policy (or reuses existing)
uint64 kycWhitelist = TIP403_REGISTRY.createPolicy(bankAdmin, PolicyType.WHITELIST);

// 2. Bank adds KYC'd addresses to the whitelist
TIP403_REGISTRY.modifyPolicyWhitelist(kycWhitelist, kycAddress1, true);
TIP403_REGISTRY.modifyPolicyWhitelist(kycWhitelist, kycAddress2, true);

// 3. Bank sets receive policy (send policy = 0, no restriction on sends)
TIP403_REGISTRY.setAccountPolicies(0, kycWhitelist);

// Result: Bank can only receive from addresses on kycWhitelist
// Bank can send to anyone (subject to token-level policy)
```

### Regulated Entity: Blacklist Sends

An exchange wants to block sends to sanctioned addresses:

```solidity
// 1. Exchange creates a blacklist policy
uint64 sanctionsBlacklist = TIP403_REGISTRY.createPolicy(exchangeAdmin, PolicyType.BLACKLIST);

// 2. Exchange adds sanctioned addresses
TIP403_REGISTRY.modifyPolicyBlacklist(sanctionsBlacklist, sanctionedAddr, true);

// 3. Exchange sets send policy (receive policy = 0, no restriction on receives)
TIP403_REGISTRY.setAccountPolicies(sanctionsBlacklist, 0);

// Result: Exchange cannot send to addresses on sanctionsBlacklist
// Exchange can receive from anyone (subject to token-level policy)
```

### Combined: Whitelist Both Directions

A custodian wants to only transact with approved counterparties:

```solidity
// 1. Custodian creates a whitelist for approved counterparties
uint64 approvedList = TIP403_REGISTRY.createPolicy(custodianAdmin, PolicyType.WHITELIST);

// 2. Add approved addresses
TIP403_REGISTRY.modifyPolicyWhitelist(approvedList, approvedAddr1, true);
TIP403_REGISTRY.modifyPolicyWhitelist(approvedList, approvedAddr2, true);

// 3. Set same policy for both send and receive
TIP403_REGISTRY.setAccountPolicies(approvedList, approvedList);

// Result: Custodian can only send to AND receive from approved addresses
```

---

# Invariants

The following invariants must always hold:

1. **Account Sovereignty**: Only an account itself can set its own account-level policies via `setAccountPolicies`. No other account can modify another account's policies.

2. **Policy Existence**: `setAccountPolicies` MUST revert with `PolicyNotFound()` if either `sendPolicyId` or `receivePolicyId` is non-zero and does not correspond to an existing policy.

3. **Zero Policy Semantics**: A policy ID of `0` in `accountPolicies` MUST be interpreted as "no account-level policy" (always authorized at the account level). This differs from the token-level semantics where policy ID 0 is "always-reject".

4. **Composable Authorization**: A transfer is authorized if and only if ALL of the following are true:
   - Token-level policy authorizes `from`: `isAuthorized(token.transferPolicyId, from)`
   - Token-level policy authorizes `to`: `isAuthorized(token.transferPolicyId, to)`
   - Account-level send policy authorizes `to`: `from.sendPolicyId == 0 OR isAuthorized(from.sendPolicyId, to)`
   - Account-level receive policy authorizes `from`: `to.receivePolicyId == 0 OR isAuthorized(to.receivePolicyId, from)`

5. **Mint Exemption**: Minting operations MUST NOT check account-level policies. Only token-level policy on the recipient is checked.

6. **Burn Exemption**: Burn operations MUST NOT check account-level policies.

7. **Fee Transfer Exemption**: Fee transfers (`transferFeePreTx`, `transferFeePostTx`) MUST NOT check account-level policies.

8. **Storage Efficiency**: Account policies MUST be stored in a single storage slot per account (packed as two uint64 values).

9. **Gas Consistency**: Reading `accountPolicies[address]` for a non-existent entry MUST return 0 (interpreted as no policies set), incurring only the cold SLOAD cost (~2,100 gas).

## Critical Test Cases

1. **Basic send policy**: Account with send policy can only send to addresses authorized under that policy
2. **Basic receive policy**: Account with receive policy can only receive from addresses authorized under that policy
3. **Combined policies**: Account with both policies enforces both on all transfers
4. **No policy (default)**: Account without policies set can send/receive freely (subject to token policy)
5. **Policy ID 0**: Setting either policy to 0 removes that restriction
6. **Token + account policies**: Both token-level AND account-level policies must pass
7. **Self-transfer**: Account with policies can transfer to itself if authorized under its own policies
8. **Mint exemption**: Minting to an account with receive policy does NOT check the receive policy
9. **Burn exemption**: Burning from an account with send policy does NOT check the send policy
10. **Fee transfer exemption**: Fee transfers bypass account-level policies
11. **Policy update**: Account can update its policies; new policies take effect immediately
12. **Invalid policy**: Setting non-existent policy ID reverts with `PolicyNotFound()`
13. **Whitelist send policy**: Account with whitelist send policy can only send to whitelisted addresses
14. **Blacklist send policy**: Account with blacklist send policy cannot send to blacklisted addresses
15. **Whitelist receive policy**: Account with whitelist receive policy can only receive from whitelisted addresses
16. **Blacklist receive policy**: Account with blacklist receive policy cannot receive from blacklisted addresses
17. **Cross-account policies**: A sends to B where A has send policy and B has receive policy; both must authorize
18. **Bidirectional transfer**: A and B both have policies; transfer A→B checks A's send + B's receive; transfer B→A checks B's send + A's receive
19. **Storage packing**: Verify sendPolicyId and receivePolicyId are correctly packed/unpacked
20. **Gas measurement**: Verify gas costs match expected values for each scenario
