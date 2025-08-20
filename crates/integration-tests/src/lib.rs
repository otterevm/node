use rayon::iter::ParallelIterator;
use rayon::iter::IntoParallelIterator;
use std::collections::BTreeMap;
use alloy::genesis::GenesisAccount;
use alloy::primitives::Address;
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy::signers::local::coins_bip39::English;
use alloy::signers::local::MnemonicBuilder;
use alloy::signers::utils::secret_key_to_address;
use eyre::Result;

pub const PREFUNDED_MNEMONIC: &str = "test test test test test test test test test test test junk";
pub const PREDEPLOYED_TIP20_ADDRESS: &str = "0x20c0000000000000000000000000000000000000";
pub const NODE_URI: &str = "http://localhost:8545";

fn get_local_provider() -> Result<DynProvider> {
    Ok(ProviderBuilder::new()
        .connect_http(NODE_URI.parse()?)
        .erased())
}

fn get_prefunded_addresses(n: u32) -> Result<Vec<Address>> {
    (0..n)
        .into_par_iter()
        .map(|worker_id| -> eyre::Result<Address> {
            let signer = MnemonicBuilder::<English>::default()
                .phrase(PREFUNDED_MNEMONIC)
                .index(worker_id)?
                .build()?;
            Ok(secret_key_to_address(signer.credential()))
        })
        .collect::<eyre::Result<Vec<Address>>>()
}

mod tip20_precompiles;
