use ethers::types::{H160, U256};
use sha3::{Digest, Keccak256};

/// Represents a leaf node (account, amount)
#[derive(Clone, Debug)]
pub struct Leaf {
    pub account: H160,
    pub amount: U256,
}

impl Leaf {
    pub fn hash(&self) -> [u8; 32] {
        let mut data = Vec::with_capacity(52); // 20 + 32
        data.extend_from_slice(self.account.as_bytes());

        let mut amount_bytes = [0u8; 32];
        self.amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        let mut hasher = Keccak256::new();
        hasher.update(&data);
        hasher.finalize().into()
    }
}

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
            amount: U256::from(100),
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: U256::from(200),
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: U256::from(300),
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let merkle_tree = MerkleTree::new(&leaves);

        // Check that the tree has the correct number of levels
        assert_eq!(merkle_tree.levels(), 3);

        // Check that the root is correctly computed
        let expected_root = [
            188, 217, 76, 171, 134, 201, 250, 109, 171, 82, 228, 204, 109, 51, 45, 186, 82, 241,
            214, 128, 140, 4, 109, 192, 32, 17, 150, 91, 243, 42, 184, 19,
        ];
        assert_eq!(merkle_tree.root, expected_root);
    }

    #[test]
    fn test_generate_merkle_proof() {
        let leaf1 = Leaf {
            account: H160::from_low_u64_be(1),
            amount: U256::from(100),
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: U256::from(200),
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: U256::from(300),
        };

        let leaves = vec![leaf1, leaf2, leaf3];
        let tree = MerkleTree::new(&leaves);

        let proof = tree.generate_proof(1);

        // Check that the proof is correct
        let expected_proof = [
            [
                234, 244, 241, 120, 25, 175, 58, 155, 20, 189, 161, 246, 201, 27, 209, 204, 198,
                61, 194, 73, 51, 236, 105, 102, 117, 106, 154, 1, 208, 76, 81, 112,
            ],
            [
                79, 139, 196, 143, 10, 254, 77, 142, 183, 149, 221, 42, 180, 128, 242, 211, 227,
                115, 224, 78, 74, 240, 57, 175, 81, 141, 32, 245, 138, 41, 156, 140,
            ],
        ];
        assert_eq!(proof, expected_proof);
    }

    #[test]
    fn test_verify_merkle_proof() {
        let leaf1 = Leaf {
            account: H160::from_low_u64_be(1),
            amount: U256::from(100),
        };
        let leaf2 = Leaf {
            account: H160::from_low_u64_be(2),
            amount: U256::from(200),
        };
        let leaf3 = Leaf {
            account: H160::from_low_u64_be(3),
            amount: U256::from(300),
        };

        let good_leaf_hash = Leaf {
            account: H160::from_low_u64_be(2),
            amount: U256::from(200),
        }
        .hash();

        let bad_leaf_hash = Leaf {
            account: H160::from_low_u64_be(2),
            amount: U256::from(10),
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
