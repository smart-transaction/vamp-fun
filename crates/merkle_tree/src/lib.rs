use sha3::{Digest, Keccak256};

/// Represents a leaf node (account, amount)
#[derive(Clone, Debug)]
pub struct Leaf {
    pub account: [u8; 20],
    pub amount: u64,
    pub decimals: u8,
}

impl Leaf {
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::with_capacity(52); // 20 + 32
        data.extend_from_slice(&self.account);

        let amount_bytes = self.amount.to_be_bytes();
        data.extend_from_slice(&amount_bytes);

        let decimals_bytes = self.decimals.to_be_bytes();
        data.extend_from_slice(&decimals_bytes);

        let mut hasher = Keccak256::new();
        hasher.update(&data);
        hasher.finalize().into()
    }
}

#[derive(Clone, Debug)]
pub struct MerkleTree {
    pub root: [u8; 32],
    pub levels: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn new(leaves: &[Leaf]) -> Self {
        if leaves.is_empty() {
            let root = [0u8; 32];
            return Self {
                root,
                levels: vec![vec![root]],
            };
        }
        let levels = Self::build(leaves);
        let root = levels.last().unwrap()[0];
        Self { root, levels }
    }

    /// Builds the Merkle tree and returns all levels (bottom to top)
    fn build(leaves: &[Leaf]) -> Vec<Vec<[u8; 32]>> {
        let mut current_level: Vec<[u8; 32]> = leaves.iter().map(|leaf| leaf.hash()).collect();

        // Pad to next power of two
        let size = current_level.len();
        let next_pow2 = size.next_power_of_two();
        if size < next_pow2 {
            let last = *current_level.last().unwrap();
            current_level.extend(std::iter::repeat(last).take(next_pow2 - size));
        }

        let mut tree: Vec<Vec<[u8; 32]>> = vec![current_level.clone()];

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for pair in current_level.chunks(2) {
                let combined = [pair[0], pair[1]].concat();
                let mut hasher = Keccak256::new();
                hasher.update(&combined);
                next_level.push(hasher.finalize().into());
            }
            tree.push(next_level.clone());
            current_level = next_level;
        }

        tree
    }

    /// Generates a Merkle proof for a leaf at the given index
    pub fn generate_proof(&self, mut index: usize) -> Vec<[u8; 32]> {
        let mut proof = Vec::new();

        for level in &self.levels[..&self.levels.len() - 1] {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };
            if sibling_index < level.len() {
                proof.push(level[sibling_index]);
            }
            index /= 2;
        }

        proof
    }

    /// Returns a number of tree levels
    pub fn levels(&self) -> usize {
        self.levels.len()
    }
}

/// Verifies a Merkle proof
pub fn verify_merkle_proof(
    leaf_hash: [u8; 32],
    proof: &[[u8; 32]],
    root: [u8; 32],
    mut index: usize,
) -> bool {
    let mut computed_hash = leaf_hash;

    for proof_element in proof {
        let combined = if index % 2 == 0 {
            [computed_hash, *proof_element].concat()
        } else {
            [*proof_element, computed_hash].concat()
        };
        let mut hasher = Keccak256::new();
        hasher.update(&combined);
        computed_hash = hasher.finalize().into();
        index /= 2;
    }

    computed_hash == root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new(&[]);
        assert_eq!(tree.root, [0u8; 32]);
        assert_eq!(tree.levels(), 1);
    }

    #[test]
    fn test_build_merkle_tree() {
        let leaf1 = Leaf {
            account: [1; 20],
            amount: 100,
            decimals: 9,
        };
        let leaf2 = Leaf {
            account: [2; 20],
            amount: 200,
            decimals: 9,
        };
        let leaf3 = Leaf {
            account: [3; 20],
            amount: 300,
            decimals: 9,
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let merkle_tree = MerkleTree::new(&leaves);

        // Check that the tree has the correct number of levels
        assert_eq!(merkle_tree.levels(), 3);

        // Check that the root is correctly computed
        let expected_root = [
            227, 26, 143, 61, 193, 137, 205, 61, 239, 153, 177, 0, 129, 106, 227, 198, 138, 63,
            162, 105, 243, 172, 52, 72, 115, 107, 61, 195, 95, 161, 3, 254,
        ];
        assert_eq!(merkle_tree.root, expected_root);
    }

    #[test]
    fn test_generate_merkle_proof() {
        let leaf1 = Leaf {
            account: [1; 20],
            amount: 100,
            decimals: 9,
        };
        let leaf2 = Leaf {
            account: [2; 20],
            amount: 200,
            decimals: 9,
        };
        let leaf3 = Leaf {
            account: [3; 20],
            amount: 300,
            decimals: 9,
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let tree = MerkleTree::new(&leaves);

        let proof = tree.generate_proof(1);

        // Check that the proof is correct
        let expected_proof = [
            [
                66, 105, 170, 88, 129, 252, 235, 50, 125, 110, 239, 176, 140, 183, 206, 19, 73, 16,
                241, 12, 109, 195, 126, 126, 145, 217, 165, 67, 200, 29, 140, 211,
            ],
            [
                238, 207, 180, 63, 241, 145, 225, 207, 4, 61, 182, 94, 187, 27, 211, 53, 70, 3, 44,
                20, 110, 80, 88, 232, 53, 187, 186, 96, 255, 143, 220, 98,
            ],
        ];
        assert_eq!(proof, expected_proof);
    }

    #[test]
    fn test_verify_merkle_proof() {
        let leaf1 = Leaf {
            account: [1; 20],
            amount: 100,
            decimals: 9,
        };
        let leaf2 = Leaf {
            account: [2; 20],
            amount: 200,
            decimals: 9,
        };
        let leaf3 = Leaf {
            account: [3; 20],
            amount: 300,
            decimals: 9,
        };

        let good_leaf_hash = Leaf {
            account: [2; 20],
            amount: 200,
            decimals: 9,
        }
        .hash();

        let bad_leaf_hash = Leaf {
            account: [2; 20],
            amount: 10,
            decimals: 9,
        }
        .hash();

        let leaves = vec![leaf1, leaf2, leaf3];
        let tree = MerkleTree::new(&leaves);

        let proof = tree.generate_proof(1);

        let verified = verify_merkle_proof(good_leaf_hash, &proof, tree.root, 1);
        assert!(verified);
        let verified = verify_merkle_proof(bad_leaf_hash, &proof, tree.root, 1);
        assert!(!verified);
    }
}
