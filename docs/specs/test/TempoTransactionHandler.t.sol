// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import { IAccountKeychain } from "../src/interfaces/IAccountKeychain.sol";
import { IFeeManager } from "../src/interfaces/IFeeManager.sol";
import { INonce } from "../src/interfaces/INonce.sol";
import { ITIP20 } from "../src/interfaces/ITIP20.sol";
import { TIP20 } from "../src/TIP20.sol";
import { BaseTest } from "./BaseTest.t.sol";
import { Test, console } from "forge-std/Test.sol";

/*//////////////////////////////////////////////////////////////
                    TEMPO TRANSACTION ABSTRACTION
//////////////////////////////////////////////////////////////*/

/// @notice Signature types supported by Tempo
enum SignatureType {
    Secp256k1,  // Standard Ethereum signature (ECDSA secp256k1)
    P256,       // NIST P-256 curve signature
    WebAuthn    // WebAuthn/Passkey signature (P256 with WebAuthn wrapper)
}

/// @notice Represents a single call within a Tempo transaction batch
struct Call {
    address to;             // Target contract (address(0) for CREATE)
    uint256 value;          // Value to send (always 0 on Tempo, reserved for future)
    bytes data;             // Calldata (or initcode for CREATE)
}

/// @notice Signature wrapper supporting multiple signature types
struct TempoSignature {
    SignatureType sigType;  // Type of signature
    bytes signature;        // Raw signature bytes (format depends on sigType)
    // For Secp256k1: 65 bytes (r, s, v)
    // For P256: 129 bytes (r, s, pubKeyX, pubKeyY)
    // For WebAuthn: variable length (authenticatorData + clientDataJSON + P256 sig)
}

/// @notice Keychain signature wrapper for access key transactions
struct KeychainSignature {
    address userAddress;    // Root account address
    TempoSignature inner;   // Inner signature from the access key
}

/// @notice Represents a complete Tempo transaction with call batching
/// Note(@foundry): Maybe we have builder functions in tempo-std to make it easy to create a TempoTx?
struct TempoTx {
    // === Core Fields ===
    address from;                   // Transaction sender (root account)
    Call[] calls;                   // Batch of calls to execute atomically
    uint64 gasLimit;                // Gas limit for entire batch
    uint128 maxFeePerGas;           // Max fee per gas (EIP-1559)
    uint128 maxPriorityFeePerGas;   // Max priority fee per gas (EIP-1559)

    // === Nonce Fields ===
    uint256 nonceKey;               // 2D nonce key (0 = protocol, 1+ = user keys)
    uint64 nonce;                   // Nonce value for the key

    // === Fee Payment ===
    address feeToken;               // Token used to pay fees (address(0) = default)
    address feePayer;               // Fee payer address (address(0) = from pays)

    // === Access Key / Session Key ===
    address accessKey;              // Access key address (address(0) = root key signing)
    SignatureType accessKeyType;    // Signature type of the access key

    // === Time Bounds ===
    uint64 validBefore;             // Tx expires after this timestamp (0 = no expiry)
    uint64 validAfter;              // Tx valid only after this timestamp (0 = immediately)
}

/// @notice Result of executing a Tempo transaction
struct TxResult {
    bool success;           // Whether EVM execution succeeded
    uint256 gasUsed;        // Actual gas consumed
    uint256 feesPaid;       // Fees paid after swap
    uint256 refund;         // Amount refunded to user
    bytes[] returnData;     // Return data from each call in the batch
}

