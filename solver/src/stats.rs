use std::{
    cmp::max, collections::HashMap, sync::{Arc, Mutex}, time::Duration
};

use ethers::types::Address;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum VampingStatus {
    #[default]
    Starting,
    Indexing,
    SendingToSolana,
    Success,
    Failure,
}

pub type IndexerProcesses = HashMap<(u64, Address), IndexerStats>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IndexerStats {
    pub chain_id: u64,
    pub token_address: Address,
    pub start_block: u64,
    pub end_block: u64,
    pub blocks_done: u64,
    pub status: VampingStatus,
    pub message: String,
    pub start_timestamp: i64,
    pub current_timestamp: i64,
    pub solana_txid: String,
    pub cloning_intent_id: String,
}

const MAX_STATS: usize = 100;

pub async fn cleanup_stats(stats: Arc<Mutex<IndexerProcesses>>) {
    loop {
        sleep(Duration::from_secs(60)).await;
        if let Ok(mut stats) = stats.lock() {
            // Find oldest stats
            let num_to_remove = max(MAX_STATS, stats.len()) - MAX_STATS;
            if num_to_remove == 0 {
                continue;
            }
            let mut updated: Vec<_> = stats
                .values()
                .map(|v| (v.current_timestamp, v.chain_id, v.token_address))
                .collect();
            updated.sort_by_key(|v| (v.0));
            // Get first N items as oldest and remove them
            for remove_item in updated[0..num_to_remove].iter() {
                stats.remove(&(remove_item.1, remove_item.2));
            }
        }
    }
}
