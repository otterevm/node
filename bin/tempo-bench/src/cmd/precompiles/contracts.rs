//! Solidity benchmark contracts for precompiles

use alloy::sol;

// TIP20 Benchmark Contract
sol! {
    #[sol(rpc)]
    interface ITip20Benchmark {
        function setup(uint256 numAccounts, uint256 initialBalance) external;
        function spamTransfers(uint256 operations) external;
        function spamApprovals(uint256 operations) external;
        function spamTransferFroms(uint256 operations) external;
        function spamMintBurn(uint256 operations) external;
        function spamBalanceChecks(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// TIP20 Factory Benchmark Contract
sol! {
    #[sol(rpc)]
    interface ITip20FactoryBenchmark {
        function setup() external;
        function spamTokenCreation(uint256 operations) external;
        function batchQuery(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// TIP403 Registry Benchmark Contract
sol! {
    #[sol(rpc)]
    interface ITip403Benchmark {
        function setup(uint256 numPolicies) external;
        function spamPolicyCreation(uint256 operations) external;
        function spamAuthChecks(uint256 operations) external;
        function spamWhitelistUpdates(uint256 operations) external;
        function bulkPolicySetup(uint256 accounts) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Fee Manager Benchmark Contract
sol! {
    #[sol(rpc)]
    interface IFeeManagerBenchmark {
        function setup(uint256 numPools) external;
        function spamSwaps(uint256 operations) external;
        function spamLiquidityOps(uint256 operations) external;
        function spamFeeCollection(uint256 operations) external;
        function spamPoolQueries(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Account Registrar Benchmark Contract
sol! {
    #[sol(rpc)]
    interface IAccountRegistrarBenchmark {
        function setup(uint256 numAccounts) external;
        function spamRegistrations(uint256 operations) external;
        function spamDelegations(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Stablecoin Exchange Benchmark Contract
sol! {
    #[sol(rpc)]
    interface IStablecoinExchangeBenchmark {
        function setup(uint256 orderbookDepth) external;
        function spamOrders(uint256 operations) external;
        function spamOrderFlips(uint256 operations) external;
        function spamOrderCancels(uint256 operations) external;
        function setupOrderbook(uint256 depth) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Nonce Benchmark Contract
sol! {
    #[sol(rpc)]
    interface INonceBenchmark {
        function setup(uint256 numAccounts) external;
        function spamNonceUpdates(uint256 operations) external;
        function spamNonceReads(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Validator Config Benchmark Contract
sol! {
    #[sol(rpc)]
    interface IValidatorConfigBenchmark {
        function setup(uint256 numValidators) external;
        function spamValidatorAdditions(uint256 operations) external;
        function spamStatusChanges(uint256 operations) external;
        function spamValidatorQueries(uint256 operations) external;
        function getOperationCount() external view returns (uint256);
    }
}

// Bytecode for the benchmark contracts
// Loaded from compiled Foundry artifacts
use serde_json::Value;

fn extract_bytecode(json_str: &str) -> String {
    let v: Value = serde_json::from_str(json_str).expect("Failed to parse contract JSON");
    v["bytecode"]["object"]
        .as_str()
        .expect("Failed to extract bytecode")
        .to_string()
}

const TIP20_JSON: &str =
    include_str!("../../../contracts/out/TIP20Benchmark.sol/TIP20Benchmark.json");
const TIP20_FACTORY_JSON: &str =
    include_str!("../../../contracts/out/TIP20FactoryBenchmark.sol/TIP20FactoryBenchmark.json");
const TIP403_JSON: &str =
    include_str!("../../../contracts/out/TIP403Benchmark.sol/TIP403Benchmark.json");
const FEE_MANAGER_JSON: &str =
    include_str!("../../../contracts/out/FeeManagerBenchmark.sol/FeeManagerBenchmark.json");
const ACCOUNT_REGISTRAR_JSON: &str = include_str!(
    "../../../contracts/out/AccountRegistrarBenchmark.sol/AccountRegistrarBenchmark.json"
);
const STABLECOIN_EXCHANGE_JSON: &str = include_str!(
    "../../../contracts/out/StablecoinExchangeBenchmark.sol/StablecoinExchangeBenchmark.json"
);
const NONCE_JSON: &str =
    include_str!("../../../contracts/out/NonceBenchmark.sol/NonceBenchmark.json");
const VALIDATOR_CONFIG_JSON: &str = include_str!(
    "../../../contracts/out/ValidatorConfigBenchmark.sol/ValidatorConfigBenchmark.json"
);

pub fn tip20_benchmark_bytecode() -> String {
    extract_bytecode(TIP20_JSON)
}

pub fn tip20_factory_benchmark_bytecode() -> String {
    extract_bytecode(TIP20_FACTORY_JSON)
}

pub fn tip403_benchmark_bytecode() -> String {
    extract_bytecode(TIP403_JSON)
}

pub fn fee_manager_benchmark_bytecode() -> String {
    extract_bytecode(FEE_MANAGER_JSON)
}

pub fn account_registrar_benchmark_bytecode() -> String {
    extract_bytecode(ACCOUNT_REGISTRAR_JSON)
}

pub fn stablecoin_exchange_benchmark_bytecode() -> String {
    extract_bytecode(STABLECOIN_EXCHANGE_JSON)
}

pub fn nonce_benchmark_bytecode() -> String {
    extract_bytecode(NONCE_JSON)
}

pub fn validator_config_benchmark_bytecode() -> String {
    extract_bytecode(VALIDATOR_CONFIG_JSON)
}