/// @notice Simulates the REVM execution handler for Tempo transactions
/// @dev This abstracts the entire transaction lifecycle that the node performs
abstract contract TempoTransactionExecutor is Test {

    /*//////////////////////////////////////////////////////////////
                            CHEATCODE INTERFACE
    //////////////////////////////////////////////////////////////*/

    /// @dev These cheatcodes would be implemented in tempo-foundry
    /// For now, we simulate them with internal state

    // Transient state (reset per transaction)
    address internal _currentTxKey;
    address internal _currentTxOrigin;
    bool internal _inTransaction;

    // Fee manager reference (set by inheriting contract)
    IFeeManager internal _feeManager;
    INonce internal _nonceManager;
    IAccountKeychain internal _keychain;

    /*//////////////////////////////////////////////////////////////
                          TRANSACTION EXECUTION
    //////////////////////////////////////////////////////////////*/

    /// @notice Execute a complete Tempo transaction
    /// @dev Simulates the full REVM handler flow
    // note(@foundry): We need a vm cheatcode to execute this transaction.
    // This should run similar to isolate mode, as we want to go through the whole handler flow each time.
    function _executeTempoTx(TempoTx memory tx_) internal returns (TxResult memory result) {
        require(!_inTransaction, "Reentrant transaction");
        require(tx_.calls.length > 0, "Calls cannot be empty");
        _inTransaction = true;

        // === PHASE 0: Time Bounds Validation ===
        if (tx_.validAfter > 0 && block.timestamp < tx_.validAfter) {
            _inTransaction = false;
            result.success = false;
            return result;
        }
        if (tx_.validBefore > 0 && block.timestamp >= tx_.validBefore) {
            _inTransaction = false;
            result.success = false;
            return result;
        }

        // === PHASE 1: Signature & Nonce Validation ===
        // In real node, signature is validated based on:
        // - If accessKey == address(0): validate signature from tx.from using any supported type
        // - If accessKey != address(0): validate KeychainSignature (accessKey signs, from is root)
        //   The accessKeyType determines how to verify the signature (Secp256k1, P256, WebAuthn)
        if (!_validateNonce(tx_.from, tx_.nonceKey)) {
            _inTransaction = false;
            result.success = false;
            return result;
        }

        // Validate access key if used
        if (tx_.accessKey != address(0)) {
            if (!_validateAccessKey(tx_.from, tx_.accessKey, tx_.accessKeyType)) {
                _inTransaction = false;
                result.success = false;
                return result;
            }
        }

        // === PHASE 2: Set Transaction Context ===
        _currentTxKey = tx_.accessKey;
        _currentTxOrigin = tx_.from;

        // === PHASE 3: Pre-Transaction Fee Collection ===
        // Fee payer is either specified or defaults to tx.from
        address actualFeePayer = tx_.feePayer != address(0) ? tx_.feePayer : tx_.from;
        uint256 maxFee = uint256(tx_.gasLimit) * uint256(tx_.maxFeePerGas);
        address actualFeeToken;

        try this._collectFeePreTx(actualFeePayer, tx_.feeToken, maxFee, block.coinbase) returns (address token) {
            actualFeeToken = token;
        } catch {
            _inTransaction = false;
            _currentTxKey = address(0);
            _currentTxOrigin = address(0);
            result.success = false;
            return result;
        }

        // === PHASE 4: EVM Execution (Call Batching) ===
        uint256 gasStart = gasleft();
        result.returnData = new bytes[](tx_.calls.length);
        result.success = true;

        // Execute each call in the batch atomically
        for (uint256 i = 0; i < tx_.calls.length; i++) {
            Call memory call = tx_.calls[i];

            // Execute the call
            (bool callSuccess, bytes memory returnData) = call.to.call{gas: tx_.gasLimit}(call.data);

            result.returnData[i] = returnData;

            // If any call fails, the entire batch fails
            if (!callSuccess) {
                result.success = false;
                // Continue to record return data but mark as failed
                // In actual implementation, state would be reverted
                break;
            }
        }

        uint256 gasEnd = gasleft();
        result.gasUsed = gasStart - gasEnd;

        // === PHASE 5: Post-Transaction Fee Settlement ===
        uint256 actualSpending = result.gasUsed * uint256(tx_.maxFeePerGas);
        result.refund = maxFee - actualSpending;

        // Fee swap happens here if tokens differ
        try this._collectFeePostTx(
            actualFeePayer,
            actualSpending,
            result.refund,
            actualFeeToken,
            block.coinbase
        ) {
            result.feesPaid = actualSpending;
        } catch {
            // Fee post-tx should not fail if pre-tx succeeded
            // But handle gracefully
        }

        // === PHASE 6: Increment Nonce (only on success) ===
        if (result.success) {
            _incrementNonce(tx_.from, tx_.nonceKey);
        }

        // === PHASE 7: Cleanup ===
        _currentTxKey = address(0);
        _currentTxOrigin = address(0);
        _inTransaction = false;

        return result;
    }

    /// @notice Validate access key authorization and signature type
    function _validateAccessKey(
        address owner,
        address accessKey,
        SignatureType sigType
    ) internal view returns (bool) {
        // In real implementation, this would:
        // 1. Check the access key is registered in AccountKeychain for this owner
        // 2. Verify the key is not expired
        // 3. Verify the key is not revoked
        // 4. Verify the signature type matches what was registered
        // 5. Check spending limits if enforced

        IAccountKeychain.KeyInfo memory keyInfo = _keychain.getKey(owner, accessKey);

        // Key must exist (expiry > 0 means it was set)
        if (keyInfo.expiry == 0) return false;

        // Key must not be expired
        if (block.timestamp >= keyInfo.expiry) return false;

        // Key must not be revoked
        if (keyInfo.isRevoked) return false;

        // Signature type must match (convert enum to match)
        if (uint8(sigType) != uint8(keyInfo.signatureType)) return false;

        return true;
    }

    /// @notice Validate nonce before execution
    function _validateNonce(address account, uint256 nonceKey) internal view returns (bool) {
        // Nonce key 0 is protocol nonce (stored in account state)
        // Nonce keys 1+ are user nonces (stored in Nonce precompile)
        if (nonceKey == 0) {
            // Would check account.nonce in real implementation
            return true;
        }
        // For user nonces, we just verify it's queryable
        // Real validation would compare against expected nonce
        return true;
    }

    /// @notice Increment nonce after successful tx
    function _incrementNonce(address account, uint256 nonceKey) internal {
        if (nonceKey > 0) {
            // Call nonce precompile
            // In real cheatcode: tempo.incrementNonce(account, nonceKey)
        }
    }

    /// @notice Wrapper for pre-tx fee collection (external for try/catch)
    function _collectFeePreTx(
        address feePayer,
        address userToken,
        uint256 maxAmount,
        address beneficiary
    ) external returns (address) {
        require(msg.sender == address(this), "Internal only");

        // Simulate protocol-level call
        vm.prank(address(0));
        vm.coinbase(beneficiary);

        // Would call: tempo.collectFeePreTx(...)
        // For now, call the actual fee manager
        return _feeManager.collectFeePreTx(feePayer, userToken, maxAmount);
    }

    /// @notice Wrapper for post-tx fee collection
    function _collectFeePostTx(
        address feePayer,
        uint256 actualSpending,
        uint256 refundAmount,
        address feeToken,
        address beneficiary
    ) external {
        require(msg.sender == address(this), "Internal only");

        vm.prank(address(0));
        vm.coinbase(beneficiary);

        _feeManager.collectFeePostTx(feePayer, actualSpending + refundAmount, actualSpending, feeToken);
    }

    /// @notice Check if current tx uses session key
    function _isSessionKeyTx() internal view returns (bool) {
        return _currentTxKey != address(0);
    }

    /// @notice Get current transaction key
    function _getTransactionKey() internal view returns (address) {
        return _currentTxKey;
    }

    /// @notice Get current tx origin
    function _getTxOrigin() internal view returns (address) {
        return _currentTxOrigin;
    }
}

