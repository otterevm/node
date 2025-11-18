// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface INonceManager {
    function setNonce(address account, uint256 nonce) external;
    function getNonce(address account) external view returns (uint256);
}

contract NonceBenchmark {
    INonceManager public immutable nonceManager;
    address[] public testAccounts;
    uint256 public operationCounter;

    constructor(address _nonceManager) {
        nonceManager = INonceManager(_nonceManager);
    }

    function setup(uint256 numAccounts) external {
        // Clear existing accounts
        delete testAccounts;

        // Generate test accounts
        for (uint256 i = 0; i < numAccounts; i++) {
            testAccounts.push(address(uint160(0x6000 + i)));
        }
    }

    function spamNonceUpdates(uint256 operations) external {
        require(testAccounts.length > 0, "Must have test accounts");

        uint256 accountLen = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            nonceManager.setNonce(testAccounts[i % accountLen], i);
        }
        operationCounter += operations;
    }

    function spamNonceReads(uint256 operations) external view returns (uint256 total) {
        require(testAccounts.length > 0, "Must have test accounts");

        uint256 accountLen = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            total += nonceManager.getNonce(testAccounts[i % accountLen]);
        }
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
