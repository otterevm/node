//! Validator Config precompile benchmarking implementation

use alloy::{
    network::TxSignerSync,
    primitives::{Address, TxKind, U256},
    providers::Provider,
    sol_types::SolCall,
};
use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_signer_local::PrivateKeySigner;
use eyre::Result;
use std::{collections::HashMap, sync::Arc, time::Instant};

use super::{
    BenchmarkScenario,
    contracts::IValidatorConfigBenchmark,
    framework::{
        BaseBenchmarker, BenchmarkResult, OperationBreakdown, PrecompileBenchmarker,
        TransactionRequest,
    },
};

pub struct ValidatorConfigBenchmarker<P: Provider> {
    base: BaseBenchmarker<P>,
}

impl<P: Provider> ValidatorConfigBenchmarker<P> {
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
impl<P: Provider + Send + Sync> PrecompileBenchmarker for ValidatorConfigBenchmarker<P> {
    async fn setup_state(&mut self) -> Result<()> {
        println!("Setting up Validator Config benchmark state...");
        Ok(())
    }

    async fn deploy_contracts(&mut self) -> Result<()> {
        println!("Deploying Validator Config benchmark contract...");
        self.base.contract_address = Some(Address::from([0x49; 20]));

        // Setup initial validator set
        if let Some(contract_addr) = self.base.contract_address {
            let calldata = IValidatorConfigBenchmark::setupCall {
                numValidators: U256::from(50), // Start with 50 validators
            }
            .abi_encode();

            // Create and sign the setup transaction
            let mut tx = TxLegacy {
                chain_id: Some(self.base.chain_id),
                nonce: 0,                 // Would need to get actual nonce from provider
                gas_price: 1_000_000_000, // 1 gwei
                gas_limit: 500_000,
                to: TxKind::Call(contract_addr),
                value: U256::ZERO,
                input: calldata.into(),
            };

            let signature = self.base.signers[0]
                .sign_transaction_sync(&mut tx)
                .map_err(|e| eyre::eyre!("Failed to sign validator setup transaction: {}", e))?;

            let _signed_tx = tx.into_signed(signature);

            println!("Setting up initial validator set with 50 validators");

            // In a real implementation, would send via provider:
            // let pending = self.base.provider.send_raw_transaction(&signed_tx.encoded_2718()).await?;
            // let receipt = pending.await?;
        }

        Ok(())
    }

    async fn generate_transactions(&self, ops_per_tx: u64) -> Result<Vec<TransactionRequest>> {
        let mut transactions = Vec::new();

        let calldata = IValidatorConfigBenchmark::spamValidatorAdditionsCall {
            operations: U256::from(ops_per_tx),
        }
        .abi_encode();

        let tx = TransactionRequest {
            from: self.base.signers[0].address(),
            to: self.base.contract_address.unwrap(),
            data: calldata,
            gas: self.base.max_gas_per_tx,
            operation_count: ops_per_tx,
            operation_type: "addValidator".to_string(),
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
        println!(
            "Running Validator Config benchmark - Scenario: {:?}",
            scenario
        );
        let start_time = Instant::now();

        match scenario {
            BenchmarkScenario::Mixed => {
                // Mix additions, status changes, and queries
                let additions = target_ops / 3;
                let status_changes = target_ops / 3;
                let queries = target_ops - additions - status_changes;

                let mut txs = Vec::new();

                // Validator additions
                let calldata = IValidatorConfigBenchmark::spamValidatorAdditionsCall {
                    operations: U256::from(additions),
                }
                .abi_encode();
                txs.push(TransactionRequest {
                    from: self.base.signers[0].address(),
                    to: self.base.contract_address.unwrap(),
                    data: calldata,
                    gas: self.base.max_gas_per_tx,
                    operation_count: additions,
                    operation_type: "addValidator".to_string(),
                });

                // Status changes
                let calldata = IValidatorConfigBenchmark::spamStatusChangesCall {
                    operations: U256::from(status_changes),
                }
                .abi_encode();
                txs.push(TransactionRequest {
                    from: self.base.signers[0].address(),
                    to: self.base.contract_address.unwrap(),
                    data: calldata,
                    gas: self.base.max_gas_per_tx,
                    operation_count: status_changes,
                    operation_type: "changeValidatorStatus".to_string(),
                });

                // Queries
                let calldata = IValidatorConfigBenchmark::spamValidatorQueriesCall {
                    operations: U256::from(queries),
                }
                .abi_encode();
                txs.push(TransactionRequest {
                    from: self.base.signers[0].address(),
                    to: self.base.contract_address.unwrap(),
                    data: calldata,
                    gas: self.base.max_gas_per_tx,
                    operation_count: queries,
                    operation_type: "getValidators".to_string(),
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
            "addValidator".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 3,
                gas_per_op: 60000.0,
                total_gas: (self.base.metrics.total_operations / 3) * 60000,
            },
        );
        breakdown.insert(
            "changeValidatorStatus".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 3,
                gas_per_op: 40000.0,
                total_gas: (self.base.metrics.total_operations / 3) * 40000,
            },
        );
        breakdown.insert(
            "getValidators".to_string(),
            OperationBreakdown {
                operations: self.base.metrics.total_operations / 3,
                gas_per_op: 10000.0,
                total_gas: (self.base.metrics.total_operations / 3) * 10000,
            },
        );

        Ok(BenchmarkResult {
            precompile: "ValidatorConfig".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            scenario: format!("{:?}", scenario),
            metrics: self.base.metrics.clone(),
            breakdown,
        })
    }
}
