use super::RecoveryGuardian;
use crate::{Precompile, dispatch_call, input_cost, mutate, mutate_void, view};
use alloy::{primitives::Address, sol_types::SolInterface};
use revm::precompile::{PrecompileError, PrecompileResult};
use tempo_contracts::precompiles::IRecoveryGuardian::IRecoveryGuardianCalls;

impl Precompile for RecoveryGuardian {
    fn call(&mut self, calldata: &[u8], msg_sender: Address) -> PrecompileResult {
        self.storage
            .deduct_gas(input_cost(calldata.len()))
            .map_err(|_| PrecompileError::OutOfGas)?;

        dispatch_call(
            calldata,
            IRecoveryGuardianCalls::abi_decode,
            |call| match call {
                IRecoveryGuardianCalls::initConfig(call) => {
                    mutate_void(call, msg_sender, |sender, c| self.init_config(sender, c))
                }
                IRecoveryGuardianCalls::initiateRecovery(call) => {
                    mutate_void(call, msg_sender, |sender, c| {
                        self.initiate_recovery(sender, c)
                    })
                }
                IRecoveryGuardianCalls::approveRecovery(call) => {
                    mutate_void(call, msg_sender, |sender, c| self.approve_recovery(sender, c))
                }
                IRecoveryGuardianCalls::cancelRecovery(call) => {
                    mutate_void(call, msg_sender, |sender, c| self.cancel_recovery(sender, c))
                }
                IRecoveryGuardianCalls::executeRecovery(call) => {
                    mutate(call, msg_sender, |sender, c| self.execute_recovery(sender, c))
                }
                IRecoveryGuardianCalls::getConfig(call) => view(call, |c| self.get_config(c)),
                IRecoveryGuardianCalls::getRecoveryRequest(call) => {
                    view(call, |c| self.get_recovery_request(c))
                }
                IRecoveryGuardianCalls::hasApproved(call) => view(call, |c| self.has_approved(c)),
                IRecoveryGuardianCalls::isValidSignatureWithKeyHash(call) => {
                    view(call, |c| self.is_valid_signature_with_key_hash(c))
                }
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
    fn test_recovery_guardian_selector_coverage() -> eyre::Result<()> {
        let mut storage = HashMapStorageProvider::new(1);
        StorageCtx::enter(&mut storage, || {
            let mut guardian = RecoveryGuardian::new();

            let unsupported = check_selector_coverage(
                &mut guardian,
                IRecoveryGuardianCalls::SELECTORS,
                "IRecoveryGuardian",
                IRecoveryGuardianCalls::name_by_selector,
            );

            assert_full_coverage([unsupported]);

            Ok(())
        })
    }
}
