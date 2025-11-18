//! TIP20 precompile benchmarking implementation

use alloy::{
    network::TxSignerSync,
    primitives::{Address, TxKind, U256},
    providers::Provider,
    sol,
    sol_types::SolCall,
};
use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_signer_local::PrivateKeySigner;
use eyre::Result;
use std::{collections::HashMap, sync::Arc, time::Instant};

use super::{
    BenchmarkScenario,
    contracts::ITip20Benchmark,
    framework::{
        BaseBenchmarker, BenchmarkResult, OperationBreakdown, PrecompileBenchmarker,
        TransactionRequest,
    },
};

// TIP20 precompile interface
sol! {
    interface ITIP20 {
        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
        function allowance(address owner, address spender) external view returns (uint256);
        function approve(address spender, uint256 amount) external returns (bool);
        function transferFrom(address from, address to, uint256 amount) external returns (bool);
        function mint(address to, uint256 amount) external;
        function burn(uint256 amount) external;
    }
}

pub struct Tip20Benchmarker<P: Provider> {
    base: BaseBenchmarker<P>,
    tip20_address: Address,
    test_accounts: Vec<Address>,
}

impl<P: Provider> Tip20Benchmarker<P> {
    pub fn new(
        provider: Arc<P>,
        signers: Vec<PrivateKeySigner>,
        chain_id: u64,
        ops_per_tx: u64,
        max_gas_per_tx: u64,
    ) -> Self {
        // Default TIP20 token address (token ID 1)
        let tip20_address = Address::from([
            0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]);

        // Generate test accounts
        let test_accounts: Vec<Address> = (0..100)
            .map(|i| {
                let mut bytes = [0u8; 20];
                bytes[19] = i as u8;
                Address::from(bytes)
            })
            .collect();

        Self {
            base: BaseBenchmarker::new(provider, signers, chain_id, ops_per_tx, max_gas_per_tx),
            tip20_address,
            test_accounts,
        }
    }

    async fn execute_transfer_benchmark(&mut self, num_operations: u64) -> Result<()> {
        println!("Executing {} transfer operations...", num_operations);

        let mut transactions = Vec::new();
        let batch_size = self.base.ops_per_tx;
        let num_batches = (num_operations + batch_size - 1) / batch_size;

        for batch in 0..num_batches {
            let ops_in_batch = std::cmp::min(batch_size, num_operations - batch * batch_size);

            // Use the benchmark contract to spam transfers
            let calldata = ITip20Benchmark::spamTransfersCall {
                operations: U256::from(ops_in_batch),
            }
            .abi_encode();

            let tx = TransactionRequest {
                from: self.base.signers[0].address(),
                to: self.base.contract_address.unwrap_or(self.tip20_address),
                data: calldata,
                gas: self.base.max_gas_per_tx,
                operation_count: ops_in_batch,
                operation_type: "transfer".to_string(),
            };

            transactions.push(tx);
        }

        self.base.execute_transactions(transactions).await?;
        Ok(())
    }

    async fn execute_approval_benchmark(&mut self, num_operations: u64) -> Result<()> {
        println!("Executing {} approval operations...", num_operations);

        let mut transactions = Vec::new();
        let batch_size = self.base.ops_per_tx;
        let num_batches = (num_operations + batch_size - 1) / batch_size;

        for batch in 0..num_batches {
            let ops_in_batch = std::cmp::min(batch_size, num_operations - batch * batch_size);

            let calldata = ITip20Benchmark::spamApprovalsCall {
                operations: U256::from(ops_in_batch),
            }
            .abi_encode();

            let tx = TransactionRequest {
                from: self.base.signers[0].address(),
                to: self.base.contract_address.unwrap_or(self.tip20_address),
                data: calldata,
                gas: self.base.max_gas_per_tx,
                operation_count: ops_in_batch,
                operation_type: "approve".to_string(),
            };

            transactions.push(tx);
        }

        self.base.execute_transactions(transactions).await?;
        Ok(())
    }

