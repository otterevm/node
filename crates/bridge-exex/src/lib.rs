//! ExEx sidecar for Tempo stablecoin bridge.
//!
//! This sidecar watches origin chains for deposits and submits validator
//! signatures to the Tempo bridge precompile.

pub mod config;
pub mod exex;
pub mod origin_watcher;
pub mod signer;
pub mod tempo_watcher;

pub use config::BridgeConfig;
pub use exex::BridgeExEx;
