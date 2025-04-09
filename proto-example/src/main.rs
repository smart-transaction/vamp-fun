use std::str::FromStr;

use ethers::{types::{Address, H160, U256}, utils::parse_units};
use prost::Message;

pub mod vamp_fun {
    include!(concat!(env!("OUT_DIR"), "/vamp.fun.rs"));
}

use vamp_fun::TokenMappingProto;

fn encode() -> Vec<u8> {
    let addresses = vec![
        Address::from_str("0xF821AdA310c3c7DA23aBEa279bA5Bf22B359A7e1").unwrap().as_bytes().to_vec(),
        Address::from_str("0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966").unwrap().as_bytes().to_vec(),
    ];
    let amount1: U256 = parse_units(200, "ether").unwrap().into();
    let amount2: U256 = parse_units(200000, "ether").unwrap().into();
    let mut amount1_bytes = [0; 32];
    let mut amount2_bytes = [0; 32];
    amount1.to_little_endian(&mut amount1_bytes);
    amount2.to_little_endian(&mut amount2_bytes);
    let amounts = vec![
        amount1_bytes.to_vec(),
        amount2_bytes.to_vec(),
    ];

    let encoded_proto = TokenMappingProto {
        addresses: addresses.clone(),
        amounts: amounts.clone(),
    };

    let mut encoded_val = Vec::new();
    TokenMappingProto::encode(&encoded_proto, &mut encoded_val).unwrap();
    encoded_val
}

fn decode(encoded_val: Vec<u8>) -> (Vec<H160>, Vec<U256>) {
    let decoded_proto = TokenMappingProto::decode(&encoded_val[..]).unwrap();

    let decoded_addresses: Vec<H160> = decoded_proto.addresses.iter()
        .map(|address| Address::from_slice(address))
        .collect();
    let decoded_amounts: Vec<U256> = decoded_proto.amounts.iter()
        .map(|amount| {
            let mut amount_bytes = [0; 32];
            amount_bytes.copy_from_slice(amount);
            U256::from_little_endian(&amount_bytes)
        })
        .collect();

    (decoded_addresses, decoded_amounts)
}

fn main() {
    let encoded_proto = encode();
    let decoded = decode(encoded_proto.clone());
    println!("Decoded Proto: {:#?}", decoded);
}
