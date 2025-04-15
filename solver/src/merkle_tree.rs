use ethers::types::H160;
use sha3::{Digest, Keccak256};

/// Represents a leaf node (account, amount)
#[derive(Clone, Debug)]
pub struct Leaf {
    pub account: H160,
    pub amount: u64,
}

impl Leaf {
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::with_capacity(52); // 20 + 32
        data.extend_from_slice(self.account.as_bytes());

        let amount_bytes = self.amount.to_be_bytes();
        data.extend_from_slice(&amount_bytes);

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
            account: H160::from_low_u64_be(1),
            amount: 100,
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: 200,
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: 300,
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let merkle_tree = MerkleTree::new(&leaves);

        // Check that the tree has the correct number of levels
        assert_eq!(merkle_tree.levels(), 3);

        // Check that the root is correctly computed
        let expected_root = [
            102, 66, 129, 231, 73, 52, 99, 129, 227, 45, 201, 117, 83, 234, 11, 91, 18, 158, 74,
            79, 99, 185, 172, 206, 71, 18, 20, 110, 6, 118, 37, 8,
        ];
        assert_eq!(merkle_tree.root, expected_root);
    }

    #[test]
    fn test_generate_merkle_proof() {
        let leaf1 = Leaf {
            account: H160::from_low_u64_be(1),
            amount: 100,
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: 200,
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: 300,
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let tree = MerkleTree::new(&leaves);

        let proof = tree.generate_proof(1);

        // Check that the proof is correct
        let expected_proof = [
            [
                143, 126, 3, 158, 232, 56, 110, 102, 35, 24, 82, 188, 79, 181, 174, 193, 224, 251,
                248, 136, 5, 246, 249, 192, 20, 154, 0, 171, 183, 234, 236, 13,
            ],
            [
                179, 106, 116, 204, 124, 194, 209, 39, 62, 132, 254, 174, 85, 210, 239, 201, 217,
                46, 201, 154, 60, 151, 162, 189, 127, 180, 112, 118, 186, 210, 207, 217,
            ],
        ];
        assert_eq!(proof, expected_proof);
    }

    #[test]
    fn test_verify_merkle_proof() {
        let leaf1 = Leaf {
            account: H160::from_low_u64_be(1),
            amount: 100,
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: 200,
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: 300,
        };

        let good_leaf_hash = Leaf {
            account: H160::from_low_u64_be(2),
            amount: 200,
        }
        .hash();

        let bad_leaf_hash = Leaf {
            account: H160::from_low_u64_be(2),
            amount: 10,
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
