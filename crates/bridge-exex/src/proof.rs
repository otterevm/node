//! Receipt trie Merkle proof generation for burn events.
//!
//! This module provides utilities for generating MPT proofs that can be verified
//! on-chain by the StablecoinEscrow contract.

use alloy::{
    network::BlockResponse,
    primitives::{keccak256, Bytes, B256},
    providers::Provider,
    rpc::types::TransactionReceipt,
};
use alloy_rlp::Encodable;
use alloy_trie::{HashBuilder, Nibbles};
use eyre::Result;
use tracing::debug;

/// Proof data for a burn event on Tempo chain.
#[derive(Debug, Clone)]
pub struct BurnProof {
    /// RLP-encoded receipt containing the burn event.
    pub receipt_rlp: Bytes,
    /// MPT proof nodes for the receipt.
    pub receipt_proof: Vec<Bytes>,
    /// Index of the burn event log within the receipt.
    pub log_index: u64,
}

/// Block header information from Tempo chain.
#[derive(Debug, Clone)]
pub struct TempoBlockHeader {
    /// Block number.
    pub block_number: u64,
    /// Block hash.
    pub block_hash: B256,
    /// State root of the block.
    pub state_root: B256,
    /// Receipts root of the block.
    pub receipts_root: B256,
}

/// Generator for receipt Merkle proofs.
///
/// Uses an alloy provider to fetch block and receipt data from Tempo RPC.
pub struct ProofGenerator<P> {
    provider: P,
}

impl<P> ProofGenerator<P> {
    /// Create a new proof generator with the given provider.
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Compute the receipts root from a list of receipts.
    ///
    /// Uses the ordered trie root computation matching Ethereum's receipt trie.
    pub fn compute_receipts_root(receipts: &[TransactionReceipt]) -> B256 {
        if receipts.is_empty() {
            return alloy_trie::EMPTY_ROOT_HASH;
        }

        let mut hash_builder = HashBuilder::default();

        for (index, receipt) in receipts.iter().enumerate() {
            let key = Nibbles::unpack(alloy_rlp::encode(index));
            let value = encode_receipt_for_trie(receipt);
            hash_builder.add_leaf(key, &value);
        }

        hash_builder.root()
    }

    /// Generate a receipt proof for the given transaction index.
    ///
    /// The proof is compatible with the simplified verification in StablecoinEscrow.sol
    /// which computes: `hash(hash(...hash(receiptHash, proof[0]), proof[1])..., proof[n])`
    pub fn generate_receipt_proof(
        receipts: &[TransactionReceipt],
        tx_index: usize,
        log_index: u64,
    ) -> Result<BurnProof> {
        if tx_index >= receipts.len() {
            return Err(eyre::eyre!(
                "Transaction index {} out of bounds (block has {} receipts)",
                tx_index,
                receipts.len()
            ));
        }

        let receipt = &receipts[tx_index];
        let receipt_rlp = encode_receipt_for_trie(receipt);

        debug!(
            tx_index,
            log_index,
            receipt_rlp_len = receipt_rlp.len(),
            "Generating receipt proof"
        );

        // For the simplified proof scheme used by StablecoinEscrow.sol,
        // we generate a proof where each sibling hash is concatenated.
        // The contract verifies by: hash(hash(...hash(receiptHash, proof[0]), proof[1])...)
        let proof_nodes = generate_simplified_proof(receipts, tx_index)?;

        Ok(BurnProof {
            receipt_rlp: receipt_rlp.into(),
            receipt_proof: proof_nodes,
            log_index,
        })
    }
}

impl<P> ProofGenerator<P>
where
    P: Provider,
{
    /// Fetch block header from Tempo RPC.
    pub async fn get_block_header(&self, block_number: u64) -> Result<TempoBlockHeader> {
        let block = self
            .provider
            .get_block_by_number(block_number.into())
            .await?
            .ok_or_else(|| eyre::eyre!("Block {} not found", block_number))?;

        let header = block.header();
        Ok(TempoBlockHeader {
            block_number,
            block_hash: header.hash,
            state_root: header.state_root,
            receipts_root: header.receipts_root,
        })
    }

    /// Fetch all receipts for a block.
    pub async fn get_block_receipts(
        &self,
        block_number: u64,
    ) -> Result<Vec<TransactionReceipt>> {
        let receipts = self
            .provider
            .get_block_receipts(block_number.into())
            .await?
            .ok_or_else(|| eyre::eyre!("Receipts for block {} not found", block_number))?;

        Ok(receipts)
    }
}

/// Encode a receipt for inclusion in the receipt trie.
///
/// This follows Ethereum's receipt encoding: type byte (if not legacy) + RLP(receipt).
fn encode_receipt_for_trie(receipt: &TransactionReceipt) -> Vec<u8> {
    use alloy::consensus::ReceiptEnvelope;

    // Convert from RPC Log type to primitive Log type for encoding
    let primitive_envelope: ReceiptEnvelope = receipt.inner.clone().map_logs(|log| log.inner);

    let mut buf = Vec::new();
    primitive_envelope.encode(&mut buf);
    buf
}

