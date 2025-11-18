// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface ITIP403Registry {
    enum PolicyType {
        WHITELIST,
        BLACKLIST
    }

    function createPolicy(address admin, PolicyType policyType) external returns (uint256);
    function isAuthorized(uint256 policyId, address user) external view returns (bool);
    function modifyPolicyWhitelist(uint256 policyId, address account, bool allowed) external;
    function modifyPolicyBlacklist(uint256 policyId, address account, bool restricted) external;
}

contract TIP403Benchmark {
    ITIP403Registry public immutable registry;
    uint256[] public policyIds;
    address[] public testAccounts;
    uint256 public operationCounter;

    constructor(address _registry) {
        registry = ITIP403Registry(_registry);
    }

    function setup(uint256 numPolicies) external {
        // Clear existing data
        delete policyIds;
        delete testAccounts;

        // Create initial policies
        for (uint256 i = 0; i < numPolicies; i++) {
            uint256 policyId = registry.createPolicy(msg.sender, ITIP403Registry.PolicyType.WHITELIST);
            policyIds.push(policyId);
        }

        // Create test accounts
        for (uint256 i = 0; i < 100; i++) {
            testAccounts.push(address(uint160(0x2000 + i)));
        }
    }

    function spamPolicyCreation(uint256 operations) external {
        for (uint256 i = 0; i < operations; i++) {
            uint256 policyId = registry.createPolicy(
                msg.sender, i % 2 == 0 ? ITIP403Registry.PolicyType.WHITELIST : ITIP403Registry.PolicyType.BLACKLIST
            );
            policyIds.push(policyId);
        }
        operationCounter += operations;
    }

    function spamAuthChecks(uint256 operations) external view returns (uint256 authorizedCount) {
        require(policyIds.length > 0, "Must have policies");
        require(testAccounts.length > 0, "Must have test accounts");

        uint256 policyLen = policyIds.length;
        uint256 accountLen = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            if (registry.isAuthorized(policyIds[i % policyLen], testAccounts[i % accountLen])) {
                authorizedCount++;
            }
        }
    }

    function spamWhitelistUpdates(uint256 operations) external {
        require(policyIds.length > 0, "Must have policies");
        require(testAccounts.length > 0, "Must have test accounts");

        uint256 policyLen = policyIds.length;
        uint256 accountLen = testAccounts.length;

        for (uint256 i = 0; i < operations; i++) {
            registry.modifyPolicyWhitelist(policyIds[i % policyLen], testAccounts[i % accountLen], i % 2 == 0);
        }
        operationCounter += operations;
    }

    function bulkPolicySetup(uint256 accounts) external {
        uint256 policyId = registry.createPolicy(msg.sender, ITIP403Registry.PolicyType.WHITELIST);

        for (uint256 i = 0; i < accounts; i++) {
            registry.modifyPolicyWhitelist(policyId, address(uint160(0x3000 + i)), true);
        }

        policyIds.push(policyId);
        operationCounter += accounts;
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
