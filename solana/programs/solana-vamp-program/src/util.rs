use anchor_lang::solana_program::keccak;
use crate::state::vamp_state::TokenMapping;

pub fn generate_merkle_root(mappings: &[TokenMapping]) -> [u8; 32] {
    // Step 1: Hash each TokenMapping to create the leaf nodes
    let mut leaves: Vec<[u8; 32]> = mappings
        .iter()
        .map(|entry| {
            let mut data = entry.token_address.to_bytes().to_vec();
            data.extend_from_slice(&entry.token_amount.to_le_bytes());
            keccak::hash(&data).0
        })
        .collect();

    // Step 2: Build the Merkle tree
    while leaves.len() > 1 {
        let mut next_level = Vec::new();
        for i in (0..leaves.len()).step_by(2) {
            let left = leaves[i];
            let right = if i + 1 < leaves.len() {
                leaves[i + 1]
            } else {
                // If there's an odd number of nodes, duplicate the last one
                leaves[i]
            };
            let mut combined = Vec::new();
            combined.extend_from_slice(&left);
            combined.extend_from_slice(&right);
            next_level.push(keccak::hash(&combined).0);
        }
        leaves = next_level;
    }

    // Step 3: Return the Merkle root
    leaves[0]
}

