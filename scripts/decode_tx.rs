#!/usr/bin/env -S cargo +nightly -Zscript
//! Decode and compare two Tempo transactions

use alloy_primitives::hex;
use alloy_eips::eip2718::Decodable2718;
use alloy_consensus::Transaction;

fn main() {
    let tx1_hex = "76f8928210798545d964b800855d21dba00082c350d8d79400000000000000000000000000000000000000008080c0880998dc9109b9dcba8080809420c000000000000000000000033abb6ac7d235e580c0b84197697636ff5d135953ba256e32f87319517ced3146aafb4080d5ea86bbf3db2242f756cc7932535c4684900412a7f11574f1e96237bb3e12118e2b5251d3b89e1c";
    let tx2_hex = "76f8928210798545d964b800855d21dba000825208d8d79400000000000000000000000000000000000000008080c0880d2e7c280b4020f68080809420c000000000000000000000033abb6ac7d235e580c0b841a7b7773cd0f6bbd42d35044614dde705d8cf30d7103133da40a2c5dc5dd595860a01955ca2383b5394efece4188fc2ca94c146a1c613264c7c11874026dc33161b";
    
    println!("TX1 (passes): gas_limit field at bytes 22-24");
    println!("TX2 (fails): gas_limit field at bytes 22-24");
    
    // The key difference is at position after max_fee_per_gas
    // Let's look at the hex difference
    println!("\nTX1: {}", tx1_hex);
    println!("TX2: {}", tx2_hex);
    
    // Find first difference
    let b1 = hex::decode(tx1_hex).unwrap();
    let b2 = hex::decode(tx2_hex).unwrap();
    
    for (i, (a, b)) in b1.iter().zip(b2.iter()).enumerate() {
        if a != b {
            println!("\nFirst diff at byte {}: 0x{:02x} vs 0x{:02x}", i, a, b);
            println!("Context TX1: {:02x?}", &b1[i.saturating_sub(5)..i+10.min(b1.len())]);
            println!("Context TX2: {:02x?}", &b2[i.saturating_sub(5)..i+10.min(b2.len())]);
            break;
        }
    }
}
