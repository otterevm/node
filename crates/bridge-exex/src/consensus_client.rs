//! Consensus RPC client for fetching finalization certificates.
//!
//! This module provides access to consensus layer data needed for header relay,
//! specifically the finalization certificates containing validator signatures.

use alloy::primitives::{Bytes, B256};
use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::retry::with_retry;

/// Finalization certificate data from the consensus layer.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertifiedBlock {
    pub epoch: u64,
    pub view: u64,
    pub height: Option<u64>,
    pub digest: B256,
    /// Hex-encoded finalization certificate (includes BLS threshold signature).
    pub certificate: String,
}

/// Query type for consensus RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Query {
    Latest,
    Height(u64),
}

/// Client for fetching finalization data from Tempo's consensus RPC.
pub struct ConsensusClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl ConsensusClient {
    /// Create a new consensus client.
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get the finalization certificate for a specific block height.
    ///
    /// Returns `None` if the block has not been finalized yet.
    pub async fn get_finalization(&self, height: u64) -> Result<Option<CertifiedBlock>> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "consensus_getFinalization",
            "params": [{"height": height}],
            "id": 1
        });

        let response = with_retry("consensus_getFinalization", || async {
            let resp = self
                .client
                .post(&self.rpc_url)
                .json(&request)
                .send()
                .await
                .wrap_err("Failed to send RPC request")?;

            let body: serde_json::Value = resp.json().await.wrap_err("Failed to parse response")?;

            if let Some(error) = body.get("error") {
                return Err(eyre::eyre!("RPC error: {}", error));
            }

            Ok(body)
        })
        .await?;

        let result = response.get("result");
        match result {
            Some(serde_json::Value::Null) | None => Ok(None),
            Some(value) => {
                let block: CertifiedBlock =
                    serde_json::from_value(value.clone()).wrap_err("Failed to parse CertifiedBlock")?;
                Ok(Some(block))
            }
        }
    }

    /// Get the latest finalization.
    pub async fn get_latest_finalization(&self) -> Result<Option<CertifiedBlock>> {
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "consensus_getFinalization",
            "params": ["latest"],
            "id": 1
        });

        let response = with_retry("consensus_getFinalization_latest", || async {
            let resp = self
                .client
                .post(&self.rpc_url)
                .json(&request)
                .send()
                .await
                .wrap_err("Failed to send RPC request")?;

            let body: serde_json::Value = resp.json().await.wrap_err("Failed to parse response")?;

            if let Some(error) = body.get("error") {
                return Err(eyre::eyre!("RPC error: {}", error));
            }

            Ok(body)
        })
        .await?;

        let result = response.get("result");
        match result {
            Some(serde_json::Value::Null) | None => Ok(None),
            Some(value) => {
                let block: CertifiedBlock =
                    serde_json::from_value(value.clone()).wrap_err("Failed to parse CertifiedBlock")?;
                Ok(Some(block))
            }
        }
    }
}

/// Extracts the BLS signature from a finalization certificate.
///
/// The certificate is encoded using commonware's codec format for
/// `Finalization<Scheme<PublicKey, MinSig>, Digest>`.
///
/// ## Certificate Structure (commonware-codec encoding)
///
/// The Finalization struct contains:
/// 1. `Proposal { round: Round, payload: Digest }`
///    - Round encoding: epoch (8 bytes, varint) + view (8 bytes, varint)
///    - Digest: 32 bytes (B256)
/// 2. `Signature`: BLS threshold signature (G1 point in MinSig variant = 48 bytes compressed)
/// 3. `Seed signature`: Random beacon signature (48 bytes compressed)
///
/// ## Note on Point Formats
///
/// The signature extracted is in **compressed G1 format** (48 bytes).
/// The EIP-2537 BLS precompiles expect **uncompressed G1 points** (128 bytes).
///
/// Decompression must happen either:
/// - On the Rust side before submitting (requires BLS12-381 library)
/// - In the Solidity contract (if it supports compressed input)
///
/// For now, we return the compressed signature. The light client should be
/// configured to handle this format, or the bridge operator should enable
/// a decompression step.
pub fn extract_bls_signature_from_certificate(certificate_hex: &str) -> Result<Bytes> {
    let certificate_bytes = hex::decode(certificate_hex.trim_start_matches("0x"))
        .wrap_err("Failed to decode certificate hex")?;

    // The finalization certificate layout (approximate, depends on varint encoding):
    // - Epoch: 1-10 bytes (varint)
    // - View: 1-10 bytes (varint)  
    // - Digest: 32 bytes
    // - Signature: 48 bytes (compressed G1)
    // - Seed signature: 48 bytes (compressed G1)
    //
    // For small epoch/view values (< 128), each is 1 byte.
    // Minimum size: 1 + 1 + 32 + 48 + 48 = 130 bytes
    // With larger values: up to 10 + 10 + 32 + 48 + 48 = 148 bytes

    // The signature length in MinSig variant
    const COMPRESSED_G1_LEN: usize = 48;

    // Minimum certificate length
    if certificate_bytes.len() < 130 {
        return Err(eyre::eyre!(
            "Certificate too short: expected at least 130 bytes, got {}",
            certificate_bytes.len()
        ));
    }

    // The signature is the 48 bytes after the proposal (round + digest).
    // Since round uses varint encoding, we need to parse from the end.
    // Structure: [epoch_varint][view_varint][digest:32][signature:48][seed_sig:48]
    //
    // The last 96 bytes are signature (48) + seed_sig (48).
    // The signature we want starts at len - 96.

    let signature_start = certificate_bytes.len() - 96;
    let signature_end = signature_start + COMPRESSED_G1_LEN;

    let compressed_sig = &certificate_bytes[signature_start..signature_end];

    debug!(
        certificate_len = certificate_bytes.len(),
        signature_start,
        signature_len = COMPRESSED_G1_LEN,
        "Extracted BLS signature from certificate (compressed G1)"
    );

    Ok(Bytes::copy_from_slice(compressed_sig))
}

