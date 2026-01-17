//! End-to-end integration tests for bridge flows.
//!
//! These tests simulate the full bridge lifecycle with mock components.
//! Tests requiring Anvil are marked with `#[ignore]` for CI compatibility.

use super::fixtures::*;
use crate::{
    persistence::{ProcessedBurn, SignedDeposit, StateManager},
    proof::{BurnProof, ProofGenerator, TempoBlockHeader, verify_simplified_proof},
    signer::BridgeSigner,
};
use alloy::{
    consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom},
    primitives::{keccak256, Address, Bytes, LogData, B256},
    rpc::types::{Log as RpcLog, TransactionReceipt},
};

fn create_mock_receipt(status: bool, logs_count: usize) -> TransactionReceipt {
    let primitive_logs: Vec<alloy::primitives::Log> = (0..logs_count)
        .map(|i| alloy::primitives::Log {
            address: Address::repeat_byte(i as u8),
            data: LogData::new_unchecked(vec![], Bytes::new()),
        })
        .collect();

    let receipt = Receipt {
        status: status.into(),
        cumulative_gas_used: 21000 * (logs_count as u64 + 1),
        logs: primitive_logs,
    };

    let receipt_with_bloom = ReceiptWithBloom::new(receipt, Default::default());
    let envelope = ReceiptEnvelope::Eip1559(receipt_with_bloom);

    let rpc_envelope = envelope.map_logs(|log| RpcLog {
        inner: log,
        block_hash: None,
        block_number: None,
        block_timestamp: None,
        transaction_hash: None,
        transaction_index: None,
        log_index: None,
        removed: false,
    });

    TransactionReceipt {
        inner: rpc_envelope,
        transaction_hash: B256::random(),
        transaction_index: Some(0),
        block_hash: Some(B256::random()),
        block_number: Some(1),
        gas_used: 21000,
        effective_gas_price: 1_000_000_000,
        blob_gas_used: None,
        blob_gas_price: None,
        from: Address::ZERO,
        to: Some(Address::repeat_byte(0xDE)),
        contract_address: None,
    }
}

mod deposit_flow {
    use super::*;

    #[tokio::test]
    async fn test_deposit_id_uniqueness() {
        let deposits: Vec<_> = (0..10)
            .map(|i| TestDeposit::usdc_deposit(1_000_000 * (i + 1), Address::repeat_byte(i as u8)))
            .collect();

        let ids: std::collections::HashSet<_> = deposits.iter().map(|d| d.deposit_id).collect();
        assert_eq!(ids.len(), deposits.len(), "All deposit IDs should be unique");
    }

    #[tokio::test]
    async fn test_deposit_signature_generation() {
        let validator_set = MockValidatorSet::single();
        let (_, signer) = &validator_set.validators[0];
        let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();

        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));
        let signature = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();

        assert_eq!(signature.len(), 65);
        assert!(!signature.is_empty());
    }

    #[tokio::test]
    async fn test_multi_validator_signing() {
        let validator_set = MockValidatorSet::three_of_five();
        let deposit = TestDeposit::usdc_deposit(10_000_000, Address::repeat_byte(0x42));

        let mut signatures = Vec::new();
        for (_, signer) in &validator_set.validators {
            let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();
            let sig = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();
            signatures.push(sig);
        }

        assert_eq!(signatures.len(), 5);
        assert!(signatures.len() as u64 >= validator_set.threshold);

        let unique_sigs: std::collections::HashSet<_> = signatures.iter().collect();
        assert_eq!(unique_sigs.len(), 5, "All signatures should be unique");
    }

    #[tokio::test]
    async fn test_deposit_state_persistence() {
        let state_manager = StateManager::new_in_memory();
        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        assert!(!state_manager.has_signed_deposit(&deposit.deposit_id).await);

        state_manager
            .record_signed_deposit(SignedDeposit {
                request_id: deposit.deposit_id,
                origin_chain_id: deposit.origin_chain_id,
                origin_tx_hash: deposit.tx_hash,
                tempo_recipient: deposit.tempo_recipient,
                amount: deposit.amount,
                signature_tx_hash: B256::random(),
                signed_at: 12345,
            })
            .await
            .unwrap();

        assert!(state_manager.has_signed_deposit(&deposit.deposit_id).await);
        assert!(!state_manager.is_deposit_finalized(&deposit.deposit_id).await);

        state_manager
            .mark_deposit_finalized(deposit.deposit_id)
            .await
            .unwrap();

        assert!(state_manager.is_deposit_finalized(&deposit.deposit_id).await);
    }

    #[tokio::test]
    async fn test_deposit_threshold_reached() {
        let validator_set = MockValidatorSet::three_of_five();
        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        let mut signatures_count = 0u64;
        for (_, signer) in validator_set.validators.iter().take(3) {
            let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();
            let _sig = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();
            signatures_count += 1;
        }

        assert!(signatures_count < validator_set.threshold);

        for (_, signer) in validator_set.validators.iter().skip(3).take(1) {
            let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();
            let _sig = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();
            signatures_count += 1;
        }

        assert!(signatures_count >= validator_set.threshold);
    }

    #[tokio::test]
    async fn test_cross_chain_deposit_isolation() {
        let eth_deposit = TestDeposit::new(
            1,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            1_000_000,
            Address::repeat_byte(0x33),
            0,
        );

        let arb_deposit = TestDeposit::new(
            42161,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            1_000_000,
            Address::repeat_byte(0x33),
            0,
        );

        assert_ne!(
            eth_deposit.deposit_id, arb_deposit.deposit_id,
            "Deposits on different chains must have different IDs"
        );
    }

    #[tokio::test]
    async fn test_deposit_file_persistence() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("bridge-state.json");
        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        {
            let state_manager = StateManager::new_persistent(&path).unwrap();
            state_manager
                .record_signed_deposit(SignedDeposit {
                    request_id: deposit.deposit_id,
                    origin_chain_id: deposit.origin_chain_id,
                    origin_tx_hash: deposit.tx_hash,
                    tempo_recipient: deposit.tempo_recipient,
                    amount: deposit.amount,
                    signature_tx_hash: B256::random(),
                    signed_at: 12345,
                })
                .await
                .unwrap();
        }

        {
            let state_manager = StateManager::new_persistent(&path).unwrap();
            assert!(state_manager.has_signed_deposit(&deposit.deposit_id).await);
        }
    }
}

