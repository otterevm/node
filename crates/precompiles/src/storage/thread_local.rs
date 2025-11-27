use alloy::primitives::{Address, LogData, U256};
use revm::state::{AccountInfo, Bytecode};
use std::{cell::Cell, marker::PhantomData};
use tempo_chainspec::hardfork::TempoHardfork;

use crate::{
    error::{Result, TempoPrecompileError},
    storage::PrecompileStorageProvider,
};

// Thread-local storage for accessing `PrecompileStorageProvider`
thread_local! {
    static STORAGE: Cell<Option<*mut dyn PrecompileStorageProvider>> = const { Cell::new(None) };
}

/// Thread-local storage guard for precompiles.
///
/// This guard sets up thread-local access to a storage provider for the duration
/// of its lifetime. When dropped, it cleans up the thread-local storage.
///
/// # IMPORTANT
///
/// The caller must ensure that:
/// 1. Only one `StorageGuard` exists at a time, in the same thread.
/// 2. If multiple storage providers are instantiated in parallel threads,
///    they CANNOT point to the same storage addresses.
#[derive(Default)]
pub struct StorageGuard<'s> {
    _storage: PhantomData<&'s mut dyn PrecompileStorageProvider>,
}

impl<'s> StorageGuard<'s> {
    /// Creates a new storage guard, initializing thread-local storage.
    /// See type-level documentation for important notes.
    pub fn new(storage: &'s mut dyn PrecompileStorageProvider) -> Result<Self> {
        if STORAGE.with(|s| s.get()).is_some() {
            return Err(TempoPrecompileError::Fatal(
                "'StorageGuard' already initialized".to_string(),
            ));
        }

        // SAFETY: Transmuting lifetime to 'static for `Cell` storage.
        //
        // This is safe because:
        // 1. Type system ensures this guard can't outlive 's
        // 2. The Drop impl clears the thread-local before the guard is destroyed
        // 3. Only one guard can exist per thread (checked above)
        let ptr: *mut dyn PrecompileStorageProvider = storage;
        let ptr_static: *mut (dyn PrecompileStorageProvider + 'static) =
            unsafe { std::mem::transmute(ptr) };

        STORAGE.with(|s| s.set(Some(ptr_static)));

        Ok(Self::default())
    }
}

impl Drop for StorageGuard<'_> {
    fn drop(&mut self) {
        STORAGE.with(|s| s.set(None));
    }
}

/// Thread-local storage accessor that implements `PrecompileStorageProvider` without the trait bound.
///
/// # Important
///
/// Since it provides access to the current thread-local storage context, it MUST be used with an active `StorageGuard`.
///
/// # Sync with `PrecompileStorageProvider`
///
/// This type mirrors `PrecompileStorageProvider` methods but with split mutability:
/// - Read operations (staticcall) take `&self`
/// - Write operations take `&mut self`
#[derive(Debug, Default, Clone, Copy)]
pub struct StorageAccessor;

impl StorageAccessor {
    /// Execute a function with access to the current thread-local storage provider.
    fn with_storage_call<F, R>(f: F) -> Result<R>
    where
        F: FnOnce(&mut dyn PrecompileStorageProvider) -> Result<R>,
    {
        let storage_ptr = STORAGE
            .with(|s| s.get())
            .ok_or(TempoPrecompileError::Fatal(
                "No storage context. 'StorageGuard' must be initialized".to_string(),
            ))?;

        // SAFETY:
        // - Caller must ensure NO recursive calls.
        // - Type system ensures the storage pointer is valid.
        let storage = unsafe { &mut *storage_ptr };
        f(storage)
    }

    // `PrecompileStorageProvider` methods (with modified mutability for read-only methods)

    pub fn chain_id(&self) -> u64 {
        // NOTE: safe to unwrap as `chain_id()` is infallible.
        Self::with_storage_call(|s| Ok(s.chain_id())).unwrap()
    }

    pub fn timestamp(&self) -> U256 {
        // NOTE: safe to unwrap as `timestamp()` is infallible.
        Self::with_storage_call(|s| Ok(s.timestamp())).unwrap()
    }

    pub fn beneficiary(&self) -> Address {
        // NOTE: safe to unwrap as `beneficiary()` is infallible.
        Self::with_storage_call(|s| Ok(s.beneficiary())).unwrap()
    }

    pub fn set_code(&mut self, address: Address, code: Bytecode) -> Result<()> {
        Self::with_storage_call(|s| s.set_code(address, code))
    }

    pub fn get_account_info(&self, address: Address) -> Result<&'_ AccountInfo> {
        // SAFETY: The returned reference is valid for the duration of the
        // `StorageGuard`'s lifetime. Since `StorageAccessor` can only be used
        // while a guard is active, the reference remains valid.
        Self::with_storage_call(|s| {
            let info = s.get_account_info(address)?;
            // Extend the lifetime to match &'_ self
            // This is safe because the underlying storage outlives the accessor
            let info: &'_ AccountInfo = unsafe { &*(info as *const AccountInfo) };
            Ok(info)
        })
    }

    pub fn sload(&self, address: Address, key: U256) -> Result<U256> {
        Self::with_storage_call(|s| s.sload(address, key))
    }

    pub fn tload(&self, address: Address, key: U256) -> Result<U256> {
        Self::with_storage_call(|s| s.tload(address, key))
    }

    pub fn sstore(&mut self, address: Address, key: U256, value: U256) -> Result<()> {
        Self::with_storage_call(|s| s.sstore(address, key, value))
    }

    pub fn tstore(&mut self, address: Address, key: U256, value: U256) -> Result<()> {
        Self::with_storage_call(|s| s.tstore(address, key, value))
    }

    pub fn emit_event(&mut self, address: Address, event: LogData) -> Result<()> {
        Self::with_storage_call(|s| s.emit_event(address, event))
    }

    pub fn deduct_gas(&mut self, gas: u64) -> Result<()> {
        Self::with_storage_call(|s| s.deduct_gas(gas))
    }

    pub fn gas_used(&self) -> u64 {
        // NOTE: safe to unwrap as `gas_used()` is infallible.
        Self::with_storage_call(|s| Ok(s.gas_used())).unwrap()
    }

    pub fn spec(&self) -> TempoHardfork {
        // NOTE: safe to unwrap as `spec()` is infallible.
        Self::with_storage_call(|s| Ok(s.spec())).unwrap()
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn get_events(&self, address: Address) -> &Vec<LogData> {
        // SAFETY: The returned reference is valid for the duration of the
        // `StorageGuard`'s lifetime. Since `StorageAccessor` can only be used
        // while a guard is active, the reference remains valid.
        Self::with_storage_call(|s| {
            let events = s.get_events(address);
            let events: &'_ Vec<LogData> = unsafe { &*(events as *const Vec<LogData>) };
            Ok(events)
        })
        .unwrap()
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_nonce(&mut self, address: Address, nonce: u64) {
        Self::with_storage_call(|s| {
            s.set_nonce(address, nonce);
            Ok(())
        })
        .unwrap()
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_timestamp(&mut self, timestamp: U256) {
        Self::with_storage_call(|s| {
            s.set_timestamp(timestamp);
            Ok(())
        })
        .unwrap()
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_beneficiary(&mut self, beneficiary: Address) {
        Self::with_storage_call(|s| {
            s.set_beneficiary(beneficiary);
            Ok(())
        })
        .unwrap()
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn set_spec(&mut self, spec: TempoHardfork) {
        Self::with_storage_call(|s| {
            s.set_spec(spec);
            Ok(())
        })
        .unwrap()
    }
}
