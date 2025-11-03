//! Code generation for primitive type storage implementations.

use proc_macro2::TokenStream;
use quote::quote;

const RUST_INT_SIZES: &[usize] = &[8, 16, 32, 64, 128];
const ALLOY_INT_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256];

// -- CONFIGURATION TYPES ------------------------------------------------------

/// Strategy for converting to U256
#[derive(Debug, Clone)]
enum StorableConversionStrategy {
    U256, // no conversion needed (identity)
    Unsigned,
    SignedRust(proc_macro2::Ident),
    SignedAlloy(proc_macro2::Ident),
    FixedBytes(usize),
}

/// Strategy for converting to storage key bytes
#[derive(Debug, Clone)]
enum StorageKeyStrategy {
    Simple,           // `self.to_be_bytes()`
    WithSize(usize),  // `self.to_be_bytes::<N>()`
    SignedRaw(usize), // `self.into_raw().to_be_bytes::<N>()`
    AsSlice,          // `self.as_slice()`
}

/// Complete configuration for generating implementations for a type
#[derive(Debug, Clone)]
struct TypeConfig {
    type_path: TokenStream,
    byte_count: usize,
    storable_strategy: StorableConversionStrategy,
    storage_key_strategy: StorageKeyStrategy,
}

/// Configuration for generating tests
#[derive(Debug, Clone)]
struct TestConfig {
    type_path: TokenStream,
    byte_count: usize,
    is_signed: bool,
    unsigned_type: Option<proc_macro2::Ident>,
    sign_variant: Option<&'static str>, // "positive" | "negative"
    use_alloy_random: bool,             // false == use proptest
}

