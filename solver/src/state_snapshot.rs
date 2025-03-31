use std::collections::HashMap;

use crate::merkle_tree::{Leaf, MerkleTree};
use ethers::types::{Address, U256};
use tokio::sync::Mutex;

use crate::use_proto::proto::StateSnapshotProto;

pub struct StateSnapshot {
    merkle_tree: MerkleTree,
    leaf_index: HashMap<Address, usize>,
    guard: Mutex<()>,
}

impl StateSnapshot {
    pub fn new() -> Self {
        Self {
            merkle_tree: MerkleTree::new(&[]),
            leaf_index: HashMap::<Address, usize>::new(),
            guard: Mutex::new(()),
        }
    }

    pub fn from_event(event: StateSnapshotProto) -> Self {
        let mut leaves = Vec::new();
        let mut leaf_index = HashMap::new();
        for i in 0..event.accounts.len() {
            let account = &event.accounts[i];
            let amount = &event.amounts[i];

            let leaf = Leaf {
                account: Address::from_slice(account),
                amount: U256::from_little_endian(amount),
            };
            leaf_index.insert(leaf.account, i);
            leaves.push(leaf);
        }
        Self {
            merkle_tree: MerkleTree::new(&leaves),
            leaf_index,
            guard: Mutex::new(()),
        }
    }

    async fn generate_proof(&self, address: Address) -> Option<Vec<[u8; 32]>> {
        let _ = self.guard.lock().await;
        if let Some(index) = self.leaf_index.get(&address) {
            return Some(self.merkle_tree.generate_proof(*index));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers::types::H160;

    #[tokio::test]
    async fn test_new() {
        // Create mock StateSnapshotProto event
        let address1 = H160::random();
        let address2 = H160::random();
        let amount1 = U256::from(100);
        let amount2 = U256::from(200);

        let mut amount1_bytes = [0u8; 32];
        let mut amount2_bytes = [0u8; 32];
        amount1.to_little_endian(&mut amount1_bytes);
        amount2.to_little_endian(&mut amount2_bytes);

        let event = StateSnapshotProto {
            accounts: vec![address1.as_bytes().to_vec(), address2.as_bytes().to_vec()],
            amounts: vec![amount1_bytes.to_vec(), amount2_bytes.to_vec()],
        };

        // Create a new StateSnapshot
        let snapshot = StateSnapshot::from_event(event);

        // Verify the leaf_index is initialized correctly
        assert_eq!(snapshot.leaf_index.get(&address1), Some(&0));
        assert_eq!(snapshot.leaf_index.get(&address2), Some(&1));

        // Verify the Merkle tree is initialized with the correct leaves
        let proof1 = snapshot.generate_proof(address1).await;
        assert!(proof1.is_some());
        let proof2 = snapshot.generate_proof(address2).await;
        assert!(proof2.is_some());
    }

    #[tokio::test]
    async fn test_generate_proof() {
        // Create mock StateSnapshotProto event
        let address1 = H160::random();
        let address2 = H160::random();
        let amount1 = U256::from(100);
        let amount2 = U256::from(200);

        let mut amount1_bytes = [0u8; 32];
        let mut amount2_bytes = [0u8; 32];
        amount1.to_little_endian(&mut amount1_bytes);
        amount2.to_little_endian(&mut amount2_bytes);

        let event = StateSnapshotProto {
            accounts: vec![address1.as_bytes().to_vec(), address2.as_bytes().to_vec()],
            amounts: vec![amount1_bytes.to_vec(), amount2_bytes.to_vec()],
        };

        // Create a new StateSnapshot
        let snapshot = StateSnapshot::from_event(event);

        // Test proof generation for address1
        let proof = snapshot.generate_proof(address1).await;
        assert!(proof.is_some());
        assert!(!proof.unwrap().is_empty());

        // Test proof generation for an unknown address
        let unknown_address = H160::random();
        let proof = snapshot.generate_proof(unknown_address).await;
        assert!(proof.is_none());
    }
}
