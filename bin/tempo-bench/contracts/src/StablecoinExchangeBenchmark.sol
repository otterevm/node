// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IStablecoinExchange {
    function place(address token, uint256 amount, bool isBid, int256 tick) external returns (uint256);

    function placeFlip(address token, uint256 amount, bool isBid, int256 tick, int256 flipTick)
        external
        returns (uint256);

    function cancel(uint256 orderId) external;
    function balanceOf(address user, address token) external view returns (uint256);
}

contract StablecoinExchangeBenchmark {
    IStablecoinExchange public immutable exchange;
    address[] public tokens;
    uint256[] public orderIds;
    uint256 public operationCounter;

    constructor(address _exchange) {
        exchange = IStablecoinExchange(_exchange);
    }

    function setup(uint256 orderbookDepth) external {
        // Clear existing data
        delete tokens;
        delete orderIds;

        // Setup test tokens
        tokens.push(address(uint160(0x2000000000000000000000000000000000000001))); // Token 1
        tokens.push(address(uint160(0x2000000000000000000000000000000000000002))); // Token 2

        // No need to setup orderbook depth here as it would require actual orders
        // This would be done in the actual test setup
    }

    function setupOrderbook(uint256 depth) external {
        // Place orders at different price levels to create depth
        for (uint256 i = 0; i < depth; i++) {
            uint256 orderId = exchange.place(
                tokens[0],
                1000,
                true, // bid
                int256(100 + int256(i))
            );
            orderIds.push(orderId);

            orderId = exchange.place(
                tokens[0],
                1000,
                false, // ask
                int256(110 + int256(i))
            );
            orderIds.push(orderId);
        }
        operationCounter += depth * 2;
    }

    function spamOrders(uint256 operations) external {
        require(tokens.length > 0, "Must have tokens");

        uint256 tokenLen = tokens.length;

        for (uint256 i = 0; i < operations; i++) {
            uint256 orderId = exchange.place(tokens[i % tokenLen], 100, i % 2 == 0, int256(100 + int256(i % 10)));
            orderIds.push(orderId);
        }
        operationCounter += operations;
    }

    function spamOrderFlips(uint256 operations) external {
        require(tokens.length > 0, "Must have tokens");

        uint256 tokenLen = tokens.length;

        for (uint256 i = 0; i < operations; i++) {
            uint256 orderId = exchange.placeFlip(
                tokens[i % tokenLen], 100, i % 2 == 0, int256(100 + int256(i % 10)), int256(110 + int256(i % 10))
            );
            orderIds.push(orderId);
        }
        operationCounter += operations;
    }

    function spamOrderCancels(uint256 operations) external {
        require(orderIds.length > 0, "Must have orders");

        uint256 orderLen = orderIds.length;

        for (uint256 i = 0; i < operations && i < orderLen; i++) {
            exchange.cancel(orderIds[i]);
        }
        operationCounter += operations;
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
