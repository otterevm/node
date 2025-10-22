pub mod evm;
pub mod hashmap;
pub mod slots;

pub mod types;
pub use types::*;

use alloy::primitives::{Address, LogData, U256};
use revm::{
    precompile::PrecompileError,
    state::{AccountInfo, Bytecode},
};

/// Low-level storage provider for interacting with the EVM.
pub trait PrecompileStorageProvider {
    fn chain_id(&self) -> u64;
    fn timestamp(&self) -> U256;
    fn set_code(&mut self, address: Address, code: Bytecode) -> Result<(), PrecompileError>;
    fn get_account_info(&mut self, address: Address) -> Result<AccountInfo, PrecompileError>;
    fn sstore(&mut self, address: Address, key: U256, value: U256) -> Result<(), PrecompileError>;
    fn sload(&mut self, address: Address, key: U256) -> Result<U256, PrecompileError>;
    fn emit_event(&mut self, address: Address, event: LogData) -> Result<(), PrecompileError>;
}

/// Storage operations for a given (contract) address.
pub trait StorageOps {
    fn sstore(&mut self, slot: U256, value: U256) -> Result<(), PrecompileError>;
    fn sload(&mut self, slot: U256) -> Result<U256, PrecompileError>;
}

/// Trait providing access to a contract's address and storage provider.
///
/// Abstracts the common pattern of contracts needing both an address and a mutable reference
/// to a storage provider. It is automatically implemented by the `#[contract]` macro.
pub trait ContractStorage {
    type Storage: PrecompileStorageProvider;
    fn address(&self) -> Address;
    fn storage(&mut self) -> &mut Self::Storage;
}

/// Blanket implementation of `StorageOps` for all type that implement `ContractStorage`.
/// Allows contracts to use `StorageOps` while delegating to `PrecompileStorageProvider`.
impl<T> StorageOps for T
where
    T: ContractStorage,
{
    fn sstore(&mut self, slot: U256, value: U256) -> Result<(), PrecompileError> {
        let address = self.address();
        self.storage().sstore(address, slot, value)
    }

    fn sload(&mut self, slot: U256) -> Result<U256, PrecompileError> {
        let address = self.address();
        self.storage().sload(address, slot)
    }
}
