//! TIP20 Factory precompile benchmarking implementation

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
    contracts::ITip20FactoryBenchmark,
    framework::{
        BaseBenchmarker, BenchmarkResult, OperationBreakdown, PrecompileBenchmarker,
        TransactionRequest,
    },
};

pub struct Tip20FactoryBenchmarker<P: Provider> {
    base: BaseBenchmarker<P>,
}

impl<P: Provider> Tip20FactoryBenchmarker<P> {
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
impl<P: Provider + Send + Sync> PrecompileBenchmarker for Tip20FactoryBenchmarker<P> {
    async fn setup_state(&mut self) -> Result<()> {
        println!("Setting up TIP20 Factory benchmark state...");
        Ok(())
    }

    async fn deploy_contracts(&mut self) -> Result<()> {
        println!("Deploying TIP20 Factory benchmark contract...");
        self.base.contract_address = Some(Address::from([0x43; 20]));
        Ok(())
    }

    async fn generate_transactions(&self, ops_per_tx: u64) -> Result<Vec<TransactionRequest>> {
        let mut transactions = Vec::new();

        let calldata = ITip20FactoryBenchmark::spamTokenCreationCall {
            operations: U256::from(ops_per_tx),
        }
        .abi_encode();

        let tx = TransactionRequest {
            from: self.base.signers[0].address(),
            to: self.base.contract_address.unwrap(),
            data: calldata,
            gas: self.base.max_gas_per_tx,
            operation_count: ops_per_tx,
            operation_type: "createToken".to_string(),
        };

        transactions.push(tx);
        Ok(transactions)
    }

    async fn run_benchmark(
        &mut self,
        scenario: BenchmarkScenario,
        _target_ops: u64,
        _duration: u64,
    ) -> Result<BenchmarkResult> {
        println!("Running TIP20 Factory benchmark - Scenario: {:?}", scenario);
        let start_time = Instant::now();

        // Generate and execute transactions based on scenario
        let transactions = self.generate_transactions(self.base.ops_per_tx).await?;
        self.base.execute_transactions(transactions).await?;

        let total_duration = start_time.elapsed();
        self.base.metrics.calculate_ops_per_second(total_duration);

        let mut breakdown = HashMap::new();
        breakdown.insert(
            "createToken".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations,
                gas_per_op: 150000.0,
                total_gas: self.base.metrics.total_gas_used,
            },
        );

        Ok(BenchmarkResult {
            precompile: "TIP20Factory".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            scenario: format!("{:?}", scenario),
            metrics: self.base.metrics.clone(),
            breakdown,
        })
    }
}
