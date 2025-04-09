use std::str::FromStr;

use ethers::{types::{Address, U256}, utils::parse_units};
use prost::Message;

pub mod proto {
    tonic::include_proto!("vamp.fun");
}

use proto::TokenMappingProto;

/*
        let amount: U256 = parse_units(200, "ether").unwrap().into();
        let gas: U256 = parse_units(1000000, "gwei").unwrap().into();
        let mut amount_bytes: [u8; 32] = [0; 32];
        amount.to_little_endian(&mut amount_bytes);
        let mut gas_bytes: [u8; 32] = [0; 32];
        gas.to_little_endian(&mut gas_bytes);

        let token_deploy_event = UserEventProto {
            app_id: keccak256("DeployToken".as_bytes()).to_vec(),
            chain_id: 84532,
            block_number: 24016254,
            user_objective: Some(UserObjectiveProto {
                app_id: keccak256("VampFunDeployToken".as_bytes()).to_vec(),
                nonse: 1,
                chain_id: 84532,
                call_objects: vec![CallObjectProto {
                    id: 1,
                    chain_id: 84532,
                    salt: keccak256("TheSalt".as_bytes()).to_vec(),
                    amount: amount_bytes.to_vec(),
                    gas: gas_bytes.to_vec(),
                    address: Address::from_str("0xF821AdA310c3c7DA23aBEa279bA5Bf22B359A7e1")
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                    skippable: true,
                    verifiable: true,
                    callvalue: vec![],
                    returnvalue: vec![],
                }],
            }),
            additional_data: vec![AdditionalDataProto {
                key: keccak256("ERC20ContractAddress".as_bytes()).to_vec(),
                value: Address::from_str("0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966")
                    .unwrap()
                    .as_bytes()
                    .to_vec(),
            }],
        };

        let mut poll_response = PollResponseProto::default();

        let mut token_deploy_buff = Vec::new();
        UserEventProto::encode(&token_deploy_event, &mut token_deploy_buff).unwrap();
*/
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

fn decode(encoded_val: Vec<u8>) -> TokenMappingProto {
    let decoded_proto = TokenMappingProto::decode(&encoded_val[..]).unwrap();
    decoded_proto
}

fn main() {
    let encoded_proto = encode();
    let decoded_proto = decode(encoded_proto.clone());
    println!("Decoded Proto: {:?}", decoded_proto);
}
