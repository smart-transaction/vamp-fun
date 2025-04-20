use std::error::Error;

use merkle_tree::{Leaf, MerkleTree};

use crate::{state::vamp_state::TokenMapping, use_proto::vamp_fun::TokenMappingProto};

pub fn verify_merkle_root(
    token_mapping: &TokenMappingProto,
    decimals: u8,
    merkle_root: &[u8; 32],
) -> Result<bool, Box<dyn Error>> {
    let mut leaves = Vec::new();
    for i in 0..token_mapping.addresses.len() {
        let leaf = Leaf {
            account: token_mapping.addresses[i].as_slice().try_into()?,
            amount: token_mapping.amounts[i],
            decimals,
        };
        leaves.push(leaf);
    }
    let merkle_tree = MerkleTree::new(&leaves);
    Ok(merkle_tree.root == *merkle_root)
}

pub fn convert_token_mapping(
    token_mapping_proto: &TokenMappingProto,
    decimals: u8,
) -> Result<Vec<TokenMapping>, Box<dyn Error>> {
    let mut token_mappings = Vec::new();
    for i in 0..token_mapping_proto.addresses.len() {
        let mut token_mapping = TokenMapping::default();
        token_mapping.eth_address = token_mapping_proto.addresses[i].as_slice().try_into()?;
        token_mapping.token_amount = token_mapping_proto.amounts[i];
        token_mapping.decimals = decimals;
        token_mappings.push(token_mapping);
    }
    Ok(token_mappings)
}
