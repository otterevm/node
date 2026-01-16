//! Tempo precompile implementations.
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use alloy::primitives::{Address, address};

pub mod abi;

#[cfg(feature = "precompile")]
pub mod dispatch;
#[cfg(feature = "precompile")]
pub mod runtime;
#[cfg(feature = "precompile")]
pub use runtime::*;

#[cfg(feature = "precompile")]
pub mod error;
#[cfg(feature = "precompile")]
pub mod storage;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_util;

#[cfg(feature = "precompile")]
pub use tempo_contracts::precompiles::{
    ACCOUNT_KEYCHAIN_ADDRESS, DEFAULT_FEE_TOKEN, NONCE_PRECOMPILE_ADDRESS, PATH_USD_ADDRESS,
    STABLECOIN_DEX_ADDRESS, TIP_FEE_MANAGER_ADDRESS, VALIDATOR_CONFIG_ADDRESS,
};

pub const TIP403_REGISTRY_ADDRESS: Address = address!("0x403C000000000000000000000000000000000000");
pub const TIP20_FACTORY_ADDRESS: Address = address!("0x20FC000000000000000000000000000000000000");

// Re-export storage layout helpers for read-only contexts (e.g., pool validation)
#[cfg(feature = "precompile")]
pub use account_keychain::AuthorizedKey;
