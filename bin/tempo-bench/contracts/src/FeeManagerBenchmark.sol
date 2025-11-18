// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IFeeManager {
    function setValidatorToken(address token) external;
    function setUserToken(address token) external;
    function getFeeTokenBalance(address token) external view returns (uint256);

    // AMM functions
    function swap(bytes32 poolId, address tokenIn, address tokenOut, uint256 amountIn, uint256 minAmountOut)
        external
        returns (uint256);

    function addLiquidity(bytes32 poolId, uint256 amount0, uint256 amount1) external returns (uint256);

    function removeLiquidity(bytes32 poolId, uint256 liquidity) external returns (uint256, uint256);

    function getPoolId(address token0, address token1) external pure returns (bytes32);
}

contract FeeManagerBenchmark {
    IFeeManager public immutable feeManager;
    address[] public tokens;
    bytes32[] public poolIds;
    uint256 public operationCounter;

    constructor(address _feeManager) {
        feeManager = IFeeManager(_feeManager);
    }

    function setup(uint256 numPools) external {
        // Clear existing data
        delete tokens;
        delete poolIds;

        // Setup test tokens (using TIP20 addresses)
        for (uint256 i = 0; i < numPools + 1; i++) {
            tokens.push(address(uint160(0x2000000000000000000000000000000000000001) + uint160(i)));
        }

        // Create pool IDs for pairs
        for (uint256 i = 0; i < numPools; i++) {
            bytes32 poolId = feeManager.getPoolId(tokens[i], tokens[i + 1]);
            poolIds.push(poolId);
        }
    }

    function spamSwaps(uint256 operations) external {
        require(poolIds.length > 0, "Must have pools");
        require(tokens.length > 1, "Must have tokens");

        uint256 poolLen = poolIds.length;
        uint256 tokenLen = tokens.length;

        for (uint256 i = 0; i < operations; i++) {
            feeManager.swap(
                poolIds[i % poolLen],
                tokens[i % tokenLen],
                tokens[(i + 1) % tokenLen],
                1000,
                900 // Allow some slippage
            );
        }
        operationCounter += operations;
    }

    function spamLiquidityOps(uint256 operations) external {
        require(poolIds.length > 0, "Must have pools");

        uint256 poolLen = poolIds.length;

        for (uint256 i = 0; i < operations; i++) {
            if (i % 2 == 0) {
                // Add liquidity
                feeManager.addLiquidity(poolIds[i % poolLen], 1000, 1000);
            } else {
                // Remove liquidity
                feeManager.removeLiquidity(poolIds[i % poolLen], 100);
            }
        }
        operationCounter += operations;
    }

    function spamFeeCollection(uint256 operations) external view returns (uint256 total) {
        require(tokens.length > 0, "Must have tokens");

        uint256 tokenLen = tokens.length;

        for (uint256 i = 0; i < operations; i++) {
            total += feeManager.getFeeTokenBalance(tokens[i % tokenLen]);
        }
    }

    function spamPoolQueries(uint256 operations) external view returns (bytes32[] memory ids) {
        require(tokens.length > 1, "Must have tokens");

        uint256 tokenLen = tokens.length;
        ids = new bytes32[](operations);

        for (uint256 i = 0; i < operations; i++) {
            ids[i] = feeManager.getPoolId(tokens[i % tokenLen], tokens[(i + 1) % tokenLen]);
        }
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
