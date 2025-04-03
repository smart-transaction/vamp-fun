use std::{collections::HashMap, sync::Arc};

use ethers::types::{Address, U256};
use log::info;
use tokio::sync::{mpsc::Receiver, Mutex};

use crate::merkle_tree::{Leaf, MerkleTree};

pub struct Snapshot {
    pub merkle_tree: MerkleTree,
}

pub async fn listen_indexed_snapshot(mut rx: Receiver<HashMap<Address, U256>>, snapshot: Arc<Mutex<Snapshot>>) {
    while let Some(message) = rx.recv().await {
        info!("Received indexed snapshot");
        let leaves = message
            .iter()
            .map(|(k, v)| {
                let leaf = Leaf {
                    account: *k,
                    amount: *v,
                };
                leaf
            })
            .collect::<Vec<_>>();
        let merkle_tree = MerkleTree::new(&leaves);
        let mut snapshot = snapshot.lock().await;
        snapshot.merkle_tree = merkle_tree;
    }
}
