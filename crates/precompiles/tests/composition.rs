//! Tests for TIP20 abi module types (Calls, Error, Event).
//!
//! These tests verify the solidity-generated types work correctly without
//! using the #[contract] macro (which has path issues in integration tests).

use alloy::{
    primitives::{Address, B256, IntoLogData, U256},
    sol_types::{SolCall, SolInterface},
};
use tempo_precompiles::contracts::tip20::tip20;

#[test]
fn test_calls_enum_decode() {
    // Test Token calls
    let call = tip20::balanceOfCall {
        account: Address::random(),
    };
    let encoded = <tip20::ITokenCalls as SolInterface>::abi_encode(&call.into());

    let decoded = tip20::Calls::abi_decode(&encoded).unwrap();
    assert!(matches!(
        decoded,
        tip20::Calls::IToken(tip20::ITokenCalls::balanceOf(_))
    ));

    // Test RolesAuth calls
    let call = tip20::hasRoleCall {
        role: B256::random(),
        account: Address::random(),
    };
    let encoded = <tip20::IRolesAuthCalls as SolInterface>::abi_encode(&call.into());

    let decoded = tip20::Calls::abi_decode(&encoded).unwrap();
    assert!(matches!(
        decoded,
        tip20::Calls::IRolesAuth(tip20::IRolesAuthCalls::hasRole(_))
    ));
}

#[test]
fn test_calls_selectors() {
    assert!(!tip20::Calls::SELECTORS.is_empty());

    // Verify all selectors are valid
    for selector in tip20::Calls::SELECTORS {
        assert!(tip20::Calls::valid_selector(*selector));
    }

    // Check specific selectors
    assert!(tip20::Calls::valid_selector(tip20::balanceOfCall::SELECTOR));
    assert!(tip20::Calls::valid_selector(tip20::hasRoleCall::SELECTOR));
    assert!(tip20::Calls::valid_selector(
        tip20::distributeRewardCall::SELECTOR
    ));
}

#[test]
fn test_error_constructors() {
    let err =
        tip20::Error::insufficient_balance(U256::from(100), U256::from(200), Address::random());
    assert!(matches!(err, tip20::Error::InsufficientBalance(_)));

    let err = tip20::Error::unauthorized();
    assert!(matches!(err, tip20::Error::Unauthorized(_)));
}

#[test]
fn test_event_constructors() {
    let event = tip20::Event::transfer(Address::random(), Address::random(), U256::from(100));
    assert!(matches!(event, tip20::Event::Transfer(_)));
    let log_data = event.into_log_data();
    assert!(!log_data.topics().is_empty());

    let event = tip20::Event::role_membership_updated(
        B256::random(),
        Address::random(),
        Address::random(),
        true,
    );
    assert!(matches!(event, tip20::Event::RoleMembershipUpdated(_)));
}

#[test]
fn test_unknown_selector_returns_error() {
    let unknown_calldata = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x00, 0x00, 0x00];
    let result = tip20::Calls::abi_decode(&unknown_calldata);
    assert!(result.is_err());
}

#[test]
fn test_sol_call_trait_methods() {
    let call = tip20::balanceOfCall {
        account: Address::random(),
    };

    // Test SolCall trait via associated items
    assert_eq!(
        <tip20::balanceOfCall as SolCall>::SELECTOR,
        tip20::balanceOfCall::SELECTOR
    );
    assert!(<tip20::balanceOfCall as SolCall>::abi_encoded_size(&call) > 0);
}
