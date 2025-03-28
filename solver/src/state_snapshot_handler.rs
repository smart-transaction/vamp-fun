use ethers::types::{Address, U256};
use crate::appchain_listener::Handler;
use crate::merkle_tree::{Leaf, MerkleTree};

pub mod proto {
    tonic::include_proto!("vamp.fun");
}

pub use proto::StateSnapshot;

pub struct StateSnapshotHandler {
    merkle_tree: MerkleTree,
}

impl StateSnapshotHandler {
    pub fn new() -> Self {
        Self {
            merkle_tree: MerkleTree::new(&[]),
        }
    }
}

impl Handler<StateSnapshot> for StateSnapshotHandler {
    fn handle(&mut self, event: StateSnapshot) {
        let mut leaves = Vec::new();
        for i in 0..event.accounts.len() {
            let account = &event.accounts[i];
            let amount = &event.amounts[i];

            let leaf = Leaf {
                account: Address::from_slice(account),
                amount: U256::from_little_endian(amount),
            };
            leaves.push(leaf);
        }
        self.merkle_tree = MerkleTree::new(&leaves);
    }
}
