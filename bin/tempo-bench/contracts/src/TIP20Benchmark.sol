// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface ITIP20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function transferFrom(address from, address to, uint256 amount) external returns (bool);
    function mint(address to, uint256 amount) external;
    function burn(uint256 amount) external;
    function balanceOf(address account) external view returns (uint256);
    function allowance(address owner, address spender) external view returns (uint256);
    function totalSupply() external view returns (uint256);
}

contract TIP20Benchmark {
    ITIP20 public immutable token;
    address[] public testAccounts;
    uint256 public operationCounter;

    constructor(address _token) {
        token = ITIP20(_token);
    }

    function setup(uint256 numAccounts, uint256 initialBalance) external {
        // Clear existing accounts
        delete testAccounts;

        // Generate deterministic test accounts
        for (uint256 i = 0; i < numAccounts; i++) {
            address account = address(uint160(0x1000 + i));
            testAccounts.push(account);

            // Mint initial balance to each account if needed
            if (initialBalance > 0) {
                token.mint(account, initialBalance);
            }
        }
    }

    function spamTransfers(uint256 operations) external {
        require(testAccounts.length > 0, "Must call setup first");
        uint256 len = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            token.transfer(testAccounts[i % len], 1);
        }
        operationCounter += operations;
    }

    function spamApprovals(uint256 operations) external {
        require(testAccounts.length > 0, "Must call setup first");
        uint256 len = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            token.approve(testAccounts[i % len], i + 1);
        }
        operationCounter += operations;
    }

    function spamTransferFroms(uint256 operations) external {
        require(testAccounts.length > 1, "Need at least 2 accounts");
        uint256 len = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            address from = testAccounts[i % len];
            address to = testAccounts[(i + 1) % len];
            token.transferFrom(from, to, 1);
        }
        operationCounter += operations;
    }

    function spamMintBurn(uint256 operations) external {
        require(testAccounts.length > 0, "Must call setup first");
        uint256 len = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            if (i % 2 == 0) {
                token.mint(testAccounts[i % len], 100);
            } else {
                token.burn(100);
            }
        }
        operationCounter += operations;
    }

    function spamBalanceChecks(uint256 operations) external view returns (uint256 total) {
        require(testAccounts.length > 0, "Must call setup first");
        uint256 len = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            total += token.balanceOf(testAccounts[i % len]);
        }
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