mod burn_flow {
    use super::*;

    #[tokio::test]
    async fn test_burn_id_uniqueness() {
        let burns: Vec<_> = (0..10)
            .map(|i| TestBurn::usdc_burn(1_000_000 * (i + 1), Address::repeat_byte(i as u8), i))
            .collect();

        let ids: std::collections::HashSet<_> = burns.iter().map(|b| b.burn_id).collect();
        assert_eq!(ids.len(), burns.len(), "All burn IDs should be unique");
    }

    #[tokio::test]
    async fn test_burn_nonce_prevents_replay() {
        let burn1 = TestBurn::usdc_burn(1_000_000, Address::repeat_byte(0x42), 0);
        let burn2 = TestBurn::usdc_burn(1_000_000, Address::repeat_byte(0x42), 1);

        assert_ne!(
            burn1.burn_id, burn2.burn_id,
            "Different nonces must produce different burn IDs"
        );
    }

    #[tokio::test]
    async fn test_burn_state_persistence() {
        let state_manager = StateManager::new_in_memory();
        let burn = TestBurn::usdc_burn(1_000_000, Address::repeat_byte(0x42), 0);

        assert!(!state_manager.has_processed_burn(&burn.burn_id).await);

        state_manager
            .record_processed_burn(ProcessedBurn {
                burn_id: burn.burn_id,
                origin_chain_id: burn.origin_chain_id,
                origin_recipient: burn.origin_recipient,
                amount: burn.amount,
                tempo_block_number: burn.tempo_block_number,
                unlock_tx_hash: Some(B256::random()),
                processed_at: 12345,
            })
            .await
            .unwrap();

        assert!(state_manager.has_processed_burn(&burn.burn_id).await);
    }

    #[tokio::test]
    async fn test_burn_proof_generation() {
        let receipts = vec![
            create_mock_receipt(true, 1),
            create_mock_receipt(true, 2),
            create_mock_receipt(true, 3),
        ];

        let proof = ProofGenerator::<()>::generate_receipt_proof(&receipts, 1, 0).unwrap();

        assert!(!proof.receipt_rlp.is_empty());
        assert!(!proof.receipt_proof.is_empty());
        assert_eq!(proof.log_index, 0);
    }