impl TestConfig {
    /// Returns the qualified unsigned type path, adding `::alloy::primitives::` prefix for alloy types
    fn qualified_unsigned_type(&self) -> Option<TokenStream> {
        self.unsigned_type.as_ref().map(|ut| {
            if self.use_alloy_random {
                quote! { ::alloy::primitives::#ut }
            } else {
                quote! { #ut }
            }
        })
    }

    /// Generates a standardized test name based on type path, suffix, and sign variant
    fn test_name(&self, suffix: &str) -> proc_macro2::Ident {
        // Clean up type path for test name: remove all whitespace, colons, angle brackets
        let type_str = self
            .type_path
            .to_string()
            .replace(|c: char| c.is_whitespace(), "")
            .replace("::", "_")
            .replace(['<', '>', ','], "_")
            .to_lowercase();
        let test_name_str = format!(
            "test_{}_{}{}",
            type_str,
            suffix,
            self.sign_variant.map_or(String::new(), |v| format!("_{v}"))
        );
        quote::format_ident!("{}", test_name_str)
    }

    /// Returns code for edge case array initialization
    fn edge_cases_code(&self) -> TokenStream {
        if self.is_signed && self.unsigned_type.is_some() {
            let type_path = &self.type_path;
            quote! { [#type_path::ZERO, #type_path::MINUS_ONE, #type_path::MAX, #type_path::MIN] }
        } else if self.use_alloy_random {
            let type_path = &self.type_path;
            quote! { [#type_path::ZERO, #type_path::MAX] }
        } else {
            let type_path = &self.type_path;
            quote! { [#type_path::MIN, #type_path::MAX, 0] }
        }
    }

    /// Returns code for random value generation
    fn random_value_code(&self) -> TokenStream {
        if self.is_signed && self.unsigned_type.is_some() {
            let type_path = &self.type_path;
            let qual_unsigned = self.qualified_unsigned_type().unwrap();
            quote! {
                let unsigned_value = #qual_unsigned::random();
                let value = #type_path::from_raw(unsigned_value);
            }
        } else if self.use_alloy_random {
            let type_path = &self.type_path;
            quote! { let value = #type_path::random(); }
        } else {
            quote! {
                let value = strategy.new_tree(&mut runner).unwrap().current();
            }
        }
    }
}

// -- IMPLEMENTATION GENERATORS ------------------------------------------------

/// Generate a `StorableType` implementation
fn gen_storable_type_impl(type_path: &TokenStream, byte_count: usize) -> TokenStream {
    quote! {
        impl StorableType for #type_path {
            const BYTE_COUNT: usize = #byte_count;
        }
    }
}

/// Generate a `StorageKey` implementation based on the conversion strategy
fn gen_storage_key_impl(type_path: &TokenStream, strategy: &StorageKeyStrategy) -> TokenStream {
    let conversion = match strategy {
        StorageKeyStrategy::Simple => quote! { self.to_be_bytes() },
        StorageKeyStrategy::WithSize(size) => quote! { self.to_be_bytes::<#size>() },
        StorageKeyStrategy::SignedRaw(size) => quote! { self.into_raw().to_be_bytes::<#size>() },
        StorageKeyStrategy::AsSlice => quote! { self.as_slice() },
    };

    quote! {
        impl StorageKey for #type_path {
            #[inline]
            fn as_storage_bytes(&self) -> impl AsRef<[u8]> {
                #conversion
            }
        }
    }
}

/// Generate a `Storable<1>` implementation based on the conversion strategy
fn gen_storable_impl(
    type_path: &TokenStream,
    strategy: &StorableConversionStrategy,
) -> TokenStream {
    match strategy {
        StorableConversionStrategy::Unsigned => {
            quote! {
                impl Storable<1> for #type_path {
                    #[inline]
                    fn load<S: StorageOps>(storage: &mut S, base_slot: U256) -> Result<Self> {
                        let value = storage.sload(base_slot)?;
                        Ok(value.to::<Self>())
                    }

                    #[inline]
                    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: U256) -> Result<()> {
                        storage.sstore(base_slot, U256::from(*self))
                    }

                    #[inline]
                    fn to_evm_words(&self) -> Result<[U256; 1]> {
                        Ok([U256::from(*self)])
                    }

                    #[inline]
                    fn from_evm_words(words: [U256; 1]) -> Result<Self> {
                        Ok(words[0].to::<Self>())
                    }
                }
            }
        }
        StorableConversionStrategy::U256 => {
            quote! {
                impl Storable<1> for #type_path {
                    #[inline]
                    fn load<S: StorageOps>(storage: &mut S, base_slot: #type_path) -> Result<Self> {
                        storage.sload(base_slot)
                    }

                    #[inline]
                    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: #type_path) -> Result<()> {
                        storage.sstore(base_slot, *self)
                    }

                    #[inline]
                    fn to_evm_words(&self) -> Result<[#type_path; 1]> {
                        Ok([*self])
                    }

                    #[inline]
                    fn from_evm_words(words: [#type_path; 1]) -> Result<Self> {
                        Ok(words[0])
                    }
                }
            }
        }
        StorableConversionStrategy::SignedRust(unsigned_type) => {
            quote! {
                impl Storable<1> for #type_path {
                    #[inline]
                    fn load<S: StorageOps>(storage: &mut S, base_slot: U256) -> Result<Self> {
                        let value = storage.sload(base_slot)?;
                        // Read as unsigned then cast to signed (preserves bit pattern)
                        Ok(value.to::<#unsigned_type>() as Self)
                    }

                    #[inline]
                    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: U256) -> Result<()> {
                        // Cast to unsigned to preserve bit pattern, then extend to U256
                        storage.sstore(base_slot, U256::from(*self as #unsigned_type))
                    }

                    #[inline]
                    fn to_evm_words(&self) -> Result<[U256; 1]> {
                        Ok([U256::from(*self as #unsigned_type)])
                    }

                    #[inline]
                    fn from_evm_words(words: [U256; 1]) -> Result<Self> {
                        Ok(words[0].to::<#unsigned_type>() as Self)
                    }
                }
            }
        }
        StorableConversionStrategy::SignedAlloy(unsigned_type) => {
            quote! {
                impl Storable<1> for #type_path {
                    #[inline]
                    fn load<S: StorageOps>(storage: &mut S, base_slot: ::alloy::primitives::U256) -> Result<Self> {
                        let value = storage.sload(base_slot)?;
                        // Convert U256 to unsigned type, then reinterpret as signed
                        let unsigned_val = value.to::<::alloy::primitives::#unsigned_type>();
                        Ok(Self::from_raw(unsigned_val))
                    }

                    #[inline]
                    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: ::alloy::primitives::U256) -> Result<()> {
                        // Get unsigned bit pattern and store it
                        let unsigned_val = self.into_raw();
                        storage.sstore(base_slot, ::alloy::primitives::U256::from(unsigned_val))
                    }

                    #[inline]
                    fn to_evm_words(&self) -> Result<[::alloy::primitives::U256; 1]> {
                        let unsigned_val = self.into_raw();
                        Ok([::alloy::primitives::U256::from(unsigned_val)])
                    }

                    #[inline]
                    fn from_evm_words(words: [::alloy::primitives::U256; 1]) -> Result<Self> {
                        let unsigned_val = words[0].to::<::alloy::primitives::#unsigned_type>();
                        Ok(Self::from_raw(unsigned_val))
                    }
                }
            }
        }
        StorableConversionStrategy::FixedBytes(size) => {
            quote! {
                impl Storable<1> for #type_path {
                    #[inline]
                    fn load<S: StorageOps>(storage: &mut S, base_slot: ::alloy::primitives::U256) -> Result<Self> {
                        let value = storage.sload(base_slot)?;
                        // `FixedBytes` are stored left-aligned in the slot. Extract the first N bytes from the U256
                        let bytes = value.to_be_bytes::<32>();
                        let mut fixed_bytes = [0u8; #size];
                        fixed_bytes.copy_from_slice(&bytes[..#size]);
                        Ok(Self::from(fixed_bytes))
                    }

                    #[inline]
                    fn store<S: StorageOps>(&self, storage: &mut S, base_slot: ::alloy::primitives::U256) -> Result<()> {
                        // Pad `FixedBytes` to 32 bytes (left-aligned).
                        let mut bytes = [0u8; 32];
                        bytes[..#size].copy_from_slice(&self[..]);
                        let value = ::alloy::primitives::U256::from_be_bytes(bytes);
                        storage.sstore(base_slot, value)
                    }

                    #[inline]
                    fn to_evm_words(&self) -> Result<[::alloy::primitives::U256; 1]> {
                        let mut bytes = [0u8; 32];
                        bytes[..#size].copy_from_slice(&self[..]);
                        Ok([::alloy::primitives::U256::from_be_bytes(bytes)])
                    }

                    #[inline]
                    fn from_evm_words(words: [::alloy::primitives::U256; 1]) -> Result<Self> {
                        let bytes = words[0].to_be_bytes::<32>();
                        let mut fixed_bytes = [0u8; #size];
                        fixed_bytes.copy_from_slice(&bytes[..#size]);
                        Ok(Self::from(fixed_bytes))
                    }
                }
            }
        }
    }
}

/// Generate all storage-related impls for a type
fn gen_complete_impl_set(config: &TypeConfig) -> TokenStream {
    let storable_type_impl = gen_storable_type_impl(&config.type_path, config.byte_count);
    let storable_impl = gen_storable_impl(&config.type_path, &config.storable_strategy);
    let storage_key_impl = gen_storage_key_impl(&config.type_path, &config.storage_key_strategy);

    quote! {
        #storable_type_impl
        #storable_impl
        #storage_key_impl
    }
}

/// Generate `StorableType` and `Storable<1>` implementations for all standard Rust integer types.
pub(crate) fn gen_storable_rust_ints() -> TokenStream {
    let mut impls = Vec::with_capacity(RUST_INT_SIZES.len() * 2);
    let mut tests = Vec::with_capacity(RUST_INT_SIZES.len() * 6);

    for size in RUST_INT_SIZES {
        let unsigned_type = quote::format_ident!("u{}", size);
        let signed_type = quote::format_ident!("i{}", size);
        let byte_count = size / 8;

        // Generate unsigned integer configuration and implementation
        let unsigned_config = TypeConfig {
            type_path: quote! { #unsigned_type },
            byte_count,
            storable_strategy: StorableConversionStrategy::Unsigned,
            storage_key_strategy: StorageKeyStrategy::Simple,
        };
        impls.push(gen_complete_impl_set(&unsigned_config));

        // Generate signed integer configuration and implementation
        let signed_config = TypeConfig {
            type_path: quote! { #signed_type },
            byte_count,
            storable_strategy: StorableConversionStrategy::SignedRust(unsigned_type.clone()),
            storage_key_strategy: StorageKeyStrategy::Simple,
        };
        impls.push(gen_complete_impl_set(&signed_config));

        // Generate tests for both unsigned and signed types
        let unsigned_test_config = TestConfig {
            type_path: quote! { #unsigned_type },
            byte_count,
            is_signed: false,
            unsigned_type: None,
            sign_variant: None,
            use_alloy_random: false,
        };

        let signed_test_config_positive = TestConfig {
            type_path: quote! { #signed_type },
            byte_count,
            is_signed: true,
            unsigned_type: Some(unsigned_type.clone()),
            sign_variant: Some("positive"),
            use_alloy_random: false,
        };

        let signed_test_config_negative = TestConfig {
            type_path: quote! { #signed_type },
            byte_count,
            is_signed: true,
            unsigned_type: Some(unsigned_type),
            sign_variant: Some("negative"),
            use_alloy_random: false,
        };

        tests.extend(gen_integer_test_suite(
            &unsigned_test_config,
            &signed_test_config_positive,
            &signed_test_config_negative,
        ));
    }

    quote! {
        #(#impls)*

        #[cfg(test)]
        mod generated_storable_integer_tests {
            use super::*;

            #(#tests)*
        }
    }
}

/// Generate `StorableType` and `Storable<1>` implementations for alloy integer types.
fn gen_alloy_integers() -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut impls = Vec::with_capacity(ALLOY_INT_SIZES.len() * 2);
    let mut tests = Vec::with_capacity(ALLOY_INT_SIZES.len() * 6);

    for &size in ALLOY_INT_SIZES {
        let unsigned_type = quote::format_ident!("U{}", size);
        let signed_type = quote::format_ident!("I{}", size);
        let byte_count = size / 8;

        // Generate unsigned integer configuration and implementation
        let unsigned_config = TypeConfig {
            type_path: quote! { ::alloy::primitives::#unsigned_type },
            byte_count,
            storable_strategy: if size == 256 {
                StorableConversionStrategy::U256
            } else {
                StorableConversionStrategy::Unsigned
            },
            storage_key_strategy: StorageKeyStrategy::WithSize(byte_count),
        };
        impls.push(gen_complete_impl_set(&unsigned_config));

        // Generate signed integer configuration and implementation
        let signed_config = TypeConfig {
            type_path: quote! { ::alloy::primitives::#signed_type },
            byte_count,
            storable_strategy: StorableConversionStrategy::SignedAlloy(unsigned_type.clone()),
            storage_key_strategy: StorageKeyStrategy::SignedRaw(byte_count),
        };
        impls.push(gen_complete_impl_set(&signed_config));

        // Generate tests for both unsigned and signed types
        let unsigned_test_config = TestConfig {
            type_path: quote! { ::alloy::primitives::#unsigned_type },
            byte_count,
            is_signed: false,
            unsigned_type: None,
            sign_variant: None,
            use_alloy_random: true,
        };

        let signed_test_config_positive = TestConfig {
            type_path: quote! { ::alloy::primitives::#signed_type },
            byte_count,
            is_signed: true,
            unsigned_type: Some(quote::format_ident!("U{}", size)),
            sign_variant: Some("positive"),
            use_alloy_random: true,
        };

        let signed_test_config_negative = TestConfig {
            type_path: quote! { ::alloy::primitives::#signed_type },
            byte_count,
            is_signed: true,
            unsigned_type: Some(quote::format_ident!("U{}", size)),
            sign_variant: Some("negative"),
            use_alloy_random: true,
        };

        tests.extend(gen_integer_test_suite(
            &unsigned_test_config,
            &signed_test_config_positive,
            &signed_test_config_negative,
        ));
    }

    (impls, tests)
}

/// Generate `StorableType` and `Storable<1>` implementations for FixedBytes<N> types.
fn gen_fixed_bytes(sizes: &[usize]) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut impls = Vec::with_capacity(sizes.len());
    let mut tests = Vec::with_capacity(sizes.len() * 2);

    for &size in sizes {
        // Generate FixedBytes configuration and implementation
        let config = TypeConfig {
            type_path: quote! { ::alloy::primitives::FixedBytes<#size> },
            byte_count: size,
            storable_strategy: StorableConversionStrategy::FixedBytes(size),
            storage_key_strategy: StorageKeyStrategy::AsSlice,
        };
        impls.push(gen_complete_impl_set(&config));

        // Generate tests
        tests.extend(gen_fixed_bytes_tests(size));
    }

    (impls, tests)
}

/// Generate `StorableType` and `Storable<1>` implementations for FixedBytes<N> types.
pub(crate) fn gen_storable_alloy_bytes() -> TokenStream {
    let sizes: Vec<usize> = (1..=32).collect();
    let (impls, tests) = gen_fixed_bytes(&sizes);

    quote! {
        #(#impls)*

        #[cfg(test)]
        mod generated_storable_fixedbytes_tests {
            use super::*;

            #(#tests)*
        }
    }
}

/// Generate `StorableType` and `Storable<1>` implementations for all alloy integer types.
pub(crate) fn gen_storable_alloy_ints() -> TokenStream {
    let (impls, tests) = gen_alloy_integers();

    quote! {
        #(#impls)*

        #[cfg(test)]
        mod generated_storable_alloy_integer_tests {
            use super::*;

            #(#tests)*
        }
    }
}

// -- TEST HELPERS -------------------------------------------------------------

/// Generate positive signed value code (edge case and random value transformation)
fn gen_positive_signed_value(config: &TestConfig) -> (TokenStream, TokenStream) {
    let type_path = &config.type_path;
    let qual_unsigned = config
        .qualified_unsigned_type()
        .expect("unsigned_type required for signed variant");

    if config.use_alloy_random {
        (
            quote! { #type_path::MAX },
            quote! {
                let unsigned_value = #qual_unsigned::random();
                // Mask to ensure it fits in the positive range (clear sign bit)
                let positive_unsigned = unsigned_value & (#qual_unsigned::MAX >> 1);
                let value = #type_path::from_raw(positive_unsigned);
            },
        )
    } else {
        let unsigned_type = config.unsigned_type.as_ref().unwrap();
        (
            quote! { #type_path::MAX },
            quote! {
                let unsigned_val = unsigned_strategy.new_tree(&mut runner).unwrap().current();
                // Mask to keep only positive range
                let masked_val = unsigned_val & (#type_path::MAX as #unsigned_type);
                let value = masked_val as #type_path;
            },
        )
    }
}

/// Generate negative signed value code (edge case and random value transformation)
fn gen_negative_signed_value(config: &TestConfig) -> (TokenStream, TokenStream) {
    let type_path = &config.type_path;
    let qual_unsigned = config
        .qualified_unsigned_type()
        .expect("unsigned_type required for signed variant");

    if config.use_alloy_random {
        (
            quote! { #type_path::MIN },
            quote! {
                let unsigned_value = #qual_unsigned::random();
                // Mask to ensure it fits in the positive range
                let positive_unsigned = unsigned_value & (#qual_unsigned::MAX >> 1);
                let positive_value = #type_path::from_raw(positive_unsigned);
                // Negate to get a negative value (handles 0 case naturally)
                let value = -positive_value;
            },
        )
    } else {
        (
            quote! { #type_path::MIN },
            quote! {
                let unsigned_val = unsigned_strategy.new_tree(&mut runner).unwrap().current();
                // For negative values, negate the unsigned value first to avoid overflow issues
                let value = (unsigned_val.wrapping_neg() as #type_path);
            },
        )
    }
}

/// Generate an EVM words roundtrip test based on configuration
fn gen_evm_roundtrip_test(config: &TestConfig) -> TokenStream {
    let type_path = &config.type_path;
    let test_name = config.test_name("evm_words_roundtrip");

    if config.is_signed && config.sign_variant.is_some() {
        // Signed type with positive/negative variant
        let (edge_case, value_transform) = if config.sign_variant == Some("positive") {
            gen_positive_signed_value(config)
        } else {
            gen_negative_signed_value(config)
        };

        if config.use_alloy_random {
            quote! {
                #[test]
                fn #test_name() {
                    // Test edge case
                    let value = #edge_case;
                    let words = value.to_evm_words().expect("to_evm_words failed");
                    let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                    assert_eq!(value, recovered, "EVM words round-trip failed for edge case");

                    // Test random values
                    for _ in 0..100 {
                        #value_transform
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for random value");
                    }
                }
            }
        } else {
            let unsigned_type = config.unsigned_type.as_ref().unwrap();
            quote! {
                #[test]
                fn #test_name() {
                    use proptest::test_runner::{Config, TestRunner};
                    use proptest::strategy::{Strategy, ValueTree};

                    // Test edge case
                    let value = #edge_case;
                    let words = value.to_evm_words().expect("to_evm_words failed");
                    let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                    assert_eq!(value, recovered, "EVM words round-trip failed for edge case {}", value);

                    // Test random values using proptest
                    let mut runner = TestRunner::new(Config::default());
                    let unsigned_strategy = proptest::arbitrary::any::<#unsigned_type>();

                    for _ in 0..100 {
                        #value_transform
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for random value {}", value);
                    }
                }
            }
        }
    } else {
        // Unsigned type or signed without variant
        let edge_cases = config.edge_cases_code();
        let random_value = config.random_value_code();

        if config.use_alloy_random {
            quote! {
                #[test]
                fn #test_name() {
                    // Test edge cases
                    let edge_cases = #edge_cases;
                    for value in edge_cases {
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for edge case");
                    }

                    // Test random values
                    for _ in 0..100 {
                        #random_value
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for random value");
                    }
                }
            }
        } else {
            quote! {
                #[test]
                fn #test_name() {
                    use proptest::test_runner::{Config, TestRunner};
                    use proptest::strategy::{Strategy, ValueTree};

                    // Test edge cases
                    let edge_cases = #edge_cases;
                    for &value in &edge_cases {
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for edge case {}", value);
                    }

                    // Test random values using proptest
                    let mut runner = TestRunner::new(Config::default());
                    let strategy = proptest::arbitrary::any::<#type_path>();

                    for _ in 0..100 {
                        #random_value
                        let words = value.to_evm_words().expect("to_evm_words failed");
                        let recovered = #type_path::from_evm_words(words).expect("from_evm_words failed");
                        assert_eq!(value, recovered, "EVM words round-trip failed for random value {}", value);
                    }
                }
            }
        }
    }
}

/// Generate a storage key test based on configuration
fn gen_storage_key_test(config: &TestConfig) -> TokenStream {
    let type_path = &config.type_path;
    let byte_count = config.byte_count;
    let test_name = config.test_name("storage_key");

    if config.is_signed && config.sign_variant.is_some() {
        // Signed type with positive/negative variant
        let (edge_case, value_transform) = if config.sign_variant == Some("positive") {
            gen_positive_signed_value(config)
        } else {
            gen_negative_signed_value(config)
        };

        let expected_bytes = if config.use_alloy_random {
            quote! { let expected_bytes = value.into_raw().to_be_bytes::<#byte_count>(); }
        } else {
            quote! {}
        };

        let bytes_assertion = if config.use_alloy_random {
            quote! { assert_eq!(bytes.as_ref(), &expected_bytes, "StorageKey bytes mismatch for edge case"); }
        } else {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes(), "StorageKey bytes mismatch for edge case {}", value); }
        };

        let random_bytes_assertion = if config.use_alloy_random {
            quote! {
                let expected_bytes = value.into_raw().to_be_bytes::<#byte_count>();
                assert_eq!(bytes.as_ref(), &expected_bytes, "StorageKey bytes mismatch for random value");
            }
        } else {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes(), "StorageKey bytes mismatch for random value"); }
        };

        if config.use_alloy_random {
            quote! {
                #[test]
                fn #test_name() {
                    // Test byte length and edge case
                    let value = #edge_case;
                    let bytes = value.as_storage_bytes();
                    assert_eq!(bytes.as_ref().len(), #byte_count, "StorageKey byte length mismatch");
                    #expected_bytes
                    #bytes_assertion

                    // Test random values
                    for _ in 0..100 {
                        #value_transform
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #random_bytes_assertion
                    }
                }
            }
        } else {
            let unsigned_type = config.unsigned_type.as_ref().unwrap();
            quote! {
                #[test]
                fn #test_name() {
                    use proptest::test_runner::{Config, TestRunner};
                    use proptest::strategy::{Strategy, ValueTree};

                    // Test byte length and edge case
                    let value = #edge_case;
                    let bytes = value.as_storage_bytes();
                    assert_eq!(bytes.as_ref().len(), #byte_count, "StorageKey byte length mismatch");
                    #bytes_assertion

                    // Test random values using proptest
                    let mut runner = TestRunner::new(Config::default());
                    let unsigned_strategy = proptest::arbitrary::any::<#unsigned_type>();

                    for _ in 0..100 {
                        #value_transform
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #random_bytes_assertion
                    }
                }
            }
        }
    } else {
        // Unsigned type or signed without variant
        let edge_cases = config.edge_cases_code();
        let random_value = config.random_value_code();

        let edge_case_assertion = if config.is_signed && config.unsigned_type.is_some() {
            quote! {
                let expected_bytes = value.into_raw().to_be_bytes::<#byte_count>();
                assert_eq!(bytes.as_ref(), &expected_bytes, "StorageKey bytes mismatch for edge case");
            }
        } else if config.use_alloy_random {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes::<#byte_count>(), "StorageKey bytes mismatch for edge case"); }
        } else {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes(), "StorageKey bytes mismatch for edge case {}", value); }
        };

        let random_assertion = if config.is_signed && config.unsigned_type.is_some() {
            quote! {
                let expected_bytes = value.into_raw().to_be_bytes::<#byte_count>();
                assert_eq!(bytes.as_ref(), &expected_bytes, "StorageKey bytes mismatch for random value");
            }
        } else if config.use_alloy_random {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes::<#byte_count>(), "StorageKey bytes mismatch for random value"); }
        } else {
            quote! { assert_eq!(bytes.as_ref(), &value.to_be_bytes(), "StorageKey bytes mismatch for random value"); }
        };

        if config.use_alloy_random {
            quote! {
                #[test]
                fn #test_name() {
                    // Test byte length
                    let value = #type_path::MAX;
                    let bytes = value.as_storage_bytes();
                    assert_eq!(bytes.as_ref().len(), #byte_count, "StorageKey byte length mismatch");

                    // Test edge cases
                    let edge_cases = #edge_cases;
                    for value in edge_cases {
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #edge_case_assertion
                    }

                    // Test random values
                    for _ in 0..100 {
                        #random_value
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #random_assertion
                    }
                }
            }
        } else {
            quote! {
                #[test]
                fn #test_name() {
                    use proptest::test_runner::{Config, TestRunner};
                    use proptest::strategy::{Strategy, ValueTree};

                    // Test byte length
                    let value = #type_path::MAX;
                    let bytes = value.as_storage_bytes();
                    assert_eq!(bytes.as_ref().len(), #byte_count, "StorageKey byte length mismatch");

                    // Test edge cases
                    let edge_cases = #edge_cases;
                    for &value in &edge_cases {
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #edge_case_assertion
                    }

                    // Test random values using proptest
                    let mut runner = TestRunner::new(Config::default());
                    let strategy = proptest::arbitrary::any::<#type_path>();

                    for _ in 0..100 {
                        #random_value
                        let bytes = value.as_storage_bytes();
                        assert_eq!(bytes.as_ref().len(), #byte_count);
                        #random_assertion
                    }
                }
            }
        }
    }
}

/// Generate a complete test suite for an unsigned/signed integer pair
fn gen_integer_test_suite(
    unsigned_config: &TestConfig,
    signed_config_positive: &TestConfig,
    signed_config_negative: &TestConfig,
) -> Vec<TokenStream> {
    vec![
        gen_evm_roundtrip_test(unsigned_config),
        gen_evm_roundtrip_test(signed_config_positive),
        gen_evm_roundtrip_test(signed_config_negative),
        gen_storage_key_test(unsigned_config),
        gen_storage_key_test(signed_config_positive),
        gen_storage_key_test(signed_config_negative),
    ]
}

/// Generate tests for FixedBytes types
fn gen_fixed_bytes_tests(size: usize) -> Vec<TokenStream> {
    let evm_test_name = quote::format_ident!("test_fixedbytes_{}_evm_words_roundtrip", size);
    let storage_test_name = quote::format_ident!("test_fixedbytes_{}_storage_key", size);

    let evm_test = quote! {
        #[test]
        fn #evm_test_name() {
            // Test edge cases
            let zero = ::alloy::primitives::FixedBytes::<#size>::ZERO;
            let words = zero.to_evm_words().expect("to_evm_words failed");
            let recovered = ::alloy::primitives::FixedBytes::<#size>::from_evm_words(words).expect("from_evm_words failed");
            assert_eq!(zero, recovered, "EVM words round-trip failed for zero");

            let max = ::alloy::primitives::FixedBytes::<#size>::from([0xFFu8; #size]);
            let words = max.to_evm_words().expect("to_evm_words failed");
            let recovered = ::alloy::primitives::FixedBytes::<#size>::from_evm_words(words).expect("from_evm_words failed");
            assert_eq!(max, recovered, "EVM words round-trip failed for max");

            // Test random values
            for _ in 0..100 {
                let value = ::alloy::primitives::FixedBytes::<#size>::random();
                let words = value.to_evm_words().expect("to_evm_words failed");
                let recovered = ::alloy::primitives::FixedBytes::<#size>::from_evm_words(words).expect("from_evm_words failed");
                assert_eq!(value, recovered, "EVM words round-trip failed for random value");
            }
        }
    };

    let storage_test = quote! {
        #[test]
        fn #storage_test_name() {
            // Test byte length
            let value = ::alloy::primitives::FixedBytes::<#size>::ZERO;
            let bytes = value.as_storage_bytes();
            assert_eq!(bytes.as_ref().len(), #size, "StorageKey byte length mismatch");

            // Test edge cases
            let zero = ::alloy::primitives::FixedBytes::<#size>::ZERO;
            let bytes = zero.as_storage_bytes();
            assert_eq!(bytes.as_ref().len(), #size);
            assert_eq!(bytes.as_ref(), zero.as_slice(), "StorageKey bytes mismatch for zero");

            let max = ::alloy::primitives::FixedBytes::<#size>::from([0xFFu8; #size]);
            let bytes = max.as_storage_bytes();
            assert_eq!(bytes.as_ref().len(), #size);
            assert_eq!(bytes.as_ref(), max.as_slice(), "StorageKey bytes mismatch for max");

            // Test random values
            for _ in 0..100 {
                let value = ::alloy::primitives::FixedBytes::<#size>::random();
                let bytes = value.as_storage_bytes();
                assert_eq!(bytes.as_ref().len(), #size);
                assert_eq!(bytes.as_ref(), value.as_slice(), "StorageKey bytes mismatch for random value");
            }
        }
    };

    vec![evm_test, storage_test]
}