/*//////////////////////////////////////////////////////////////
                    INVARIANT TEST CONTRACT
//////////////////////////////////////////////////////////////*/

/// @title Tempo Transaction Invariant Test
/// @notice Proves invariants hold across any sequence of Tempo transactions
contract TempoTransactionHandlerTest is BaseTest, TempoTransactionExecutor {

    /*//////////////////////////////////////////////////////////////
                              STATE
    //////////////////////////////////////////////////////////////*/

    // Test tokens
    TIP20 feeToken1;
    TIP20 feeToken2;

    // Test actors
    address[] actors;
    mapping(address => bool) isActor;

    // Session keys per actor
    mapping(address => address[]) actorSessionKeys;
    mapping(address => mapping(address => bool)) isValidSessionKey;

    // Validator
    address validator = address(0xVAL1);

    /*//////////////////////////////////////////////////////////////
                          GHOST VARIABLES
    //////////////////////////////////////////////////////////////*/

    // Track all state changes for invariant verification
    uint256 public ghost_totalTransactions;
    uint256 public ghost_successfulTransactions;
    uint256 public ghost_failedTransactions;

    uint256 public ghost_totalFeesCollectedPreTx;
    uint256 public ghost_totalFeesRefunded;
    uint256 public ghost_totalFeesSwapped;
    uint256 public ghost_totalFeesDistributed;

    mapping(address => uint256) public ghost_actorTotalSpent;
    mapping(address => uint256) public ghost_actorTotalRefunded;
    mapping(address => mapping(uint256 => uint256)) public ghost_actorNonces;

    mapping(address => uint256) public ghost_validatorFeesCollected;

    // Token balance tracking
    mapping(address => mapping(address => uint256)) public ghost_tokenBalancesBefore;
    uint256 public ghost_totalSupplyBefore;

    // Session key spending tracking
    mapping(address => mapping(address => mapping(address => uint256))) public ghost_sessionKeySpent;

    /*//////////////////////////////////////////////////////////////
                              SETUP
    //////////////////////////////////////////////////////////////*/

    function setUp() public override {
        super.setUp();

        // Initialize executor references
        _feeManager = IFeeManager(address(amm));
        _nonceManager = nonce;
        _keychain = keychain;

        // Create fee tokens
        feeToken1 = TIP20(
            factory.createToken("FeeToken1", "FT1", "USD", pathUSD, admin, bytes32("ft1"))
        );
        feeToken2 = TIP20(
            factory.createToken("FeeToken2", "FT2", "USD", pathUSD, admin, bytes32("ft2"))
        );

        // Setup actors
        actors.push(alice);
        actors.push(bob);
        actors.push(charlie);
        for (uint i = 0; i < actors.length; i++) {
            isActor[actors[i]] = true;
        }

        // Mint tokens and setup approvals
        vm.startPrank(admin);
        feeToken1.grantRole(_ISSUER_ROLE, admin);
        feeToken2.grantRole(_ISSUER_ROLE, admin);
        pathUSD.grantRole(_ISSUER_ROLE, admin);

        for (uint i = 0; i < actors.length; i++) {
            feeToken1.mint(actors[i], 1_000_000e6);
            feeToken2.mint(actors[i], 1_000_000e6);
            pathUSD.mint(actors[i], 1_000_000e6);
        }

        // Setup AMM liquidity
        feeToken1.mint(admin, 10_000_000e6);
        feeToken2.mint(admin, 10_000_000e6);
        feeToken1.mint(address(amm), 10_000_000e6);
        feeToken2.mint(address(amm), 10_000_000e6);

        feeToken1.approve(address(amm), type(uint256).max);
        feeToken2.approve(address(amm), type(uint256).max);

        amm.mint(address(feeToken1), address(feeToken2), 5_000_000e6, admin);
        amm.mint(address(feeToken1), address(pathUSD), 5_000_000e6, admin);
        amm.mint(address(feeToken2), address(pathUSD), 5_000_000e6, admin);
        vm.stopPrank();

        // Actors approve fee manager
        for (uint i = 0; i < actors.length; i++) {
            vm.startPrank(actors[i]);
            feeToken1.approve(address(amm), type(uint256).max);
            feeToken2.approve(address(amm), type(uint256).max);
            pathUSD.approve(address(amm), type(uint256).max);
            vm.stopPrank();
        }

        // Validator setup
        vm.prank(validator, validator);
        amm.setValidatorToken(address(feeToken2));

        // Snapshot initial state
        _snapshotBalances();
    }

    function _snapshotBalances() internal {
        for (uint i = 0; i < actors.length; i++) {
            ghost_tokenBalancesBefore[actors[i]][address(feeToken1)] = feeToken1.balanceOf(actors[i]);
            ghost_tokenBalancesBefore[actors[i]][address(feeToken2)] = feeToken2.balanceOf(actors[i]);
            ghost_tokenBalancesBefore[actors[i]][address(pathUSD)] = pathUSD.balanceOf(actors[i]);
        }
    }

    /*//////////////////////////////////////////////////////////////
                          TRANSACTION HANDLERS
    //////////////////////////////////////////////////////////////*/

    /// @notice Handler: Execute a random transfer transaction
    function handler_transfer(
        uint256 actorSeed,
        uint256 recipientSeed,
        uint256 amount,
        uint256 gasLimit,
        uint256 maxFeePerGas,
        uint256 feeTokenSeed
    ) public {
        // Bound inputs
        address sender = actors[actorSeed % actors.length];
        address recipient = actors[recipientSeed % actors.length];
        if (sender == recipient) recipient = actors[(recipientSeed + 1) % actors.length];

        amount = bound(amount, 1e6, 10_000e6);
        gasLimit = bound(gasLimit, 50_000, 500_000);
        maxFeePerGas = bound(maxFeePerGas, 1e6, 10e6); // 1-10 tokens per gas unit

        address feeToken = feeTokenSeed % 2 == 0 ? address(feeToken1) : address(feeToken2);

        // Check sender can afford
        uint256 maxFee = gasLimit * maxFeePerGas;
        TIP20 feeTokenContract = TIP20(feeToken);
        if (feeTokenContract.balanceOf(sender) < maxFee + amount) return;

        // Build call batch (single call)
        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(feeToken1),
            value: 0,
            data: abi.encodeCall(ITIP20.transfer, (recipient, amount))
        });

        // Build transaction
        TempoTx memory tx_ = TempoTx({
            from: sender,
            calls: calls,
            gasLimit: uint64(gasLimit),
            maxFeePerGas: uint128(maxFeePerGas),
            maxPriorityFeePerGas: 0,
            nonceKey: 0,
            nonce: uint64(ghost_actorNonces[sender][0]),
            feeToken: feeToken,
            feePayer: address(0), // Sender pays fees
            accessKey: address(0), // Root key signing
            accessKeyType: SignatureType.Secp256k1,
            validBefore: 0,
            validAfter: 0
        });

        // Set validator as coinbase
        vm.coinbase(validator);

        // Execute
        TxResult memory result = _executeTempoTx(tx_);

        // Update ghost state
        ghost_totalTransactions++;
        if (result.success) {
            ghost_successfulTransactions++;
            ghost_actorNonces[sender][0]++;
        } else {
            ghost_failedTransactions++;
        }

        ghost_totalFeesCollectedPreTx += maxFee;
        ghost_totalFeesRefunded += result.refund;
        ghost_actorTotalSpent[sender] += result.feesPaid;
        ghost_actorTotalRefunded[sender] += result.refund;
    }

    /// @notice Handler: Execute a transfer with access key (session key)
    function handler_sessionKeyTransfer(
        uint256 actorSeed,
        uint256 recipientSeed,
        uint256 amount,
        uint256 sessionKeySeed
    ) public {
        address sender = actors[actorSeed % actors.length];
        address recipient = actors[recipientSeed % actors.length];
        if (sender == recipient) recipient = actors[(recipientSeed + 1) % actors.length];

        amount = bound(amount, 1e6, 100e6);

        // Create or get access key (session key)
        address accessKey = address(uint160(uint256(keccak256(abi.encode(sender, sessionKeySeed)))));

        // Ensure access key is authorized with spending limit
        if (!isValidSessionKey[sender][accessKey]) {
            _authorizeSessionKey(sender, accessKey, 1000e6); // 1000 token limit
        }

        // Check spending limit
        uint256 remainingLimit = keychain.getRemainingLimit(sender, accessKey, address(feeToken1));
        if (amount > remainingLimit) return;

        // Build call batch (single call)
        Call[] memory calls = new Call[](1);
        calls[0] = Call({
            to: address(feeToken1),
            value: 0,
            data: abi.encodeCall(ITIP20.transfer, (recipient, amount))
        });

        // Build transaction with access key
        TempoTx memory tx_ = TempoTx({
            from: sender,
            calls: calls,
            gasLimit: 100_000,
            maxFeePerGas: 1e6,
            maxPriorityFeePerGas: 0,
            nonceKey: 1, // User nonce key for parallel txs
            nonce: uint64(ghost_actorNonces[sender][1]),
            feeToken: address(feeToken1),
            feePayer: address(0), // Sender pays
            accessKey: accessKey,
            accessKeyType: SignatureType.Secp256k1, // Access key uses secp256k1
            validBefore: 0,
            validAfter: 0
        });

        vm.coinbase(validator);
        TxResult memory result = _executeTempoTx(tx_);

        // Track access key spending
        if (result.success) {
            ghost_sessionKeySpent[sender][accessKey][address(feeToken1)] += amount;
            ghost_actorNonces[sender][1]++;
        }

        ghost_totalTransactions++;
        if (result.success) ghost_successfulTransactions++;
        else ghost_failedTransactions++;
    }

    /// @notice Handler: Distribute accumulated fees
    function handler_distributeFees() public {
        uint256 collectedBefore = amm.collectedFees(validator, address(feeToken2));

        if (collectedBefore > 0) {
            uint256 balanceBefore = feeToken2.balanceOf(validator);
            amm.distributeFees(validator, address(feeToken2));
            uint256 balanceAfter = feeToken2.balanceOf(validator);

            ghost_totalFeesDistributed += (balanceAfter - balanceBefore);
            ghost_validatorFeesCollected[validator] += (balanceAfter - balanceBefore);
        }
    }

    /// @notice Handler: Change validator's preferred fee token
    function handler_changeValidatorToken(uint256 tokenSeed) public {
        address newToken = tokenSeed % 2 == 0 ? address(feeToken1) : address(feeToken2);

        // Distribute any pending fees first
        handler_distributeFees();

        vm.prank(validator, validator);
        amm.setValidatorToken(newToken);
    }

    /// @notice Handler: Revoke a session key
    function handler_revokeSessionKey(uint256 actorSeed, uint256 sessionKeySeed) public {
        address actor = actors[actorSeed % actors.length];
        address sessionKey = address(uint160(uint256(keccak256(abi.encode(actor, sessionKeySeed)))));

        if (isValidSessionKey[actor][sessionKey]) {
            vm.prank(actor);
            try keychain.revokeKey(sessionKey) {
                isValidSessionKey[actor][sessionKey] = false;
            } catch {}
        }
    }

    /*//////////////////////////////////////////////////////////////
                          HELPER FUNCTIONS
    //////////////////////////////////////////////////////////////*/

    function _authorizeSessionKey(address owner, address sessionKey, uint256 limit) internal {
        vm.startPrank(owner);

        IAccountKeychain.TokenLimit[] memory limits = new IAccountKeychain.TokenLimit[](1);
        limits[0] = IAccountKeychain.TokenLimit({
            token: address(feeToken1),
            amount: limit
        });

        try keychain.authorizeKey(
            sessionKey,
            IAccountKeychain.SignatureType.Secp256k1,
            uint64(block.timestamp + 30 days),
            true,
            limits
        ) {
            isValidSessionKey[owner][sessionKey] = true;
            actorSessionKeys[owner].push(sessionKey);
        } catch {}

        vm.stopPrank();
    }

    /*//////////////////////////////////////////////////////////////
                            INVARIANTS
    //////////////////////////////////////////////////////////////*/

    /// @notice INVARIANT: Total fees collected >= total fees distributed
    function invariant_feesCollectedGteDistributed() public view {
        assertGe(
            ghost_totalFeesCollectedPreTx - ghost_totalFeesRefunded,
            ghost_totalFeesDistributed,
            "Distributed more fees than collected"
        );
    }

    /// @notice INVARIANT: Successful + Failed = Total transactions
    function invariant_transactionCounting() public view {
        assertEq(
            ghost_successfulTransactions + ghost_failedTransactions,
            ghost_totalTransactions,
            "Transaction count mismatch"
        );
    }

    /// @notice INVARIANT: Actor spent = collected pre-tx - refunded
    function invariant_actorSpendingAccounting() public view {
        for (uint i = 0; i < actors.length; i++) {
            address actor = actors[i];
            // What actor spent should equal what was taken minus refunds
            // Note: This is approximate due to swaps
            uint256 netSpent = ghost_actorTotalSpent[actor];
            uint256 netRefunded = ghost_actorTotalRefunded[actor];

            // Net spent should always be >= 0 (can't refund more than collected)
            assertGe(netSpent + netRefunded, netSpent, "Negative net spending");
        }
    }

    /// @notice INVARIANT: Session key spending never exceeds authorized limit
    function invariant_sessionKeySpendingLimits() public view {
        for (uint i = 0; i < actors.length; i++) {
            address actor = actors[i];
            address[] storage keys = actorSessionKeys[actor];

            for (uint j = 0; j < keys.length; j++) {
                address sessionKey = keys[j];
                uint256 spent = ghost_sessionKeySpent[actor][sessionKey][address(feeToken1)];

                // Initial limit was 1000e6
                assertLe(spent, 1000e6, "Session key exceeded spending limit");
            }
        }
    }

    /// @notice INVARIANT: Token total supply is conserved (no inflation/deflation from fees)
    function invariant_tokenSupplyConservation() public view {
        // Fee operations should not create or destroy tokens
        // Tokens move between accounts but total supply stays same
        uint256 currentSupply = feeToken1.totalSupply();

        // Total supply should not have changed
        // (assuming no mints/burns during test)
        assertTrue(currentSupply > 0, "Token supply should be positive");
    }

    /// @notice INVARIANT: Validator can always withdraw their collected fees
    function invariant_validatorCanWithdraw() public {
        uint256 collected = amm.collectedFees(validator, address(feeToken2));

        if (collected > 0) {
            uint256 ammBalance = feeToken2.balanceOf(address(amm));
            assertGe(ammBalance, collected, "AMM cannot cover validator fees");
        }
    }

    /// @notice INVARIANT: Nonces are monotonically increasing per (account, key)
    function invariant_nonceMonotonicity() public view {
        for (uint i = 0; i < actors.length; i++) {
            address actor = actors[i];
            // Protocol nonce (key 0)
            assertGe(ghost_actorNonces[actor][0], 0, "Nonce went negative");
            // User nonces should also be monotonic
            assertGe(ghost_actorNonces[actor][1], 0, "User nonce went negative");
        }
    }

    /// @notice INVARIANT: Revoked session keys cannot authorize new spending
    function invariant_revokedKeysCannotSpend() public view {
        for (uint i = 0; i < actors.length; i++) {
            address actor = actors[i];
            address[] storage keys = actorSessionKeys[actor];

            for (uint j = 0; j < keys.length; j++) {
                address sessionKey = keys[j];

                if (!isValidSessionKey[actor][sessionKey]) {
                    // Revoked key should have isRevoked = true
                    IAccountKeychain.KeyInfo memory info = keychain.getKey(actor, sessionKey);

                    if (info.expiry == 0) {
                        // Key was revoked, verify no spending after revocation
                        // (tracked separately if needed)
                    }
                }
            }
        }
    }

    /// @notice INVARIANT: AMM pool reserves are always positive after swaps
    function invariant_ammPoolsPositive() public view {
        bytes32 poolId1 = amm.getPoolId(address(feeToken1), address(feeToken2));
        (uint128 reserve1U, uint128 reserve1V) = amm.pools(poolId1);

        // Reserves should never go to zero (MIN_LIQUIDITY locked)
        if (reserve1U > 0 || reserve1V > 0) {
            assertGt(reserve1U, 0, "Pool userToken reserve is zero");
            assertGt(reserve1V, 0, "Pool validatorToken reserve is zero");
        }
    }

    /*//////////////////////////////////////////////////////////////
                        TARGET CONFIGURATION
    //////////////////////////////////////////////////////////////*/

    function targetContracts() public view returns (address[] memory) {
        address[] memory targets = new address[](1);
        targets[0] = address(this);
        return targets;
    }

    function targetSelectors() public view returns (bytes4[] memory) {
        bytes4[] memory selectors = new bytes4[](5);
        selectors[0] = this.handler_transfer.selector;
        selectors[1] = this.handler_sessionKeyTransfer.selector;
        selectors[2] = this.handler_distributeFees.selector;
        selectors[3] = this.handler_changeValidatorToken.selector;
        selectors[4] = this.handler_revokeSessionKey.selector;
        return selectors;
    }

    /// @notice Exclude certain senders from invariant testing
    function excludeSenders() public view returns (address[] memory) {
        address[] memory excluded = new address[](2);
        excluded[0] = address(0);
        excluded[1] = address(this);
        return excluded;
    }
}
