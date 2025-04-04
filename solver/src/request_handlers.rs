use std::sync::Arc;

use crate::use_proto::proto::UserEventProto;
use crate::snapshot_indexer::SnapshotIndexer;
use ethers::types::Address;
use ethers::utils::keccak256;
use log::error;

pub struct DeployTokenHandler {
    pub indexer: Arc<SnapshotIndexer>,
    pub contract_address_name: [u8; 32],
}

impl DeployTokenHandler {
    pub fn new(indexer: Arc<SnapshotIndexer>) -> Self {
        Self {
            indexer,
            contract_address_name: keccak256(CONTRACT_ADDRESS_NAME.as_bytes()),
        }
    }

    pub async fn handle(&self, event: UserEventProto) {
        for add_data in event.additional_data {
            if add_data.key == self.contract_address_name {
                let erc20_address = Address::from_slice(&add_data.value);
                if let Err(err) = self
                    .indexer
                    .index_snapshot(event.chain_id, erc20_address, event.block_number)
                    .await
                {
                    error!("Failed to index snapshot: {:?}", err);
                }
                break;
            }
        }
    }
}

const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";