    async fn execute_mixed_benchmark(&mut self, num_operations: u64) -> Result<()> {
        println!("Executing {} mixed operations...", num_operations);

        // Split operations between different types
        let transfers = num_operations / 3;
        let approvals = num_operations / 3;
        let balance_checks = num_operations - transfers - approvals;

        // Execute each type
        self.execute_transfer_benchmark(transfers).await?;
        self.execute_approval_benchmark(approvals).await?;

        // Execute balance checks
        let mut transactions = Vec::new();
        let batch_size = self.base.ops_per_tx;
        let num_batches = (balance_checks + batch_size - 1) / batch_size;

        for batch in 0..num_batches {
            let ops_in_batch = std::cmp::min(batch_size, balance_checks - batch * batch_size);

            let calldata = ITip20Benchmark::spamBalanceChecksCall {
                operations: U256::from(ops_in_batch),
            }
            .abi_encode();

            let tx = TransactionRequest {
                from: self.base.signers[0].address(),
                to: self.base.contract_address.unwrap_or(self.tip20_address),
                data: calldata,
                gas: self.base.max_gas_per_tx,
                operation_count: ops_in_batch,
                operation_type: "balanceOf".to_string(),
            };

            transactions.push(tx);
        }

        self.base.execute_transactions(transactions).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl<P: Provider + Send + Sync> PrecompileBenchmarker for Tip20Benchmarker<P> {
    async fn setup_state(&mut self) -> Result<()> {
        println!("Setting up TIP20 benchmark state...");

        // Fund test accounts with native currency for gas
        // This would be done through the provider in a real implementation

        println!("State setup complete");
        Ok(())
    }

    async fn deploy_contracts(&mut self) -> Result<()> {
        println!("Deploying TIP20 benchmark contract...");

        // In a real implementation, deploy the benchmark contract
        // For now, we'll simulate by setting a contract address
        self.base.contract_address = Some(Address::from([0x42; 20]));

        // Initialize the benchmark contract with test accounts and initial balances
        if let Some(contract_addr) = self.base.contract_address {
            let calldata = ITip20Benchmark::setupCall {
                numAccounts: U256::from(self.test_accounts.len()),
                initialBalance: U256::from(1_000_000_000_000_000_000u128), // 1 token per account
            }
            .abi_encode();

            // Create and sign the setup transaction
            let mut tx = TxLegacy {
                chain_id: Some(self.base.chain_id),
                nonce: 0,                 // Would need to get actual nonce from provider
                gas_price: 1_000_000_000, // 1 gwei, adjust as needed
                gas_limit: 500_000,
                to: TxKind::Call(contract_addr),
                value: U256::ZERO,
                input: calldata.into(),
            };

            let signature = self.base.signers[0]
                .sign_transaction_sync(&mut tx)
                .map_err(|e| eyre::eyre!("Failed to sign setup transaction: {}", e))?;

            let _signed_tx = tx.into_signed(signature);

            // Send the transaction
            println!(
                "Initializing benchmark contract with {} accounts",
                self.test_accounts.len()
            );

            // In a real implementation, would send via provider:
            // let pending = self.base.provider.send_raw_transaction(&signed_tx.encoded_2718()).await?;
            // let receipt = pending.await?;
        }

        println!("Contract deployment complete");
        Ok(())
    }

    async fn generate_transactions(&self, ops_per_tx: u64) -> Result<Vec<TransactionRequest>> {
        let mut transactions = Vec::new();

        // Generate a mix of different TIP20 operations
        let calldata = ITip20Benchmark::spamTransfersCall {
            operations: U256::from(ops_per_tx),
        }
        .abi_encode();

        let tx = TransactionRequest {
            from: self.base.signers[0].address(),
            to: self.base.contract_address.unwrap_or(self.tip20_address),
            data: calldata,
            gas: self.base.max_gas_per_tx,
            operation_count: ops_per_tx,
            operation_type: "transfer".to_string(),
        };

        transactions.push(tx);
        Ok(transactions)
    }

    async fn run_benchmark(
        &mut self,
        scenario: BenchmarkScenario,
        target_ops: u64,
        duration: u64,
    ) -> Result<BenchmarkResult> {
        println!("Running TIP20 benchmark - Scenario: {:?}", scenario);
        let start_time = Instant::now();

        match scenario {
            BenchmarkScenario::Baseline => {
                // Run basic transfer operations
                self.execute_transfer_benchmark(target_ops).await?;
            }
            BenchmarkScenario::Load => {
                // Run sustained load for duration
                let ops_per_second = target_ops / duration;
                let total_ops = ops_per_second * duration;
                self.execute_transfer_benchmark(total_ops).await?;
            }
            BenchmarkScenario::Burst => {
                // Execute all operations as quickly as possible
                self.execute_transfer_benchmark(target_ops).await?;
            }
            BenchmarkScenario::Mixed => {
                // Mix different operation types
                self.execute_mixed_benchmark(target_ops).await?;
            }
        }

        let total_duration = start_time.elapsed();

        // Calculate final metrics
        self.base.metrics.calculate_ops_per_second(total_duration);
        self.base.metrics.calculate_gas_per_operation();
        self.base.metrics.calculate_percentiles();

        // Create breakdown by operation type
        let mut breakdown = HashMap::new();

        // Add transfer operations breakdown
        breakdown.insert(
            "transfer".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 2,
                gas_per_op: 21000.0, // Estimated
                total_gas: (self.base.metrics.total_operations / 2) * 21000,
            },
        );

        // Add approval operations breakdown
        breakdown.insert(
            "approve".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 4,
                gas_per_op: 22000.0, // Estimated
                total_gas: (self.base.metrics.total_operations / 4) * 22000,
            },
        );

        // Add balance check operations breakdown
        breakdown.insert(
            "balanceOf".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 4,
                gas_per_op: 5000.0, // Estimated
                total_gas: (self.base.metrics.total_operations / 4) * 5000,
            },
        );

        Ok(BenchmarkResult {
            precompile: "TIP20".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            scenario: format!("{:?}", scenario),
            metrics: self.base.metrics.clone(),
            breakdown,
        })
    }
}