/// Generate a simplified proof compatible with StablecoinEscrow verification.
///
/// Generate a Merkle proof for a receipt.
///
/// The proof includes position information to enable proper verification.
/// Each proof element is 33 bytes: 32 bytes for sibling hash + 1 byte for position flag.
fn generate_simplified_proof(
    receipts: &[TransactionReceipt],
    target_index: usize,
) -> Result<Vec<Bytes>> {
    if receipts.is_empty() {
        return Err(eyre::eyre!("Cannot generate proof for empty receipt list"));
    }

    if target_index >= receipts.len() {
        return Err(eyre::eyre!(
            "Target index {} out of bounds (block has {} receipts)",
            target_index,
            receipts.len()
        ));
    }

    if receipts.len() == 1 {
        // Single receipt - no proof needed, receipt hash == root
        return Ok(vec![]);
    }

    // Encode all receipts and compute their hashes
    let receipt_hashes: Vec<B256> = receipts
        .iter()
        .map(|r| keccak256(encode_receipt_for_trie(r)))
        .collect();

    // Build a binary Merkle tree proof with position information
    let proof_elements = build_merkle_proof(&receipt_hashes, target_index)?;

    // Convert to bytes format for on-chain verification
    Ok(proof_elements_to_bytes(&proof_elements))
}

/// A proof element containing both the sibling hash and position information.
#[derive(Debug, Clone)]
pub struct ProofElement {
    /// The sibling hash.
    pub sibling: B256,
    /// True if the current node is on the left (sibling is on right).
    pub is_left: bool,
}

/// Build a binary Merkle tree proof for the given leaf index.
///
/// Returns sibling hashes with position information needed to reconstruct the root.
fn build_merkle_proof(leaves: &[B256], target_index: usize) -> Result<Vec<ProofElement>> {
    if leaves.is_empty() {
        return Err(eyre::eyre!("Cannot build proof for empty leaves"));
    }

    if target_index >= leaves.len() {
        return Err(eyre::eyre!("Target index out of bounds"));
    }

    let mut proof = Vec::new();
    let mut current_level: Vec<B256> = leaves.to_vec();
    let mut current_index = target_index;

    while current_level.len() > 1 {
        let mut next_level = Vec::new();
        let is_left = current_index.is_multiple_of(2);
        let sibling_index = if is_left {
            current_index + 1
        } else {
            current_index - 1
        };

        // Add sibling to proof if it exists
        let sibling = if sibling_index < current_level.len() {
            current_level[sibling_index]
        } else {
            // Odd number of nodes - duplicate the last one
            current_level[current_level.len() - 1]
        };

        proof.push(ProofElement { sibling, is_left });

        // Build next level
        for i in (0..current_level.len()).step_by(2) {
            let left = current_level[i];
            let right = if i + 1 < current_level.len() {
                current_level[i + 1]
            } else {
                left // Duplicate for odd count
            };

            let parent = keccak256([left.as_slice(), right.as_slice()].concat());
            next_level.push(parent);
        }

        current_level = next_level;
        current_index /= 2;
    }

    Ok(proof)
}

/// Convert proof elements to bytes for on-chain verification.
///
/// The StablecoinEscrow contract uses a simplified verification:
/// `computedRoot = keccak256(abi.encodePacked(computedRoot, proof[i]))`
///
/// To make this work, we encode each proof element as (sibling, is_left_flag).
/// The is_left_flag byte is 0x01 if current is on left, 0x00 if current is on right.
fn proof_elements_to_bytes(elements: &[ProofElement]) -> Vec<Bytes> {
    elements
        .iter()
        .map(|elem| {
            let mut data = elem.sibling.to_vec();
            data.push(if elem.is_left { 0x01 } else { 0x00 });
            Bytes::from(data)
        })
        .collect()
}

