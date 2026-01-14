// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import { TIP20 } from "../../src/TIP20.sol";
import { ITIP20 } from "../../src/interfaces/ITIP20.sol";
import { InvariantBaseTest } from "./InvariantBaseTest.t.sol";

/// @title TIP20 Invariant Tests
/// @notice Fuzz-based invariant tests for the TIP20 token implementation
/// @dev Tests invariants TEMPO-TIP1 through TEMPO-TIP22 as documented in README.md
contract TIP20InvariantTest is InvariantBaseTest {

    /// @dev Log file path for recording actions
    string private constant LOG_FILE = "tip20.log";

    /// @dev Ghost variables for tracking operations
    uint256 private _totalTransfers;
    uint256 private _totalMints;
    uint256 private _totalBurns;
    uint256 private _totalApprovals;

    /// @dev Ghost variables for reward distribution tracking
    uint256 private _totalRewardsDistributed;
    uint256 private _totalRewardsClaimed;
    uint256 private _ghostRewardInputSum;
    uint256 private _ghostRewardClaimSum;

    /// @dev Track total supply changes for conservation check
    mapping(address => uint256) private _tokenMintSum;
    mapping(address => uint256) private _tokenBurnSum;

    /// @dev Constants
    uint256 internal constant ACC_PRECISION = 1e18;

    /// @notice Sets up the test environment
    function setUp() public override {
        super.setUp();

        targetContract(address(this));

        _setupInvariantBase();
        _actors = _buildActors(20);

        // Track initial mints from _buildActors
        for (uint256 i = 0; i < _tokens.length; i++) {
            _tokenMintSum[address(_tokens[i])] = 20 * 1_000_000_000_000;
        }

        _initLogFile(LOG_FILE, "TIP20 Invariant Test Log");
    }

    /*//////////////////////////////////////////////////////////////
                            FUZZ HANDLERS
    //////////////////////////////////////////////////////////////*/

    /// @notice Handler for token transfers
    /// @dev Tests TEMPO-TIP1 (balance conservation), TEMPO-TIP2 (transfer events)
    function transfer(uint256 actorSeed, uint256 tokenSeed, uint256 recipientSeed, uint256 amount)
        external
    {
        address actor = _selectActor(actorSeed);
        address recipient = _selectActor(recipientSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        vm.assume(actor != recipient);

        uint256 actorBalance = token.balanceOf(actor);
        vm.assume(actorBalance > 0);

        amount = bound(amount, 1, actorBalance);

        vm.assume(_isAuthorized(address(token), actor));
        vm.assume(_isAuthorized(address(token), recipient));
        vm.assume(!token.paused());

        uint256 recipientBalanceBefore = token.balanceOf(recipient);
        uint256 totalSupplyBefore = token.totalSupply();

        vm.startPrank(actor);
        try token.transfer(recipient, amount) returns (bool success) {
            vm.stopPrank();
            assertTrue(success, "TEMPO-TIP1: Transfer should return true");

            _totalTransfers++;

            // TEMPO-TIP1: Balance conservation
            assertEq(
                token.balanceOf(actor),
                actorBalance - amount,
                "TEMPO-TIP1: Sender balance not decreased correctly"
            );
            assertEq(
                token.balanceOf(recipient),
                recipientBalanceBefore + amount,
                "TEMPO-TIP1: Recipient balance not increased correctly"
            );

            // TEMPO-TIP2: Total supply unchanged
            assertEq(
                token.totalSupply(),
                totalSupplyBefore,
                "TEMPO-TIP2: Total supply changed during transfer"
            );

            _log(
                string.concat(
                    "TRANSFER: ",
                    _getActorIndex(actor),
                    " -> ",
                    _getActorIndex(recipient),
                    " ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for transferFrom with allowance
    /// @dev Tests TEMPO-TIP3 (allowance consumption), TEMPO-TIP4 (infinite allowance)
    function transferFrom(
        uint256 actorSeed,
        uint256 tokenSeed,
        uint256 ownerSeed,
        uint256 recipientSeed,
        uint256 amount
    ) external {
        address spender = _selectActor(actorSeed);
        address owner = _selectActor(ownerSeed);
        address recipient = _selectActor(recipientSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        vm.assume(owner != spender);
        vm.assume(owner != recipient);

        uint256 ownerBalance = token.balanceOf(owner);
        vm.assume(ownerBalance > 0);

        uint256 allowance = token.allowance(owner, spender);
        vm.assume(allowance > 0);

        amount = bound(amount, 1, ownerBalance < allowance ? ownerBalance : allowance);

        vm.assume(_isAuthorized(address(token), owner));
        vm.assume(_isAuthorized(address(token), recipient));
        vm.assume(!token.paused());

        uint256 recipientBalanceBefore = token.balanceOf(recipient);
        bool isInfiniteAllowance = allowance == type(uint256).max;

        vm.startPrank(spender);
        try token.transferFrom(owner, recipient, amount) returns (bool success) {
            vm.stopPrank();
            assertTrue(success, "TEMPO-TIP3: TransferFrom should return true");

            _totalTransfers++;

            // TEMPO-TIP3/TIP4: Allowance handling
            if (isInfiniteAllowance) {
                assertEq(
                    token.allowance(owner, spender),
                    type(uint256).max,
                    "TEMPO-TIP4: Infinite allowance should remain infinite"
                );
            } else {
                assertEq(
                    token.allowance(owner, spender),
                    allowance - amount,
                    "TEMPO-TIP3: Allowance not decreased correctly"
                );
            }

            assertEq(
                token.balanceOf(owner),
                ownerBalance - amount,
                "TEMPO-TIP3: Owner balance not decreased"
            );
            assertEq(
                token.balanceOf(recipient),
                recipientBalanceBefore + amount,
                "TEMPO-TIP3: Recipient balance not increased"
            );

            _log(
                string.concat(
                    "TRANSFER_FROM: ",
                    _getActorIndex(owner),
                    " -> ",
                    _getActorIndex(recipient),
                    " via ",
                    _getActorIndex(spender),
                    " ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for approvals
    /// @dev Tests TEMPO-TIP5 (allowance setting)
    function approve(uint256 actorSeed, uint256 tokenSeed, uint256 spenderSeed, uint256 amount)
        external
    {
        address actor = _selectActor(actorSeed);
        address spender = _selectActor(spenderSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        amount = bound(amount, 0, type(uint128).max);

        vm.startPrank(actor);
        try token.approve(spender, amount) returns (bool success) {
            vm.stopPrank();
            assertTrue(success, "TEMPO-TIP5: Approve should return true");

            _totalApprovals++;

            assertEq(
                token.allowance(actor, spender),
                amount,
                "TEMPO-TIP5: Allowance not set correctly"
            );

            _log(
                string.concat(
                    "APPROVE: ",
                    _getActorIndex(actor),
                    " approved ",
                    _getActorIndex(spender),
                    " for ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for minting tokens
    /// @dev Tests TEMPO-TIP6 (supply increase), TEMPO-TIP7 (supply cap)
    function mint(uint256 tokenSeed, uint256 recipientSeed, uint256 amount) external {
        TIP20 token = _selectBaseToken(tokenSeed);
        address recipient = _selectActor(recipientSeed);

        uint256 currentSupply = token.totalSupply();
        uint256 supplyCap = token.supplyCap();
        uint256 remaining = supplyCap > currentSupply ? supplyCap - currentSupply : 0;

        vm.assume(remaining > 0);
        amount = bound(amount, 1, remaining);

        vm.assume(_isAuthorized(address(token), recipient));

        uint256 recipientBalanceBefore = token.balanceOf(recipient);

        vm.startPrank(admin);
        try token.mint(recipient, amount) {
            vm.stopPrank();

            _totalMints++;
            _tokenMintSum[address(token)] += amount;

            // TEMPO-TIP6: Total supply should increase
            assertEq(
                token.totalSupply(),
                currentSupply + amount,
                "TEMPO-TIP6: Total supply not increased correctly"
            );

            // TEMPO-TIP7: Total supply should not exceed cap
            assertLe(
                token.totalSupply(),
                supplyCap,
                "TEMPO-TIP7: Total supply exceeds supply cap"
            );

            assertEq(
                token.balanceOf(recipient),
                recipientBalanceBefore + amount,
                "TEMPO-TIP6: Recipient balance not increased"
            );

            _log(
                string.concat(
                    "MINT: ",
                    vm.toString(amount),
                    " ",
                    token.symbol(),
                    " to ",
                    _getActorIndex(recipient)
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for burning tokens
    /// @dev Tests TEMPO-TIP8 (supply decrease)
    function burn(uint256 tokenSeed, uint256 amount) external {
        TIP20 token = _selectBaseToken(tokenSeed);

        uint256 adminBalance = token.balanceOf(admin);
        vm.assume(adminBalance > 0);

        amount = bound(amount, 1, adminBalance);

        uint256 totalSupplyBefore = token.totalSupply();

        vm.startPrank(admin);
        try token.burn(amount) {
            vm.stopPrank();

            _totalBurns++;
            _tokenBurnSum[address(token)] += amount;

            // TEMPO-TIP8: Total supply should decrease
            assertEq(
                token.totalSupply(),
                totalSupplyBefore - amount,
                "TEMPO-TIP8: Total supply not decreased correctly"
            );

            assertEq(
                token.balanceOf(admin),
                adminBalance - amount,
                "TEMPO-TIP8: Admin balance not decreased"
            );

            _log(
                string.concat(
                    "BURN: ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for transfer with memo
    /// @dev Tests TEMPO-TIP9 (memo transfers work like regular transfers)
    function transferWithMemo(
        uint256 actorSeed,
        uint256 tokenSeed,
        uint256 recipientSeed,
        uint256 amount,
        bytes32 memo
    ) external {
        address actor = _selectActor(actorSeed);
        address recipient = _selectActor(recipientSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        vm.assume(actor != recipient);

        uint256 actorBalance = token.balanceOf(actor);
        vm.assume(actorBalance > 0);

        amount = bound(amount, 1, actorBalance);

        vm.assume(_isAuthorized(address(token), actor));
        vm.assume(_isAuthorized(address(token), recipient));
        vm.assume(!token.paused());

        uint256 recipientBalanceBefore = token.balanceOf(recipient);
        uint256 totalSupplyBefore = token.totalSupply();

        vm.startPrank(actor);
        try token.transferWithMemo(recipient, amount, memo) {
            vm.stopPrank();

            _totalTransfers++;

            // TEMPO-TIP9: Balance changes same as regular transfer
            assertEq(
                token.balanceOf(actor),
                actorBalance - amount,
                "TEMPO-TIP9: Sender balance not decreased"
            );
            assertEq(
                token.balanceOf(recipient),
                recipientBalanceBefore + amount,
                "TEMPO-TIP9: Recipient balance not increased"
            );
            assertEq(
                token.totalSupply(),
                totalSupplyBefore,
                "TEMPO-TIP9: Total supply changed"
            );

            _log(
                string.concat(
                    "TRANSFER_WITH_MEMO: ",
                    _getActorIndex(actor),
                    " -> ",
                    _getActorIndex(recipient),
                    " ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for setting reward recipient (opt-in)
    /// @dev Tests TEMPO-TIP10 (opted-in supply tracking)
    function setRewardRecipient(uint256 actorSeed, uint256 tokenSeed, uint256 recipientSeed)
        external
    {
        address actor = _selectActor(actorSeed);
        TIP20 token = _selectBaseToken(tokenSeed);
        
        bool optIn = recipientSeed % 2 == 0;
        address newRecipient = optIn ? actor : address(0);

        vm.assume(_isAuthorized(address(token), actor));
        if (optIn) {
            vm.assume(_isAuthorized(address(token), newRecipient));
        }
        vm.assume(!token.paused());

        (address currentRecipient,,) = token.userRewardInfo(actor);
        uint256 actorBalance = token.balanceOf(actor);
        uint128 optedInSupplyBefore = token.optedInSupply();

        vm.startPrank(actor);
        try token.setRewardRecipient(newRecipient) {
            vm.stopPrank();

            (address storedRecipient,,) = token.userRewardInfo(actor);

            // TEMPO-TIP10: Reward recipient should be updated
            assertEq(
                storedRecipient,
                newRecipient,
                "TEMPO-TIP10: Reward recipient not set correctly"
            );

            // TEMPO-TIP11: Opted-in supply should update correctly
            uint128 optedInSupplyAfter = token.optedInSupply();
            if (currentRecipient == address(0) && newRecipient != address(0)) {
                assertEq(
                    optedInSupplyAfter,
                    optedInSupplyBefore + uint128(actorBalance),
                    "TEMPO-TIP11: Opted-in supply not increased"
                );
            } else if (currentRecipient != address(0) && newRecipient == address(0)) {
                assertEq(
                    optedInSupplyAfter,
                    optedInSupplyBefore - uint128(actorBalance),
                    "TEMPO-TIP11: Opted-in supply not decreased"
                );
            } else {
                assertEq(
                    optedInSupplyAfter,
                    optedInSupplyBefore,
                    "TEMPO-TIP11: Opted-in supply changed unexpectedly"
                );
            }

            _log(
                string.concat(
                    "SET_REWARD_RECIPIENT: ",
                    _getActorIndex(actor),
                    " -> ",
                    optIn ? _getActorIndex(newRecipient) : "NONE",
                    " on ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for distributing rewards
    /// @dev Tests TEMPO-TIP12, TEMPO-TIP13
    function distributeReward(uint256 actorSeed, uint256 tokenSeed, uint256 amount) external {
        address actor = _selectActor(actorSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        uint256 actorBalance = token.balanceOf(actor);
        vm.assume(actorBalance > 0);

        amount = bound(amount, 1, actorBalance);

        vm.assume(_isAuthorized(address(token), actor));
        vm.assume(!token.paused());

        uint128 optedInSupply = token.optedInSupply();
        vm.assume(optedInSupply > 0);

        uint256 globalRPTBefore = token.globalRewardPerToken();
        uint256 tokenBalanceBefore = token.balanceOf(address(token));

        vm.startPrank(actor);
        try token.distributeReward(amount) {
            vm.stopPrank();

            _totalRewardsDistributed++;
            _ghostRewardInputSum += amount;

            // TEMPO-TIP12: Global reward per token should increase (or stay same for very small amounts)
            uint256 globalRPTAfter = token.globalRewardPerToken();
            assertGe(
                globalRPTAfter,
                globalRPTBefore,
                "TEMPO-TIP12: Global reward per token should not decrease"
            );

            // TEMPO-TIP13: Tokens should be transferred to the token contract
            assertEq(
                token.balanceOf(address(token)),
                tokenBalanceBefore + amount,
                "TEMPO-TIP13: Tokens not transferred to contract"
            );

            _log(
                string.concat(
                    "DISTRIBUTE_REWARD: ",
                    _getActorIndex(actor),
                    " distributed ",
                    vm.toString(amount),
                    " ",
                    token.symbol()
                )
            );
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for claiming rewards
    /// @dev Tests TEMPO-TIP14, TEMPO-TIP15
    function claimRewards(uint256 actorSeed, uint256 tokenSeed) external {
        address actor = _selectActor(actorSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        vm.assume(_isAuthorized(address(token), actor));
        vm.assume(_isAuthorized(address(token), address(token)));
        vm.assume(!token.paused());

        (,, uint256 rewardBalance) = token.userRewardInfo(actor);
        uint256 actorBalanceBefore = token.balanceOf(actor);
        uint256 contractBalanceBefore = token.balanceOf(address(token));

        vm.startPrank(actor);
        try token.claimRewards() returns (uint256 claimed) {
            vm.stopPrank();

            if (rewardBalance > 0 || claimed > 0) {
                _totalRewardsClaimed++;
                _ghostRewardClaimSum += claimed;
            }

            // TEMPO-TIP14: Actor should receive claimed amount
            assertEq(
                token.balanceOf(actor),
                actorBalanceBefore + claimed,
                "TEMPO-TIP14: Actor balance not increased by claimed amount"
            );

            assertEq(
                token.balanceOf(address(token)),
                contractBalanceBefore - claimed,
                "TEMPO-TIP14: Contract balance not decreased"
            );

            // TEMPO-TIP15: Claimed amount should not exceed available
            assertLe(
                claimed,
                contractBalanceBefore,
                "TEMPO-TIP15: Claimed more than contract balance"
            );

            if (claimed > 0) {
                _log(
                    string.concat(
                        "CLAIM_REWARDS: ",
                        _getActorIndex(actor),
                        " claimed ",
                        vm.toString(claimed),
                        " ",
                        token.symbol()
                    )
                );
            }
        } catch (bytes memory reason) {
            vm.stopPrank();
            _assertKnownError(reason);
        }
    }

    /// @notice Handler for toggling blacklist
    /// @dev Tests TEMPO-TIP16 (blacklist enforcement)
    function toggleBlacklist(uint256 actorSeed, uint256 tokenSeed, bool blacklist) external {
        address actor = _selectActor(actorSeed);
        TIP20 token = _selectBaseToken(tokenSeed);

        // Only toggle for actors 0-4
        vm.assume(actorSeed % _actors.length < 5);

        bool currentlyAuthorized = _isAuthorized(address(token), actor);

        if (blacklist && !currentlyAuthorized) return;
        if (!blacklist && currentlyAuthorized) return;

        _setBlacklist(address(token), actor, blacklist);

        // TEMPO-TIP16: Authorization status should be updated
        bool afterAuthorized = _isAuthorized(address(token), actor);
        assertEq(
            afterAuthorized,
            !blacklist,
            "TEMPO-TIP16: Blacklist status not updated correctly"
        );

        _log(
            string.concat(
                "TOGGLE_BLACKLIST: ",
                _getActorIndex(actor),
                " ",
                blacklist ? "BLACKLISTED" : "UNBLACKLISTED",
                " on ",
                token.symbol()
            )
        );
    }

    /// @notice Handler for pause/unpause
    /// @dev Tests TEMPO-TIP17 (pause enforcement)
    function togglePause(uint256 tokenSeed, bool pause) external {
        TIP20 token = _selectBaseToken(tokenSeed);

        vm.startPrank(admin);
        token.grantRole(_PAUSE_ROLE, admin);
        token.grantRole(_UNPAUSE_ROLE, admin);

        if (pause && !token.paused()) {
            token.pause();
            assertTrue(token.paused(), "TEMPO-TIP17: Token should be paused");
        } else if (!pause && token.paused()) {
            token.unpause();
            assertFalse(token.paused(), "TEMPO-TIP17: Token should be unpaused");
        }
        vm.stopPrank();

        _log(
            string.concat(
                "TOGGLE_PAUSE: ",
                token.symbol(),
                " ",
                pause ? "PAUSED" : "UNPAUSED"
            )
        );
    }

    /*//////////////////////////////////////////////////////////////
                         GLOBAL INVARIANTS
    //////////////////////////////////////////////////////////////*/

    /// @notice Run all invariant checks
    function invariant_globalInvariants() public view {
        _invariantOptedInSupplyBounded();
        _invariantDecimalsConstant();
        _invariantSupplyCapEnforced();
    }

    /// @notice TEMPO-TIP19: Opted-in supply <= total supply
    function _invariantOptedInSupplyBounded() internal view {
        for (uint256 i = 0; i < _tokens.length; i++) {
            TIP20 token = _tokens[i];
            assertLe(
                token.optedInSupply(),
                token.totalSupply(),
                "TEMPO-TIP19: Opted-in supply exceeds total supply"
            );
        }
    }

    /// @notice TEMPO-TIP21: Decimals is always 6
    function _invariantDecimalsConstant() internal view {
        for (uint256 i = 0; i < _tokens.length; i++) {
            assertEq(
                _tokens[i].decimals(),
                6,
                "TEMPO-TIP21: Decimals should always be 6"
            );
        }
    }

    /// @notice TEMPO-TIP22: Supply cap is enforced
    function _invariantSupplyCapEnforced() internal view {
        for (uint256 i = 0; i < _tokens.length; i++) {
            TIP20 token = _tokens[i];
            assertLe(
                token.totalSupply(),
                token.supplyCap(),
                "TEMPO-TIP22: Total supply exceeds supply cap"
            );
        }
    }

    /*//////////////////////////////////////////////////////////////
                            HELPERS
    //////////////////////////////////////////////////////////////*/

    /// @dev Checks if an error is known/expected
    function _assertKnownError(bytes memory reason) internal pure {
        assertTrue(_isKnownTIP20Error(bytes4(reason)), "Unknown error encountered");
    }

}
