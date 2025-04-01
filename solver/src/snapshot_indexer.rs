use ethers::types::Address;
use log::info;

pub struct SnapshotIndexer {
}

impl SnapshotIndexer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn index_snapshot(&self, token_address: Address, block_number: u64) {
      info!("Indexing snapshot for token address: {:?} at block number: {:?}", token_address, block_number);
    }
}