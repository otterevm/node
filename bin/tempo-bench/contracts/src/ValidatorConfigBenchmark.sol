// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface IValidatorConfig {
    function addValidator(bytes32 nodeId, address validator) external;
    function updateValidator(bytes32 nodeId, address newValidator) external;
    function changeValidatorStatus(bytes32 nodeId, bool active) external;
    function getValidators() external view returns (bytes32[] memory, address[] memory);
}

contract ValidatorConfigBenchmark {
    IValidatorConfig public immutable validatorConfig;
    bytes32[] public nodeIds;
    uint256 public operationCounter;

    constructor(address _validatorConfig) {
        validatorConfig = IValidatorConfig(_validatorConfig);
    }

    function setup(uint256 numValidators) external {
        // Clear existing data
        delete nodeIds;

        // Add initial validators
        for (uint256 i = 0; i < numValidators; i++) {
            bytes32 nodeId = keccak256(abi.encodePacked("validator", i));
            validatorConfig.addValidator(nodeId, address(uint160(0x7000 + i)));
            nodeIds.push(nodeId);
        }
    }

    function spamValidatorAdditions(uint256 operations) external {
        for (uint256 i = 0; i < operations; i++) {
            bytes32 nodeId = keccak256(abi.encodePacked("new", operationCounter + i));
            validatorConfig.addValidator(nodeId, address(uint160(0x8000 + operationCounter + i)));
            nodeIds.push(nodeId);
        }
        operationCounter += operations;
    }

    function spamStatusChanges(uint256 operations) external {
        require(nodeIds.length > 0, "Must have validators");

        uint256 nodeLen = nodeIds.length;

        for (uint256 i = 0; i < operations; i++) {
            validatorConfig.changeValidatorStatus(nodeIds[i % nodeLen], i % 2 == 0);
        }
        operationCounter += operations;
    }

    function spamValidatorQueries(uint256 operations) external view returns (uint256 total) {
        for (uint256 i = 0; i < operations; i++) {
            (bytes32[] memory ids, address[] memory addrs) = validatorConfig.getValidators();
            total += ids.length + addrs.length;
        }
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }
}
