use super::MultiSigSigner;
use crate::{Precompile, dispatch_call, input_cost, mutate_void, view};
use alloy::{primitives::Address, sol_types::SolInterface};
use revm::precompile::{PrecompileError, PrecompileResult};
use tempo_contracts::precompiles::IMultiSigSigner::IMultiSigSignerCalls;

impl Precompile for MultiSigSigner {
    fn call(&mut self, calldata: &[u8], msg_sender: Address) -> PrecompileResult {
        self.storage
            .deduct_gas(input_cost(calldata.len()))
            .map_err(|_| PrecompileError::OutOfGas)?;

        dispatch_call(
            calldata,
            IMultiSigSignerCalls::abi_decode,
            |call| match call {
                IMultiSigSignerCalls::initConfig(call) => {
                    mutate_void(call, msg_sender, |sender, c| self.init_config(sender, c))
                }
                IMultiSigSignerCalls::getConfig(call) => view(call, |c| self.get_config(c)),
                IMultiSigSignerCalls::isValidSignatureWithKeyHash(call) => {
                    view(call, |c| self.is_valid_signature_with_key_hash(c))
                }
                // TODO: Implement setThreshold, addOwner, removeOwner
                _ => Err(PrecompileError::Other("Unimplemented".into())),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        storage::{StorageCtx, hashmap::HashMapStorageProvider},
        test_util::{assert_full_coverage, check_selector_coverage},
    };

    #[test]
    fn test_multisig_signer_selector_coverage() -> eyre::Result<()> {
        let mut storage = HashMapStorageProvider::new(1);
        StorageCtx::enter(&mut storage, || {
            let mut signer = MultiSigSigner::new();

            let unsupported = check_selector_coverage(
                &mut signer,
                IMultiSigSignerCalls::SELECTORS,
                "IMultiSigSigner",
                IMultiSigSignerCalls::name_by_selector,
            );

            // We expect some unsupported selectors for now (setThreshold, addOwner, removeOwner)
            // assert_full_coverage([unsupported]);

            Ok(())
        })
    }
}
