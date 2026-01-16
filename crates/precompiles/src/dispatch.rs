//! Re-exports from runtime for macro compatibility.

pub use crate::runtime::{
    Precompile, dispatch_call, extend_tempo_precompiles, input_cost, metadata, mutate, mutate_void,
    unknown_selector, view,
};

#[cfg(test)]
pub use crate::runtime::expect_precompile_revert;