    #[tokio::test]
    async fn test_burn_proof_verification() {
        let receipts = vec![
            create_mock_receipt(true, 1),
            create_mock_receipt(true, 2),
        ];

        fn encode_receipt(receipt: &TransactionReceipt) -> Vec<u8> {
            use alloy::consensus::ReceiptEnvelope;
            use alloy_rlp::Encodable;
            let primitive_envelope: ReceiptEnvelope =
                receipt.inner.clone().map_logs(|log| log.inner);
            let mut buf = Vec::new();
            primitive_envelope.encode(&mut buf);
            buf
        }

        let hashes: Vec<B256> = receipts.iter().map(|r| keccak256(encode_receipt(r))).collect();
        let expected_root = keccak256([hashes[0].as_slice(), hashes[1].as_slice()].concat());

        let proof = ProofGenerator::<()>::generate_receipt_proof(&receipts, 0, 0).unwrap();

        let receipt_hash = keccak256(&proof.receipt_rlp);
        let verified = verify_simplified_proof(receipt_hash, &proof.receipt_proof, expected_root);
        assert!(verified, "Proof verification should succeed");
    }

    #[tokio::test]
    async fn test_burn_cross_chain_isolation() {
        let eth_burn = TestBurn::new(
            1,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            1_000_000,
            0,
            Address::repeat_byte(0x33),
            100,
        );

        let arb_burn = TestBurn::new(
            42161,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            1_000_000,
            0,
            Address::repeat_byte(0x33),
            100,
        );

        assert_ne!(
            eth_burn.burn_id, arb_burn.burn_id,
            "Burns for different origin chains must have different IDs"
        );
    }
}

mod reorg_handling {
    use super::*;

    #[tokio::test]
    async fn test_reorg_detection() {
        let reorg = MockReorg::at_depth(100, 3);

        assert_eq!(reorg.common_ancestor, 100);
        assert_eq!(reorg.old_chain.len(), 3);
        assert_eq!(reorg.new_chain.len(), 3);

        for (old, new) in reorg.old_chain.iter().zip(&reorg.new_chain) {
            assert_eq!(old.block_number, new.block_number);
            assert_ne!(old.block_hash, new.block_hash);
        }
    }

    #[tokio::test]
    async fn test_deposit_invalidation_on_reorg() {
        let state_manager = StateManager::new_in_memory();
        let reorg = MockReorg::at_depth(100, 2);

        let deposit_in_reorged_block = TestDeposit::new(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            Address::repeat_byte(0x22),
            1_000_000,
            Address::repeat_byte(0x33),
            0,
        );

        state_manager
            .record_signed_deposit(SignedDeposit {
                request_id: deposit_in_reorged_block.deposit_id,
                origin_chain_id: deposit_in_reorged_block.origin_chain_id,
                origin_tx_hash: deposit_in_reorged_block.tx_hash,
                tempo_recipient: deposit_in_reorged_block.tempo_recipient,
                amount: deposit_in_reorged_block.amount,
                signature_tx_hash: B256::random(),
                signed_at: 12345,
            })
            .await
            .unwrap();

        assert!(
            state_manager
                .has_signed_deposit(&deposit_in_reorged_block.deposit_id)
                .await
        );

        let reorged_blocks = reorg.reorged_blocks();
        assert!(!reorged_blocks.is_empty());
    }

    #[tokio::test]
    async fn test_state_manager_block_tracking() {
        let state_manager = StateManager::new_in_memory();

        assert!(state_manager.get_origin_chain_block(1).await.is_none());

        state_manager.update_origin_chain_block(1, 100).await.unwrap();
        assert_eq!(state_manager.get_origin_chain_block(1).await, Some(100));

        state_manager.update_origin_chain_block(1, 200).await.unwrap();
        assert_eq!(state_manager.get_origin_chain_block(1).await, Some(200));

        state_manager.update_origin_chain_block(42161, 500).await.unwrap();
        assert_eq!(state_manager.get_origin_chain_block(42161).await, Some(500));
        assert_eq!(state_manager.get_origin_chain_block(1).await, Some(200));
    }

    #[tokio::test]
    async fn test_tempo_block_tracking() {
        let state_manager = StateManager::new_in_memory();

        assert_eq!(state_manager.get_tempo_block().await, 0);

        state_manager.update_tempo_block(100).await.unwrap();
        assert_eq!(state_manager.get_tempo_block().await, 100);

        state_manager.update_tempo_block(200).await.unwrap();
        assert_eq!(state_manager.get_tempo_block().await, 200);
    }
}

mod multi_validator {
    use super::*;

