// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface ITipAccountRegistrar {
    function delegateToDefault(address account) external returns (bool);
}

contract AccountRegistrarBenchmark {
    ITipAccountRegistrar public immutable registrar;
    address[] public testAccounts;
    uint256 public operationCounter;

    constructor(address _registrar) {
        registrar = ITipAccountRegistrar(_registrar);
    }

    function setup(uint256 numAccounts) external {
        // Clear existing accounts
        delete testAccounts;

        // Generate test accounts
        for (uint256 i = 0; i < numAccounts; i++) {
            testAccounts.push(address(uint160(0x4000 + i)));
        }
    }

    function spamRegistrations(uint256 operations) external {
        for (uint256 i = 0; i < operations; i++) {
            // Register new accounts with incremental addresses
            address newAccount = address(uint160(0x5000 + operationCounter + i));
            registrar.delegateToDefault(newAccount);
        }
        operationCounter += operations;
    }

    function spamDelegations(uint256 operations) external {
        require(testAccounts.length > 0, "Must have test accounts");

        uint256 accountLen = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            registrar.delegateToDefault(testAccounts[i % accountLen]);
        }
        operationCounter += operations;
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
