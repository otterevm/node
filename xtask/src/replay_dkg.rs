//! Replay DKG outcome from block headers.
//!
//! This command fetches all block headers from an epoch and replays the DKG
//! ceremony to verify the on-chain outcome.

use std::collections::BTreeMap;

use alloy::{
    primitives::{B256, Bytes},
    providers::{Provider, ProviderBuilder},
};
use commonware_codec::{Encode as _, Read, ReadExt as _};
use commonware_consensus::types::{Epoch, Epocher as _, FixedEpocher, Height};
use commonware_cryptography::{
    bls12381::{
        dkg::{self, DealerLog, Info, SignedDealerLog},
        primitives::{sharing::Mode, variant::MinSig},
    },
    ed25519::{PrivateKey, PublicKey},
};
use commonware_parallel::Sequential;
use commonware_utils::{N3f1, NZU32, NZU64};
use eyre::{Context as _, OptionExt as _, bail, eyre};
use serde::Serialize;
use tempo_dkg_onchain_artifacts::OnchainDkgOutcome;

/// The namespace used by Tempo for DKG ceremonies.
const NAMESPACE: &[u8] = b"TEMPO";

#[derive(Debug, clap::Args)]
pub(crate) struct ReplayDkg {
    /// RPC endpoint URL (http://, https://, ws://, or wss://)
    #[arg(long)]
    rpc_url: String,

    /// Block number of the epoch E-1 boundary block (last block of epoch E-1).
    /// The DKG ceremony from epoch E will be replayed.
    #[arg(long, group = "boundary")]
    boundary_block: Option<u64>,

    /// Block hash of the epoch E-1 boundary block.
    #[arg(long, group = "boundary")]
    boundary_hash: Option<B256>,

    /// Epoch length in blocks (required to determine epoch boundaries).
    #[arg(long)]
    epoch_length: u64,
}

#[derive(Serialize)]
struct ReplayResult {
    /// The epoch that was replayed (epoch E).
    epoch: u64,
    /// Block range of epoch E (first block, last block).
    block_range: (u64, u64),
    /// Number of blocks fetched.
    blocks_fetched: usize,
    /// Number of dealer logs found in block headers.
    dealer_logs_found: usize,
    /// Dealers that contributed logs.
    dealers: Vec<String>,
    /// Whether the replayed outcome matches the on-chain outcome.
    matches_onchain: bool,
    /// The replayed network identity (group public key).
    replayed_identity: Bytes,
    /// The on-chain network identity for comparison.
    onchain_identity: Bytes,
    /// Threshold required for signing.
    threshold: u32,
    /// Total number of participants.
    total_participants: u32,
}

fn pubkey_to_hex(pk: &PublicKey) -> String {
    const_hex::encode_prefixed(pk.as_ref())
}

impl ReplayDkg {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let provider = ProviderBuilder::new()
            .connect(&self.rpc_url)
            .await
            .wrap_err("failed to connect to RPC")?;

        let epocher = FixedEpocher::new(NZU64!(self.epoch_length));

        // Step 1: Get the boundary block of epoch E-1
        let boundary_block = if let Some(hash) = self.boundary_hash {
            provider
                .get_block_by_hash(hash)
                .await
                .wrap_err_with(|| format!("failed to fetch block hash `{hash}`"))?
                .ok_or_else(|| eyre!("block {hash} not found"))?
        } else if let Some(block_num) = self.boundary_block {
            provider
                .get_block_by_number(block_num.into())
                .await
                .wrap_err_with(|| format!("failed to fetch block number `{block_num}`"))?
                .ok_or_else(|| eyre!("block {block_num} not found"))?
        } else {
            bail!("must provide either --boundary-block or --boundary-hash");
        };

        let boundary_block_num = boundary_block.header.number;

        // Verify this is actually an epoch boundary block
        let epoch_e_minus_1_info = epocher
            .containing(Height::new(boundary_block_num))
            .ok_or_eyre("block number too large for epoch strategy")?;

        if boundary_block_num != epoch_e_minus_1_info.last().get() {
            bail!(
                "block {} is not an epoch boundary block; epoch {} ends at block {}",
                boundary_block_num,
                epoch_e_minus_1_info.epoch().get(),
                epoch_e_minus_1_info.last().get()
            );
        }

        // Extract the DKG outcome from epoch E-1 boundary block (this gives us the
        // state going into epoch E)
        let epoch_e_minus_1_outcome =
            OnchainDkgOutcome::read(&mut boundary_block.header.inner.extra_data.as_ref())
                .wrap_err("failed to parse DKG outcome from epoch E-1 boundary block")?;

        let epoch_e = epoch_e_minus_1_outcome.epoch;
        eprintln!(
            "Epoch E-1 boundary block {} contains outcome for epoch {}",
            boundary_block_num,
            epoch_e.get()
        );

