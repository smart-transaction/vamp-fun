use std::str::FromStr;

use ethers::types::H160;
use merkle_tree::{Leaf, MerkleTree};
use prost::Message;

pub mod vamp_fun {
    include!(concat!(env!("OUT_DIR"), "/vamp.fun.rs"));
}

use vamp_fun::TokenVampingInfoProto;

fn prepare_vamping_info_sample() -> Vec<u8> {
    let mut result = Vec::new();

    let mut vamping_info = TokenVampingInfoProto::default();
    vamping_info.token_name = "Vamping Token".to_string();
    vamping_info.token_symbol = "VAMP".to_string();
    vamping_info.token_erc20_address = H160::from_str("0xb69A656b2Be8aa0b3859B24eed3c22dB206Ee966")
        .unwrap()
        .to_fixed_bytes()
        .to_vec();
    vamping_info.token_uri = Some("https://example.com/token/1".to_string());
    vamping_info.decimal = 9;

    // Merkle tree computation
    let leaves = vec![
        Leaf {
            account: H160::from_str("0xc3913d4D8bAb4914328651C2EAE817C8b78E1f4c")
                .unwrap()
                .to_fixed_bytes(),
            amount: 1000000000,
            decimals: 9,
        },
        Leaf {
            account: H160::from_str("0x65D08a056c17Ae13370565B04cF77D2AfA1cB9FA")
                .unwrap()
                .to_fixed_bytes(),
            amount: 2000000000,
            decimals: 9,
        },
        Leaf {
            account: H160::from_str("0x5918b2e647464d4743601a865753e64C8059Dc4F")
                .unwrap()
                .to_fixed_bytes(),
            amount: 3000000000,
            decimals: 9,
        },
        Leaf {
            account: H160::from_str("0xF5504cE2BcC52614F121aff9b93b2001d92715CA")
                .unwrap()
                .to_fixed_bytes(),
            amount: 4000000000,
            decimals: 9,
        },
        Leaf {
            account: H160::from_str("0xfDCe42116f541fc8f7b0776e2B30832bD5621C85")
                .unwrap()
                .to_fixed_bytes(),
            amount: 5000000000,
            decimals: 9,
        },
    ];

    let merkle_tree = MerkleTree::new(&leaves);
    vamping_info.merkle_root = merkle_tree.root.to_vec();
    vamping_info.amount = 15000000000;
    vamping_info.token_mapping = Some(vamp_fun::TokenMappingProto {
        addresses: leaves.iter().map(|leaf| leaf.account.to_vec()).collect(),
        amounts: leaves.iter().map(|leaf| leaf.amount).collect(),
    });

    TokenVampingInfoProto::encode(&vamping_info, &mut result).unwrap();

    result
}

fn main() {
    let encoded_sample = prepare_vamping_info_sample();
    println!("Encoded sample: {:?}", encoded_sample);
}
