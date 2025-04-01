use std::sync::Arc;

use crate::{appchain_listener::Handler, snapshot_indexer::SnapshotIndexer};
use crate::use_proto::proto::UserEventProto;
use ethers::types::Address;
use ethers::utils::keccak256;
use log::info;

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
}

const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";

impl Handler<UserEventProto> for DeployTokenHandler {
    async fn handle(&mut self, event: UserEventProto) {
        info!("Received StateSnapshot: {:?}", event);
        for add_data in event.additional_data {
            if add_data.key == self.contract_address_name {
                let contract_address = Address::from_slice(&add_data.value);
                self.indexer.index_snapshot(contract_address, event.block_number).await;
                break;
            }
        }
    }
}
