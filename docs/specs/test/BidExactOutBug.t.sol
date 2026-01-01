// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import { IStablecoinExchange } from "../src/interfaces/IStablecoinExchange.sol";
import { ITIP20 } from "../src/interfaces/ITIP20.sol";
import { BaseTest } from "./BaseTest.t.sol";
import { console } from "forge-std/console.sol";

/**
 * @title BidExactOutBug
 * @notice Proof-of-concept test demonstrating a bug where exactOut for the full
 *         available quote from a bid order does NOT fully fill the order.
 *
 * The bug: When a taker does swapExactAmountOut to get all available quote from a bid,
 * the baseNeeded calculation uses floor division, which can be less than the order amount.
 * This leaves 1 unit of base remaining in the order even though all quote was released.
 *
 * Example at tick = -2000 (price = 0.98), base = 100_000_051:
 * - escrow = ceil(100_000_051 * 0.98) = 98_000_050
 * - release = floor(100_000_051 * 0.98) = 98_000_049
 * - baseNeeded = ceil(98_000_049 / 0.98) = ceil(99_999_999.98) = 100_000_000
 * - baseNeeded (100_000_000) < base (100_000_051) - BUG!
 */
contract BidExactOutBugTest is BaseTest {

    uint128 constant INITIAL_BALANCE = 1_000_000_000;
    uint64 constant PRICE_SCALE = 100_000;

    function setUp() public override {
        super.setUp();

        vm.startPrank(admin);
        token1.grantRole(_ISSUER_ROLE, admin);
        token1.mint(alice, INITIAL_BALANCE);
        token1.mint(bob, INITIAL_BALANCE);
        vm.stopPrank();

        vm.startPrank(pathUSDAdmin);
        pathUSD.grantRole(_ISSUER_ROLE, pathUSDAdmin);
        pathUSD.mint(alice, INITIAL_BALANCE);
        pathUSD.mint(bob, INITIAL_BALANCE);
        vm.stopPrank();

        vm.startPrank(alice);
        token1.approve(address(exchange), type(uint256).max);
        pathUSD.approve(address(exchange), type(uint256).max);
        vm.stopPrank();

        vm.startPrank(bob);
        token1.approve(address(exchange), type(uint256).max);
        pathUSD.approve(address(exchange), type(uint256).max);
        vm.stopPrank();

        exchange.createPair(address(token1));
    }

    function test_BidExactOutBug_FullEscrowDoesNotFullyFillOrder() public {
        // Values that trigger the rounding issue
        uint128 baseAmount = 100_000_051;
        int16 tick = -2000; // price = 98000, p = 0.98

        uint32 price = exchange.tickToPrice(tick);
        
        // Calculate escrow (ceil) and release (floor)
        uint128 escrow = uint128((uint256(baseAmount) * uint256(price) + PRICE_SCALE - 1) / PRICE_SCALE);
        uint128 release = uint128((uint256(baseAmount) * uint256(price)) / PRICE_SCALE);

        // Verify our math
        assertEq(escrow, 98_000_050, "Escrow should be 98_000_050");
        assertEq(release, 98_000_049, "Release should be 98_000_049");

        // Alice places a bid for baseAmount base tokens, escrowing quote
        vm.prank(alice);
        uint128 orderId = exchange.place(address(token1), baseAmount, true, tick);

        // Bob does exactOut for the full release amount
        vm.prank(bob);
        uint128 baseIn = exchange.swapExactAmountOut(
            address(token1),  // tokenIn = base (bob sells base)
            address(pathUSD), // tokenOut = quote (bob receives quote)
            release,          // amountOut = 98_000_049 quote
            type(uint128).max // maxAmountIn
        );

        // THE BUG: baseIn is 100_000_050, not 100_000_051
        // This leaves 1 base remaining in the order
        console.log("Base amount in order:", baseAmount);
        console.log("Base consumed by swap:", baseIn);
        console.log("Base remaining in order:", baseAmount - baseIn);
        console.log("Expected remaining: 0");

        // This assertion FAILS on the buggy code
        assertEq(
            baseIn,
            baseAmount,
            "Order should be fully filled when taker takes all available quote"
        );
    }
}