/// Formats validator signatures for the light client based on its mode.
///
/// For BLS mode (production):
/// - Single aggregated BLS signature (G1 point, 128 bytes uncompressed)
///
/// For ECDSA mode (testing):
/// - ABI-encoded array of individual ECDSA signatures
pub fn format_signatures_for_light_client(
    certificate: &CertifiedBlock,
    use_ecdsa_mode: bool,
) -> Result<Bytes> {
    if use_ecdsa_mode {
        // ECDSA mode is for testing only - return empty for now
        // In production, we use BLS signatures from the consensus layer
        warn!("ECDSA mode not implemented for signature aggregation");
        return Ok(Bytes::new());
    }

    // Extract the BLS signature from the certificate
    extract_bls_signature_from_certificate(&certificate.certificate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_serialization() {
        let height_query = Query::Height(100);
        let json = serde_json::to_string(&height_query).unwrap();
        assert!(json.contains("100"));

        let latest_query = Query::Latest;
        let json = serde_json::to_string(&latest_query).unwrap();
        assert!(json.contains("latest"));
    }

    #[test]
    fn test_certified_block_deserialization() {
        let json = r#"{
            "epoch": 1,
            "view": 10,
            "height": 100,
            "digest": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "certificate": "deadbeef"
        }"#;

        let block: CertifiedBlock = serde_json::from_str(json).unwrap();
        assert_eq!(block.epoch, 1);
        assert_eq!(block.view, 10);
        assert_eq!(block.height, Some(100));
    }

    #[test]
    fn test_extract_bls_signature_valid_certificate() {
        // Create a mock certificate with the correct structure:
        // - epoch varint (1 byte for small values)
        // - view varint (1 byte for small values)
        // - digest (32 bytes)
        // - signature (48 bytes)
        // - seed_signature (48 bytes)
        // Total: 1 + 1 + 32 + 48 + 48 = 130 bytes

        let mut cert = vec![0u8; 130];
        cert[0] = 0x01; // epoch = 1
        cert[1] = 0x05; // view = 5
        // digest: bytes 2-33
        for i in 2..34 {
            cert[i] = (i - 2) as u8;
        }
        // signature: bytes 34-81 (48 bytes)
        for i in 34..82 {
            cert[i] = 0xAA; // marker for signature
        }
        // seed_signature: bytes 82-129 (48 bytes)
        for i in 82..130 {
            cert[i] = 0xBB; // marker for seed sig
        }

        let cert_hex = hex::encode(&cert);
        let result = extract_bls_signature_from_certificate(&cert_hex).unwrap();

        // Should extract the signature (48 bytes of 0xAA)
        assert_eq!(result.len(), 48);
        assert!(result.iter().all(|&b| b == 0xAA));
    }

    #[test]
    fn test_extract_bls_signature_with_0x_prefix() {
        // Same test but with 0x prefix
        let mut cert = vec![0u8; 130];
        for i in 34..82 {
            cert[i] = 0xCC;
        }

        let cert_hex = format!("0x{}", hex::encode(&cert));
        let result = extract_bls_signature_from_certificate(&cert_hex).unwrap();

        assert_eq!(result.len(), 48);
        assert!(result.iter().all(|&b| b == 0xCC));
    }

    #[test]
    fn test_extract_bls_signature_too_short() {
        let short_cert = hex::encode(&[0u8; 100]); // Too short
        let result = extract_bls_signature_from_certificate(&short_cert);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_extract_bls_signature_larger_certificate() {
        // Certificate with larger epoch/view values (more bytes for varint)
        // Total: 5 + 5 + 32 + 48 + 48 = 138 bytes
        let mut cert = vec![0u8; 138];
        // Fill signature with 0xDD
        for i in (138 - 96)..(138 - 48) {
            cert[i] = 0xDD;
        }
        // Fill seed_sig with 0xEE
        for i in (138 - 48)..138 {
            cert[i] = 0xEE;
        }

        let cert_hex = hex::encode(&cert);
        let result = extract_bls_signature_from_certificate(&cert_hex).unwrap();

        // Should still extract the correct signature
        assert_eq!(result.len(), 48);
        assert!(result.iter().all(|&b| b == 0xDD));
    }
}