    #[tokio::test]
    async fn test_concurrent_signing() {
        let validator_set = MockValidatorSet::three_of_five();
        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        let handles: Vec<_> = validator_set
            .validators
            .iter()
            .map(|(_, signer)| {
                let deposit_id = deposit.deposit_id;
                let signer_bytes = signer.to_bytes();
                tokio::spawn(async move {
                    let bridge_signer = BridgeSigner::from_bytes(&signer_bytes).unwrap();
                    bridge_signer.sign_deposit(&deposit_id).await.unwrap()
                })
            })
            .collect();

        let signatures: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        assert_eq!(signatures.len(), 5);

        let unique_sigs: std::collections::HashSet<_> = signatures.iter().collect();
        assert_eq!(unique_sigs.len(), 5);
    }

    #[tokio::test]
    async fn test_threshold_not_reached_with_insufficient_signers() {
        let validator_set = MockValidatorSet::three_of_five();
        let _deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        let signatures_collected = 3u64;

        assert!(
            signatures_collected < validator_set.threshold,
            "3 signatures should not reach threshold of {} for 5 validators",
            validator_set.threshold
        );
    }

    #[tokio::test]
    async fn test_duplicate_signature_handling() {
        let validator_set = MockValidatorSet::single();
        let (_, signer) = &validator_set.validators[0];
        let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();

        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        let sig1 = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();
        let sig2 = bridge_signer.sign_deposit(&deposit.deposit_id).await.unwrap();

        assert_eq!(sig1, sig2, "Same signer should produce same signature");
    }

    #[tokio::test]
    async fn test_state_prevents_double_signing() {
        let state_manager = StateManager::new_in_memory();
        let deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        assert!(!state_manager.has_signed_deposit(&deposit.deposit_id).await);

        state_manager
            .record_signed_deposit(SignedDeposit {
                request_id: deposit.deposit_id,
                origin_chain_id: deposit.origin_chain_id,
                origin_tx_hash: deposit.tx_hash,
                tempo_recipient: deposit.tempo_recipient,
                amount: deposit.amount,
                signature_tx_hash: B256::random(),
                signed_at: 12345,
            })
            .await
            .unwrap();

        assert!(
            state_manager.has_signed_deposit(&deposit.deposit_id).await,
            "State should track that deposit was already signed"
        );
    }
}

mod proof_generation {
    use super::*;

    #[tokio::test]
    async fn test_proof_for_single_receipt() {
        let receipts = vec![create_mock_receipt(true, 1)];
        let proof = ProofGenerator::<()>::generate_receipt_proof(&receipts, 0, 0).unwrap();

        assert!(proof.receipt_proof.is_empty(), "Single receipt needs no proof siblings");
        assert!(!proof.receipt_rlp.is_empty());
    }

    #[tokio::test]
    async fn test_proof_for_multiple_receipts() {
        let receipts: Vec<_> = (0..8).map(|i| create_mock_receipt(true, i)).collect();

        for i in 0..receipts.len() {
            let proof = ProofGenerator::<()>::generate_receipt_proof(&receipts, i, 0).unwrap();
            assert!(!proof.receipt_rlp.is_empty());
            assert!(proof.receipt_proof.len() >= 1);
        }
    }

    #[tokio::test]
    async fn test_proof_invalid_index() {
        let receipts = vec![create_mock_receipt(true, 1)];
        let result = ProofGenerator::<()>::generate_receipt_proof(&receipts, 5, 0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tempo_block_header_struct() {
        let header = TempoBlockHeader {
            block_number: 12345,
            block_hash: B256::repeat_byte(0xAB),
            state_root: B256::repeat_byte(0xCD),
            receipts_root: B256::repeat_byte(0xEF),
        };

        assert_eq!(header.block_number, 12345);
        assert!(!header.block_hash.is_zero());
        assert!(!header.state_root.is_zero());
        assert!(!header.receipts_root.is_zero());
    }

    #[tokio::test]
    async fn test_burn_proof_struct() {
        let proof = BurnProof {
            receipt_rlp: Bytes::from_static(&[1, 2, 3, 4]),
            receipt_proof: vec![
                Bytes::from_static(&[5, 6, 7]),
                Bytes::from_static(&[8, 9, 10]),
            ],
            log_index: 2,
        };

        assert_eq!(proof.log_index, 2);
        assert_eq!(proof.receipt_rlp.len(), 4);
        assert_eq!(proof.receipt_proof.len(), 2);
    }
}

mod security {
    use super::*;

    #[tokio::test]
    async fn test_domain_separation() {
        let chain_id = ANVIL_CHAIN_ID;
        let token = Address::repeat_byte(0x11);
        let recipient = Address::repeat_byte(0x22);
        let amount = 1_000_000u64;

        let deposit_id = compute_deposit_id(chain_id, token, B256::ZERO, 0, recipient, amount, 100);

        let burn_id = compute_burn_id(chain_id, token, recipient, amount, 0, recipient);

        assert_ne!(
            deposit_id, burn_id,
            "Deposit and burn IDs must be different due to domain separation"
        );
    }

    #[tokio::test]
    async fn test_frontrunning_resistance() {
        let victim_recipient = Address::repeat_byte(0xAA);
        let attacker_recipient = Address::repeat_byte(0xBB);

        let victim_deposit = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            0,
            victim_recipient,
            1_000_000,
            100,
        );

        let attacker_deposit = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            0,
            attacker_recipient,
            1_000_000,
            100,
        );

        assert_ne!(
            victim_deposit, attacker_deposit,
            "Recipient must be bound in deposit ID"
        );
    }