        // Get epoch E block range using .last() for epoch E
        let first_block_e = epocher
            .last(epoch_e.previous().unwrap_or(Epoch::new(0)))
            .map(|h| h.get() + 1)
            .unwrap_or(1); // First epoch starts at block 1
        let last_block_e = epocher
            .last(epoch_e)
            .ok_or_eyre("epoch E is not valid for this epoch strategy")?
            .get();

        eprintln!(
            "Fetching {} blocks from epoch {} (blocks {} to {})...",
            last_block_e - first_block_e + 1,
            epoch_e.get(),
            first_block_e,
            last_block_e
        );

        // Step 2: Build the DKG Info for epoch E
        // The dealers for epoch E are the players from epoch E-1 outcome
        // The players for epoch E are the next_players from epoch E-1 outcome
        let dealers = epoch_e_minus_1_outcome.players().clone();
        let players = epoch_e_minus_1_outcome.next_players().clone();
        let is_full_dkg = epoch_e_minus_1_outcome.is_next_full_dkg;

        let previous_output = if is_full_dkg {
            None
        } else {
            Some(epoch_e_minus_1_outcome.output.clone())
        };

        let info = Info::new::<N3f1>(
            NAMESPACE,
            epoch_e.get(),
            previous_output,
            Mode::NonZeroCounter,
            dealers.clone(),
            players.clone(),
        )
        .wrap_err("failed to create DKG info")?;

        // Step 3: Fetch all block headers from epoch E and extract dealer logs
        let mut logs: BTreeMap<PublicKey, DealerLog<MinSig, PublicKey>> = BTreeMap::new();
        let mut blocks_fetched = 0;

        for block_num in first_block_e..=last_block_e {
            let block = provider
                .get_block_by_number(block_num.into())
                .await
                .wrap_err_with(|| format!("failed to fetch block {block_num}"))?
                .ok_or_else(|| eyre!("block {block_num} not found"))?;

            blocks_fetched += 1;

            // Skip the boundary block (it contains the outcome, not a dealer log)
            if block_num == last_block_e {
                continue;
            }

            let extra_data = &block.header.inner.extra_data;
            if extra_data.is_empty() {
                continue;
            }

            // Try to parse as a SignedDealerLog
            match parse_dealer_log(extra_data, &info, dealers.len() as u32) {
                Ok((dealer, log)) => {
                    eprintln!(
                        "  Block {}: found dealer log from {}",
                        block_num,
                        pubkey_to_hex(&dealer)
                    );
                    logs.insert(dealer, log);
                }
                Err(e) => {
                    eprintln!("  Block {block_num}: failed to parse dealer log: {e}");
                }
            }
        }

        eprintln!(
            "Found {} dealer logs from {} blocks",
            logs.len(),
            blocks_fetched
        );

        // Step 4: Replay the DKG ceremony using observe
        let replayed_output = dkg::observe::<_, _, N3f1>(info, logs.clone(), &Sequential)
            .map_err(|e| eyre!("DKG replay failed: {:?}", e))?;

        // Step 5: Fetch the epoch E boundary block to get the on-chain outcome
        let epoch_e_boundary = provider
            .get_block_by_number(last_block_e.into())
            .await
            .wrap_err_with(|| format!("failed to fetch epoch E boundary block {last_block_e}"))?
            .ok_or_else(|| eyre!("epoch E boundary block {last_block_e} not found"))?;

        let onchain_outcome =
            OnchainDkgOutcome::read(&mut epoch_e_boundary.header.inner.extra_data.as_ref())
                .wrap_err("failed to parse DKG outcome from epoch E boundary block")?;

        // Step 6: Compare results
        let matches = replayed_output == onchain_outcome.output;

        let replayed_sharing = replayed_output.public();
        let onchain_sharing = onchain_outcome.sharing();

        let result = ReplayResult {
            epoch: epoch_e.get(),
            block_range: (first_block_e, last_block_e),
            blocks_fetched,
            dealer_logs_found: logs.len(),
            dealers: logs.keys().map(pubkey_to_hex).collect(),
            matches_onchain: matches,
            replayed_identity: Bytes::copy_from_slice(&replayed_sharing.public().encode()),
            onchain_identity: Bytes::copy_from_slice(&onchain_sharing.public().encode()),
            threshold: replayed_sharing.required::<N3f1>(),
            total_participants: replayed_sharing.total().get(),
        };

        println!("{}", serde_json::to_string_pretty(&result)?);

        if !matches {
            eprintln!("\nWARNING: Replayed DKG outcome does NOT match on-chain outcome!");
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Parse a SignedDealerLog from block extra_data and verify it against the round info.
fn parse_dealer_log(
    extra_data: &Bytes,
    info: &Info<MinSig, PublicKey>,
    max_validators: u32,
) -> eyre::Result<(PublicKey, DealerLog<MinSig, PublicKey>)> {
    let signed_log = SignedDealerLog::<MinSig, PrivateKey>::read_cfg(
        &mut extra_data.as_ref(),
        &NZU32!(max_validators),
    )
    .wrap_err("failed to decode SignedDealerLog")?;

    signed_log
        .check(info)
        .ok_or_else(|| eyre!("dealer log failed verification against round info"))
}
