// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import { IStablecoinDEX } from "../../src/interfaces/IStablecoinDEX.sol";
import { ITIP20 } from "../../src/interfaces/ITIP20.sol";
import { Test, console2 } from "forge-std/Test.sol";

contract StablecoinDEXInvariantTest is Test {

    IStablecoinDEX public dex = IStablecoinDEX(0xDEc0000000000000000000000000000000000000);
    ITIP20 pathUsd = ITIP20(0x20C0000000000000000000000000000000000000);
    ITIP20 betaUsd = ITIP20(0x20C0000000000000000000000000000000000002);

    address[] private _actors;
    mapping(address => uint128[]) private _placedOrders;
    int16[10] private _ticks = [int16(10), 20, 30, 40, 50, 60, 70, 80, 90, 100];
    uint128 private _nextOrderId;

    function setUp() public {
        vm.createSelectFork(vm.envString("TEMPO_RPC_URL"));
        targetContract(address(this));

        _actors = _buildActors(20);
        _nextOrderId = dex.nextOrderId();
    }

    /// Place ask / bid order and randomly cancel them.
    function placeOrder(uint256 actorRnd, uint128 amount, uint256 tickRnd, bool isBid, bool cancel)
        external
    {
        int16 tick = _ticks[tickRnd % _ticks.length];
        address actor = _actors[actorRnd % _actors.length];
        amount = uint128(bound(amount, 100_000_000, 10_000_000_000));

        _ensureFunds(actor, amount);

        vm.startPrank(actor);
        uint128 orderId = dex.place(address(betaUsd), amount, isBid, tick);
        _assertNextOrderId(orderId);

        uint32 price = dex.tickToPrice(tick);
        uint256 expectedEscrow = (uint256(amount) * uint256(price)) / uint256(dex.PRICE_SCALE());

        if (cancel) {
            dex.cancel(orderId);
            if (isBid) {
                dex.withdraw(address(pathUsd), uint128(expectedEscrow));
            } else {
                dex.withdraw(address(betaUsd), amount);
            }
            // TODO: check TEMPO-DEX2 invariant
        } else {
            // TODO: check TEMPO-DEX3 invariant
            _placedOrders[actor].push(orderId);
        }

        vm.stopPrank();
    }

    /// Place ask / bid flip orders.
    function placeFlipOrder(uint256 actorRnd, uint128 amount, uint256 tickRnd, bool isBid)
        external
    {
        int16 tick = _ticks[tickRnd % _ticks.length];
        address actor = _actors[actorRnd % _actors.length];
        amount = uint128(bound(amount, 100_000_000, 10_000_000_000));

        _ensureFunds(actor, amount);

        vm.startPrank(actor);
        uint128 orderId;
        if (isBid) {
            orderId = dex.placeFlip(address(betaUsd), amount, true, tick, 200);
        } else {
            orderId = dex.placeFlip(address(betaUsd), amount, false, 200, tick);
        }
        _assertNextOrderId(orderId);
        // TODO: check TEMPO-DEX3 invariant
        _placedOrders[actor].push(orderId);

        vm.stopPrank();
    }

    /// Place ask / bid flip orders.
    function swapExactAmount(uint256 swapperRnd, uint128 amount, bool amtIn) external {
        address swapper = _actors[swapperRnd % _actors.length];
        amount = uint128(bound(amount, 100_000_000, 1_000_000_000));

        vm.startPrank(swapper);
        if (amtIn) {
            try dex.swapExactAmountIn(
                address(betaUsd), address(pathUsd), amount, amount - 100
            ) returns (
                uint128 amountOut
            ) {
                // TEMPO-DEX4 invariant
                assertTrue(amountOut >= amount - 100, "swap exact amountOut less than expected");
            } catch { }
        } else {
            try dex.swapExactAmountOut(
                address(betaUsd), address(pathUsd), amount, amount + 100
            ) returns (
                uint128 amountIn
            ) {
                // TEMPO-DEX5 invariant
                assertTrue(amountIn <= amount + 100, "swap exact amountIn less than expected");
            } catch { }
        }
        // Read next order id - if a flip order is hit then next order id is incremented.
        _nextOrderId = dex.nextOrderId();

        vm.stopPrank();
    }

    /// Cancel placed orders (if still active).
    /// TODO: add more exit checks (e.g. liquidity check).
    function afterInvariant() public {
        for (uint256 i = 0; i < _actors.length; i++) {
            address actor = _actors[i];
            vm.startPrank(actor);
            for (uint256 orderId = 0; orderId < _placedOrders[actor].length; orderId++) {
                uint128 placedOrderId = _placedOrders[actor][orderId];
                // Placed orders could be filled and removed.
                try dex.getOrder(placedOrderId) {
                    dex.cancel(placedOrderId);
                    // TODO: check TEMPO-DEX2 invariant
                } catch { }
            }
            vm.stopPrank();
        }
    }

    function invariantStablecoinDEX() public view {
        // TODO: track balances of path usd and beta usd for exchange and for each actor, assert inline with on chain.
        // uint256 dexPathUsdBalance = pathUsd.balanceOf(address(dex));
        // uint256 dexBetaUsdBalance = betaUsd.balanceOf(address(dex));
        //assertEq(dexPathUsdBalance, expectedPathUsdDEXBalance, "pathUSD dex balance different than expected");
    }

    function _assertNextOrderId(uint128 orderId) internal {
        // TEMPO-DEX1 invariant
        assertEq(orderId, _nextOrderId, "next order id mismatch");
        _nextOrderId += 1;
    }

    function _buildActors(uint256 noOfActors_) internal returns (address[] memory) {
        address[] memory actorsAddress = new address[](noOfActors_);

        for (uint256 i = 0; i < noOfActors_; i++) {
            address actor = makeAddr(string(abi.encodePacked("Actor", vm.toString(i))));
            actorsAddress[i] = actor;

            // initial actor balance
            _ensureFunds(actor, 1_000_000_000_000);

            vm.startPrank(actor);
            betaUsd.approve(address(dex), type(uint256).max);
            pathUsd.approve(address(dex), type(uint256).max);
            vm.stopPrank();
        }

        return actorsAddress;
    }

    function _ensureFunds(address actor, uint256 amount) internal {
        vm.startPrank(0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266);
        if (pathUsd.balanceOf(address(actor)) < amount) {
            pathUsd.mint(actor, amount);
        }
        if (betaUsd.balanceOf(address(actor)) < amount) {
            betaUsd.mint(actor, amount);
        }
        vm.stopPrank();
    }

}
