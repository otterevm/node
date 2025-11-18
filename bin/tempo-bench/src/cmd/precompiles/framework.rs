//! Core framework for precompile benchmarking

use alloy::{primitives::Address, providers::Provider};
use alloy_signer_local::PrivateKeySigner;
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use super::BenchmarkScenario;

/// Trait for implementing precompile benchmarkers
#[async_trait::async_trait]
pub trait PrecompileBenchmarker {
    /// Setup initial state for benchmarking
    async fn setup_state(&mut self) -> Result<()>;

    /// Deploy benchmark contracts
    async fn deploy_contracts(&mut self) -> Result<()>;

    /// Generate transactions for the benchmark
    async fn generate_transactions(&self, ops_per_tx: u64) -> Result<Vec<TransactionRequest>>;

    /// Run the benchmark with specified parameters
    async fn run_benchmark(
        &mut self,
        scenario: BenchmarkScenario,
        target_ops: u64,
        duration: u64,
    ) -> Result<BenchmarkResult>;
}

/// Transaction request for benchmarking
#[derive(Debug, Clone)]
pub struct TransactionRequest {
    pub from: Address,
    pub to: Address,
    pub data: Vec<u8>,
    pub gas: u64,
    pub operation_count: u64,
    pub operation_type: String,
}

/// Metrics collected during benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    pub total_operations: u64,
    pub operations_per_transaction: u64,
    pub operations_per_block: u64,
    pub operations_per_second: f64,
    pub gas_per_operation: f64,
    pub total_gas_used: u64,
    pub blocks_processed: u64,
    pub transactions_sent: u64,
    pub p50_latency: Duration,
    pub p99_latency: Duration,
    pub execution_times: Vec<Duration>,
    pub gas_used_per_tx: Vec<u64>,
}

impl Default for BenchmarkMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            operations_per_transaction: 0,
            operations_per_block: 0,
            operations_per_second: 0.0,
            gas_per_operation: 0.0,
            total_gas_used: 0,
            blocks_processed: 0,
            transactions_sent: 0,
            p50_latency: Duration::from_millis(0),
            p99_latency: Duration::from_millis(0),
            execution_times: Vec::new(),
            gas_used_per_tx: Vec::new(),
        }
    }
}

impl BenchmarkMetrics {
    /// Calculate percentile latencies from execution times
    pub fn calculate_percentiles(&mut self) {
        if self.execution_times.is_empty() {
            return;
        }

        let mut times = self.execution_times.clone();
        times.sort();

        let p50_idx = times.len() / 2;
        let p99_idx = (times.len() * 99) / 100;

        self.p50_latency = times[p50_idx];
        self.p99_latency = times
            .get(p99_idx)
            .copied()
            .unwrap_or(times[times.len() - 1]);
    }

    /// Calculate operations per second from total duration
    pub fn calculate_ops_per_second(&mut self, total_duration: Duration) {
        if total_duration.as_secs_f64() > 0.0 {
            self.operations_per_second =
                self.total_operations as f64 / total_duration.as_secs_f64();
        }
    }

    /// Calculate average gas per operation
    pub fn calculate_gas_per_operation(&mut self) {
        if self.total_operations > 0 {
            self.gas_per_operation = self.total_gas_used as f64 / self.total_operations as f64;
        }
    }
}

/// Result of a benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub precompile: String,
    pub timestamp: String,
    pub scenario: String,
    pub metrics: BenchmarkMetrics,
    pub breakdown: HashMap<String, OperationBreakdown>,
}

/// Breakdown of operations by type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationBreakdown {
    pub operations: u64,
    pub gas_per_op: f64,
    pub total_gas: u64,
}

/// Base implementation for common benchmarking functionality
pub struct BaseBenchmarker<P: Provider> {
    pub provider: Arc<P>,
    pub signers: Vec<PrivateKeySigner>,
    pub chain_id: u64,
    pub ops_per_tx: u64,
    pub max_gas_per_tx: u64,
    pub contract_address: Option<Address>,
    pub metrics: BenchmarkMetrics,
}

impl<P: Provider> BaseBenchmarker<P> {
    pub fn new(
        provider: Arc<P>,
        signers: Vec<PrivateKeySigner>,
        chain_id: u64,
        ops_per_tx: u64,
        max_gas_per_tx: u64,
    ) -> Self {
        Self {
            provider,
            signers,
            chain_id,
            ops_per_tx,
            max_gas_per_tx,
            contract_address: None,
            metrics: BenchmarkMetrics::default(),
        }
    }

    /// Execute a batch of transactions and collect metrics
    pub async fn execute_transactions(
        &mut self,
        transactions: Vec<TransactionRequest>,
    ) -> Result<()> {
        let start_time = Instant::now();
        let mut total_operations = 0u64;
        let mut total_gas_used = 0u64;
        let mut execution_times = Vec::new();
        let mut gas_used_per_tx = Vec::new();

        for tx_req in transactions {
            let tx_start = Instant::now();

            // Send transaction (implementation depends on specific benchmark)
            // This would be implemented by the specific benchmarker
            // For now, we'll simulate the transaction

            let tx_duration = tx_start.elapsed();
            execution_times.push(tx_duration);

            // Track operations and gas
            total_operations += tx_req.operation_count;
            total_gas_used += tx_req.gas;
            gas_used_per_tx.push(tx_req.gas);

            self.metrics.transactions_sent += 1;
        }

        // Update metrics
        self.metrics.total_operations = total_operations;
        self.metrics.total_gas_used = total_gas_used;
        self.metrics.execution_times = execution_times;
        self.metrics.gas_used_per_tx = gas_used_per_tx;
        self.metrics.operations_per_transaction = self.ops_per_tx;

        // Calculate derived metrics
        self.metrics.calculate_percentiles();
        self.metrics.calculate_ops_per_second(start_time.elapsed());
        self.metrics.calculate_gas_per_operation();

        Ok(())
    }

    /// Run benchmark with specified scenario
    pub async fn run_scenario(
        &mut self,
        scenario: BenchmarkScenario,
        target_ops: u64,
        duration: u64,
    ) -> Result<()> {
        match scenario {
            BenchmarkScenario::Baseline => {
                // Run operations sequentially at a steady rate
                let txs_needed = (target_ops + self.ops_per_tx - 1) / self.ops_per_tx;
                println!("Running baseline scenario: {} transactions", txs_needed);
            }
            BenchmarkScenario::Load => {
                // Sustained high throughput for duration
                println!("Running load test for {} seconds", duration);
            }
            BenchmarkScenario::Burst => {
                // Sudden spike in operations
                println!("Running burst test with target {} operations", target_ops);
            }
            BenchmarkScenario::Mixed => {
                // Mix of different operation types
                println!("Running mixed workload scenario");
            }
        }

        Ok(())
    }
}

/// Helper function to calculate statistics from a list of values
pub fn calculate_stats(values: &[f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }

    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;

    let variance: f64 =
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;

    let std_dev = variance.sqrt();

    (
        mean,
        std_dev,
        values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
    )
}
