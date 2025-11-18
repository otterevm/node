//! Precompile benchmarking
//!
//! Benchmarking for precompiles, we have:
//! * TIP20
//! * TIP20 Factory
//! * TIP403 Registry
//! * TIP Fee manager
//! * TIP Account Registrar
//! * TIP Stablecoin Exchange
//! * TIP Nonce Precompile
//! * TIP Validator Config

mod account_registrar;
mod contracts;
mod fee_manager;
mod framework;
mod nonce;
mod stablecoin_exchange;
mod tip20;
mod tip20_factory;
mod tip403_registry;
mod validator_config;

use alloy::{
    providers::{Provider, ProviderBuilder},
    transports::http::reqwest::Url,
};
use alloy_signer_local::{MnemonicBuilder, PrivateKeySigner, coins_bip39::English};
use clap::{Parser, ValueEnum};
use eyre::{Context, Result};
use std::{fs::File, io::BufWriter, sync::Arc};

use self::framework::{BenchmarkResult, PrecompileBenchmarker};

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum PrecompileType {
    Tip20,
    Tip20Factory,
    Tip403,
    FeeManager,
    AccountRegistrar,
    StablecoinExchange,
    Nonce,
    ValidatorConfig,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum BenchmarkScenario {
    /// Test individual operations in isolation
    Baseline,
    /// Sustained high throughput
    Load,
    /// Sudden spike in operations
    Burst,
    /// Combination of different operations
    Mixed,
}

/// Run precompile benchmarking
#[derive(Parser, Debug)]
pub struct PrecompilesArgs {
    /// Precompile to benchmark
    #[arg(short, long)]
    precompile: PrecompileType,

    /// Operations per transaction
    #[arg(long, default_value = "100")]
    ops_per_tx: u64,

    /// Total operations to execute
    #[arg(long, default_value = "10000")]
    target_ops: u64,

    /// Maximum gas per transaction
    #[arg(long, default_value = "30000000")]
    max_gas_per_tx: u64,

    /// Benchmark scenario
    #[arg(short, long, default_value = "baseline")]
    scenario: BenchmarkScenario,

    /// Output file for results (JSON)
    #[arg(short, long)]
    output: Option<String>,

    /// Number of worker threads
    #[arg(short, long, default_value = "10")]
    workers: usize,

    /// RPC endpoint URL
    #[arg(long, default_value = "http://localhost:8545")]
    rpc_url: String,

    /// Mnemonic for generating accounts
    #[arg(
        short,
        long,
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,

    /// Chain ID
    #[arg(long, default_value = "1337")]
    chain_id: u64,

    /// Test duration in seconds (for load/burst scenarios)
    #[arg(short, long, default_value = "30")]
    duration: u64,
}

impl PrecompilesArgs {
    pub async fn run(&self) -> Result<()> {
        println!("Starting precompile benchmarks...");
        println!("Configuration:");
        println!("  Precompile: {:?}", self.precompile);
        println!("  Operations per TX: {}", self.ops_per_tx);
        println!("  Target operations: {}", self.target_ops);
        println!("  Max gas per TX: {}", self.max_gas_per_tx);
        println!("  Scenario: {:?}", self.scenario);
        println!("  Workers: {}", self.workers);
        println!();

        // Setup provider and signers
        let url = Url::parse(&self.rpc_url).context("Failed to parse RPC URL")?;
        let provider = Arc::new(ProviderBuilder::new().connect_http(url));

        // Generate signers from mnemonic
        let signers = self.generate_signers()?;

        // Run benchmarks based on selected precompile
        let result = match self.precompile {
            PrecompileType::Tip20 => {
                self.run_tip20_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::Tip20Factory => {
                self.run_tip20_factory_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::Tip403 => {
                self.run_tip403_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::FeeManager => {
                self.run_fee_manager_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::AccountRegistrar => {
                self.run_account_registrar_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::StablecoinExchange => {
                self.run_stablecoin_exchange_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::Nonce => {
                self.run_nonce_benchmark(provider.clone(), signers.clone())
                    .await?
            }
            PrecompileType::ValidatorConfig => {
                self.run_validator_config_benchmark(provider.clone(), signers.clone())
                    .await?
            }
        };

        // Output results
        self.output_results(&[result])?;

        Ok(())
    }

    fn generate_signers(&self) -> Result<Vec<PrivateKeySigner>> {
        let mut signers = Vec::new();

        for i in 0..self.workers {
            let signer = MnemonicBuilder::<English>::default()
                .phrase(&self.mnemonic)
                .index(i as u32)?
                .build()?;
            signers.push(signer);
        }

        Ok(signers)
    }

    async fn run_tip20_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = tip20::Tip20Benchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_tip20_factory_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = tip20_factory::Tip20FactoryBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_tip403_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = tip403_registry::Tip403Benchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_fee_manager_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = fee_manager::FeeManagerBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_account_registrar_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = account_registrar::AccountRegistrarBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_stablecoin_exchange_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = stablecoin_exchange::StablecoinExchangeBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_nonce_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = nonce::NonceBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    async fn run_validator_config_benchmark(
        &self,
        provider: Arc<impl Provider>,
        signers: Vec<PrivateKeySigner>,
    ) -> Result<BenchmarkResult> {
        let mut benchmarker = validator_config::ValidatorConfigBenchmarker::new(
            provider,
            signers,
            self.chain_id,
            self.ops_per_tx,
            self.max_gas_per_tx,
        );

        benchmarker.setup_state().await?;
        benchmarker.deploy_contracts().await?;
        benchmarker
            .run_benchmark(self.scenario, self.target_ops, self.duration)
            .await
    }

    fn output_results(&self, results: &[BenchmarkResult]) -> Result<()> {
        // Print summary to console
        println!("\n=== Benchmark Results ===");
        for result in results {
            println!("\nPrecompile: {}", result.precompile);
            println!("Total operations: {}", result.metrics.total_operations);
            println!(
                "Operations per transaction: {}",
                result.metrics.operations_per_transaction
            );
            println!(
                "Operations per block: {}",
                result.metrics.operations_per_block
            );
            println!(
                "Operations per second: {:.2}",
                result.metrics.operations_per_second
            );
            println!("Gas per operation: {:.2}", result.metrics.gas_per_operation);
            println!("Total gas used: {}", result.metrics.total_gas_used);
            println!("Blocks processed: {}", result.metrics.blocks_processed);
            println!("Transactions sent: {}", result.metrics.transactions_sent);
            println!("P50 latency: {:?}", result.metrics.p50_latency);
            println!("P99 latency: {:?}", result.metrics.p99_latency);
        }

        // Write to file if specified
        if let Some(output_path) = &self.output {
            let file = File::create(output_path)?;
            let writer = BufWriter::new(file);
            serde_json::to_writer_pretty(writer, results)?;
            println!("\nResults written to {}", output_path);
        }

        Ok(())
    }
}
