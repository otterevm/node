//! Nonce precompile benchmarking implementation

use alloy::{
    primitives::{Address, U256},
    providers::Provider,
    sol_types::SolCall,
};
use alloy_signer_local::PrivateKeySigner;
use eyre::Result;
use std::{collections::HashMap, sync::Arc, time::Instant};

use super::{
    BenchmarkScenario,
    contracts::INonceBenchmark,
    framework::{
        BaseBenchmarker, BenchmarkResult, OperationBreakdown, PrecompileBenchmarker,
        TransactionRequest,
    },
};

pub struct NonceBenchmarker<P: Provider> {
    base: BaseBenchmarker<P>,
}

impl<P: Provider> NonceBenchmarker<P> {
    pub fn new(
        provider: Arc<P>,
        signers: Vec<PrivateKeySigner>,
        chain_id: u64,
        ops_per_tx: u64,
        max_gas_per_tx: u64,
    ) -> Self {
        Self {
            base: BaseBenchmarker::new(provider, signers, chain_id, ops_per_tx, max_gas_per_tx),
        }
    }
}

#[async_trait::async_trait]
impl<P: Provider + Send + Sync> PrecompileBenchmarker for NonceBenchmarker<P> {
    async fn setup_state(&mut self) -> Result<()> {
        println!("Setting up Nonce benchmark state...");
        Ok(())
    }

    async fn deploy_contracts(&mut self) -> Result<()> {
        println!("Deploying Nonce benchmark contract...");
        self.base.contract_address = Some(Address::from([0x48; 20]));
        Ok(())
    }

    async fn generate_transactions(&self, ops_per_tx: u64) -> Result<Vec<TransactionRequest>> {
        let mut transactions = Vec::new();

        let calldata = INonceBenchmark::spamNonceUpdatesCall {
            operations: U256::from(ops_per_tx),
        }
        .abi_encode();

        let tx = TransactionRequest {
            from: self.base.signers[0].address(),
            to: self.base.contract_address.unwrap(),
            data: calldata,
            gas: self.base.max_gas_per_tx,
            operation_count: ops_per_tx,
            operation_type: "setNonce".to_string(),
        };

        transactions.push(tx);
        Ok(transactions)
    }

    async fn run_benchmark(
        &mut self,
        scenario: BenchmarkScenario,
        target_ops: u64,
        _duration: u64,
    ) -> Result<BenchmarkResult> {
        println!("Running Nonce benchmark - Scenario: {:?}", scenario);
        let start_time = Instant::now();

        match scenario {
            BenchmarkScenario::Mixed => {
                // Mix reads and writes
                let writes = target_ops / 2;
                let reads = target_ops - writes;

                let mut txs = Vec::new();

                // Nonce updates
                let calldata = INonceBenchmark::spamNonceUpdatesCall {
                    operations: U256::from(writes),
                }
                .abi_encode();
                txs.push(TransactionRequest {
                    from: self.base.signers[0].address(),
                    to: self.base.contract_address.unwrap(),
                    data: calldata,
                    gas: self.base.max_gas_per_tx,
                    operation_count: writes,
                    operation_type: "setNonce".to_string(),
                });

                // Nonce reads
                let calldata = INonceBenchmark::spamNonceReadsCall {
                    operations: U256::from(reads),
                }
                .abi_encode();
                txs.push(TransactionRequest {
                    from: self.base.signers[0].address(),
                    to: self.base.contract_address.unwrap(),
                    data: calldata,
                    gas: self.base.max_gas_per_tx,
                    operation_count: reads,
                    operation_type: "getNonce".to_string(),
                });

                self.base.execute_transactions(txs).await?;
            }
            _ => {
                let transactions = self.generate_transactions(self.base.ops_per_tx).await?;
                self.base.execute_transactions(transactions).await?;
            }
        }

        let total_duration = start_time.elapsed();
        self.base.metrics.calculate_ops_per_second(total_duration);

        let mut breakdown = HashMap::new();
        breakdown.insert(
            "setNonce".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 2,
                gas_per_op: 20000.0,
                total_gas: (self.base.metrics.total_operations / 2) * 20000,
            },
        );
        breakdown.insert(
            "getNonce".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 2,
                gas_per_op: 5000.0,
                total_gas: (self.base.metrics.total_operations / 2) * 5000,
            },
        );

        Ok(BenchmarkResult {
            precompile: "Nonce".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            scenario: format!("{:?}", scenario),
            metrics: self.base.metrics.clone(),
            breakdown,
        })
    }
}