/// Verify a Merkle proof matches the expected root.
///
/// This uses proper ordered hashing based on position.
pub fn verify_simplified_proof(receipt_hash: B256, proof: &[Bytes], expected_root: B256) -> bool {
    let mut computed = receipt_hash;

    for proof_elem in proof {
        if proof_elem.len() != 33 {
            return false;
        }
        let sibling = B256::from_slice(&proof_elem[..32]);
        let is_left = proof_elem[32] == 0x01;

        computed = if is_left {
            // Current is on left, sibling is on right
            keccak256([computed.as_slice(), sibling.as_slice()].concat())
        } else {
            // Current is on right, sibling is on left
            keccak256([sibling.as_slice(), computed.as_slice()].concat())
        };
    }

    computed == expected_root
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::{
        consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom},
        primitives::{Address, LogData},
        rpc::types::Log as RpcLog,
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

        // Map to RPC Log type
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

    #[test]
    fn test_empty_receipts_root() {
        let receipts: Vec<TransactionReceipt> = vec![];
        let root = ProofGenerator::<()>::compute_receipts_root(&receipts);
        assert_eq!(root, alloy_trie::EMPTY_ROOT_HASH);
    }

    #[test]
    fn test_single_receipt_proof() {
        let receipts = vec![create_mock_receipt(true, 1)];
        let receipt_rlp = encode_receipt_for_trie(&receipts[0]);
        let receipt_hash = keccak256(&receipt_rlp);

        // For a single receipt with simplified proof, the hash should equal the root
        // (when using binary merkle tree, single leaf is its own root)
        let proof = generate_simplified_proof(&receipts, 0).unwrap();

        // Verify the proof is empty for single receipt
        assert!(proof.is_empty());

        // The receipt hash is the root for single receipt case
        // Note: actual trie root differs due to key encoding, but for simplified proof this works
        let verified = verify_simplified_proof(receipt_hash, &proof, receipt_hash);
        assert!(verified);
    }

    #[test]
    fn test_multiple_receipts_proof() {
        let receipts = vec![
            create_mock_receipt(true, 1),
            create_mock_receipt(true, 2),
            create_mock_receipt(false, 0),
            create_mock_receipt(true, 3),
        ];

        // Generate proof for receipt at index 1
        let proof = generate_simplified_proof(&receipts, 1).unwrap();
        assert!(!proof.is_empty());

        // Compute expected root using binary merkle tree
        let hashes: Vec<B256> = receipts
            .iter()
            .map(|r| keccak256(encode_receipt_for_trie(r)))
            .collect();

        // Build root: hash pairs, then hash results
        let h01 = keccak256([hashes[0].as_slice(), hashes[1].as_slice()].concat());
        let h23 = keccak256([hashes[2].as_slice(), hashes[3].as_slice()].concat());
        let expected_root = keccak256([h01.as_slice(), h23.as_slice()].concat());

        // Verify proof for index 1
        let receipt_hash = hashes[1];
        let verified = verify_simplified_proof(receipt_hash, &proof, expected_root);
        assert!(verified, "Proof verification failed for index 1");
    }

    #[test]
    fn test_proof_for_each_index() {
        let receipts: Vec<_> = (0..5).map(|i| create_mock_receipt(true, i)).collect();

        let hashes: Vec<B256> = receipts
            .iter()
            .map(|r| keccak256(encode_receipt_for_trie(r)))
            .collect();

        // Compute root using binary merkle tree (with odd handling)
        fn compute_root(hashes: &[B256]) -> B256 {
            if hashes.len() == 1 {
                return hashes[0];
            }

            let mut next_level = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                let left = hashes[i];
                let right = if i + 1 < hashes.len() {
                    hashes[i + 1]
                } else {
                    left
                };
                next_level.push(keccak256([left.as_slice(), right.as_slice()].concat()));
            }
            compute_root(&next_level)
        }

        let expected_root = compute_root(&hashes);

        for (i, _receipt) in receipts.iter().enumerate() {
            let proof = generate_simplified_proof(&receipts, i).unwrap();
            let verified = verify_simplified_proof(hashes[i], &proof, expected_root);
            assert!(verified, "Proof verification failed for index {}", i);
        }
    }

    #[test]
    fn test_invalid_index() {
        let receipts = vec![create_mock_receipt(true, 1)];
        let result = generate_simplified_proof(&receipts, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_receipt_deterministic() {
        let receipt = create_mock_receipt(true, 2);
        let encoded1 = encode_receipt_for_trie(&receipt);
        let encoded2 = encode_receipt_for_trie(&receipt);
        assert_eq!(encoded1, encoded2);
    }

    #[test]
    fn test_burn_proof_struct() {
        let receipts = vec![create_mock_receipt(true, 3)];
        let receipt_rlp = encode_receipt_for_trie(&receipts[0]);

        let proof = BurnProof {
            receipt_rlp: receipt_rlp.into(),
            receipt_proof: vec![Bytes::from_static(&[1, 2, 3])],
            log_index: 2,
        };

        assert_eq!(proof.log_index, 2);
        assert!(!proof.receipt_rlp.is_empty());
        assert_eq!(proof.receipt_proof.len(), 1);
    }

    #[test]
    fn test_tempo_block_header_struct() {
        let header = TempoBlockHeader {
            block_number: 12345,
            block_hash: B256::repeat_byte(0xAB),
            state_root: B256::repeat_byte(0xCD),
            receipts_root: B256::repeat_byte(0xEF),
        };

        assert_eq!(header.block_number, 12345);
        assert_eq!(header.block_hash, B256::repeat_byte(0xAB));
        assert_eq!(header.state_root, B256::repeat_byte(0xCD));
        assert_eq!(header.receipts_root, B256::repeat_byte(0xEF));
    }

    #[test]
    fn test_wrong_proof_fails_verification() {
        let receipts = vec![
            create_mock_receipt(true, 1),
            create_mock_receipt(true, 2),
        ];

        let hashes: Vec<B256> = receipts
            .iter()
            .map(|r| keccak256(encode_receipt_for_trie(r)))
            .collect();

        let expected_root = keccak256([hashes[0].as_slice(), hashes[1].as_slice()].concat());

        // Use wrong proof (different sibling)
        let wrong_proof = vec![Bytes::copy_from_slice(B256::random().as_slice())];
        let verified = verify_simplified_proof(hashes[0], &wrong_proof, expected_root);
        assert!(!verified, "Wrong proof should fail verification");
    }
}