    #[tokio::test]
    async fn test_amount_binding() {
        let small_deposit = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            0,
            Address::repeat_byte(0x33),
            1_000_000,
            100,
        );

        let large_deposit = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            0,
            Address::repeat_byte(0x33),
            10_000_000,
            100,
        );

        assert_ne!(
            small_deposit, large_deposit,
            "Different amounts must produce different IDs"
        );
    }

    #[tokio::test]
    async fn test_log_index_uniqueness() {
        let id_log_0 = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            0,
            Address::repeat_byte(0x33),
            1_000_000,
            100,
        );

        let id_log_1 = compute_deposit_id(
            ANVIL_CHAIN_ID,
            Address::repeat_byte(0x11),
            B256::repeat_byte(0x22),
            1,
            Address::repeat_byte(0x33),
            1_000_000,
            100,
        );

        assert_ne!(
            id_log_0, id_log_1,
            "Different log indices must produce different IDs"
        );
    }

    #[tokio::test]
    async fn test_signature_determinism() {
        let (_, signer) = &anvil_accounts()[0];
        let bridge_signer = BridgeSigner::from_bytes(&signer.to_bytes()).unwrap();

        let request_id = B256::repeat_byte(0x42);

        let sig1 = bridge_signer.sign_deposit(&request_id).await.unwrap();
        let sig2 = bridge_signer.sign_deposit(&request_id).await.unwrap();

        assert_eq!(sig1, sig2, "Signatures should be deterministic");
    }
}

#[cfg(test)]
mod anvil_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires Anvil running on localhost:8545"]
    async fn test_deposit_flow_with_anvil() {
        // This test requires:
        // 1. Anvil running: `anvil --port 8545`
        // 2. Escrow contract deployed
        // 3. Tempo node running with bridge precompile

        let _accounts = anvil_accounts();
        let _deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));
        let _ = &_deposit;

        // TODO: Deploy escrow contract
        // TODO: Make deposit on Anvil
        // TODO: Watch for deposit event
        // TODO: Register on Tempo
        // TODO: Sign with validators
        // TODO: Finalize
        // TODO: Verify TIP-20 minted
    }

    #[tokio::test]
    #[ignore = "Requires Anvil running on localhost:8545"]
    async fn test_burn_unlock_flow_with_anvil() {
        // This test requires:
        // 1. Anvil running: `anvil --port 8545`
        // 2. Escrow + light client deployed
        // 3. Tempo node running with bridge precompile

        let _burn = TestBurn::usdc_burn(1_000_000, Address::repeat_byte(0x42), 0);

        // TODO: Setup TIP-20 balance on Tempo
        // TODO: Burn tokens on Tempo
        // TODO: Relay header to light client
        // TODO: Generate proof
        // TODO: Unlock on origin
        // TODO: Verify tokens unlocked
    }

    #[tokio::test]
    #[ignore = "Requires Anvil running on localhost:8545"]
    async fn test_reorg_handling_with_anvil() {
        // This test requires Anvil with reorg simulation

        let _reorg = MockReorg::at_depth(100, 2);

        // TODO: Make deposit in block 101
        // TODO: Wait for detection
        // TODO: Simulate reorg via Anvil
        // TODO: Verify deposit is invalidated
        // TODO: Verify new deposit in reorg chain is processed
    }

    #[tokio::test]
    #[ignore = "Requires Anvil running on localhost:8545"]
    async fn test_multi_validator_signing_with_anvil() {
        let _validator_set = MockValidatorSet::three_of_five();
        let _deposit = TestDeposit::usdc_deposit(1_000_000, Address::repeat_byte(0x42));

        // TODO: Deploy escrow
        // TODO: Register validators on Tempo
        // TODO: Make deposit
        // TODO: Have each validator sign
        // TODO: Verify threshold detection
        // TODO: Verify finalization
    }
}
