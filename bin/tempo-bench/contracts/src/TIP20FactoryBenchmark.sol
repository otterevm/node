// SPDX-License-Identifier: MIT
pragma solidity ^0.8.26;

interface ITIP20Factory {
    function createToken(
        string memory name,
        string memory symbol,
        string memory currency,
        address quoteToken,
        address admin
    ) external returns (uint256);

    function tokenIdCounter() external view returns (uint256);
}

contract TIP20FactoryBenchmark {
    ITIP20Factory public immutable factory;
    uint256 public operationCounter;
    uint256 public tokenCounter;
    address public constant QUOTE_TOKEN = address(0x2000000000000000000000000000000000000000); // LINKING_USD

    constructor(address _factory) {
        factory = ITIP20Factory(_factory);
    }

    function setup() external {
        // Reset counter
        tokenCounter = 0;
    }

    function spamTokenCreation(uint256 operations) external {
        for (uint256 i = 0; i < operations; i++) {
            factory.createToken(
                string(abi.encodePacked("Token", uint2str(tokenCounter))),
                string(abi.encodePacked("TK", uint2str(tokenCounter))),
                "USD",
                QUOTE_TOKEN,
                msg.sender
            );
            tokenCounter++;
        }
        operationCounter += operations;
    }

    function batchQuery(uint256 operations) external view returns (uint256 total) {
        for (uint256 i = 0; i < operations; i++) {
            total += factory.tokenIdCounter();
        }
    }

    function getOperationCount() external view returns (uint256) {
        return operationCounter;
    }

    // Helper function to convert uint to string
    function uint2str(uint256 _i) internal pure returns (string memory) {
        if (_i == 0) {
            return "0";
        }
        uint256 j = _i;
        uint256 len;
        while (j != 0) {
            len++;
            j /= 10;
        }
        bytes memory bstr = new bytes(len);
        uint256 k = len;
        while (_i != 0) {
            k = k - 1;
            uint8 temp = (48 + uint8(_i - _i / 10 * 10));
            bytes1 b1 = bytes1(temp);
            bstr[k] = b1;
            _i /= 10;
        }
        return string(bstr);
    }
}
