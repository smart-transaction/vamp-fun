use std::error::Error;
use std::sync::{Arc, Mutex};

use crate::snapshot_indexer::{SnapshotIndexer, TokenRequestData};
use crate::stats::{IndexerProcesses, VampingStatus};
use crate::use_proto::proto::UserEventProto;

use chrono::Utc;
use ethers::types::Address;
use ethers::utils::keccak256;
use log::info;
use sha3::{Digest, Keccak256};

pub struct DeployTokenHandler {
    pub indexer: Arc<SnapshotIndexer>,
    pub contract_address_name: [u8; 32],
    pub token_full_name: [u8; 32],
    pub token_symbol_name: [u8; 32],
    pub token_uri_name: [u8; 32],
    pub token_decimal_name: [u8; 32],
    pub stats: Arc<Mutex<IndexerProcesses>>
}

const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";
const TOKEN_FULL_NAME: &str = "TokenFullName";
const TOKEN_SYMBOL_NAME: &str = "TokenSymbolName";
const TOKEN_URI_NAME: &str = "TokenURI";
const TOKEN_DECIMAL_NAME: &str = "TokenDecimal";

impl DeployTokenHandler {
    pub fn new(indexer: Arc<SnapshotIndexer>, indexing_stats: Arc<Mutex<IndexerProcesses>>) -> Self {
        Self {
            indexer,
            contract_address_name: keccak256(CONTRACT_ADDRESS_NAME.as_bytes()),
            token_full_name: keccak256(TOKEN_FULL_NAME.as_bytes()),
            token_symbol_name: keccak256(TOKEN_SYMBOL_NAME.as_bytes()),
            token_uri_name: keccak256(TOKEN_URI_NAME.as_bytes()),
            token_decimal_name: keccak256(TOKEN_DECIMAL_NAME.as_bytes()),
            stats: indexing_stats,
        }
    }

    pub async fn handle(&self, sequence_id: u64, event: UserEventProto) -> Result<(), Box<dyn Error>> {
        info!("DeployTokenHandler triggered");
        let mut request_data = TokenRequestData::default();
        request_data.sequence_id = sequence_id;
        request_data.chain_id = event.chain_id;
        request_data.block_number = event.block_number;
        // Temporary random value for the intent_id
        let mut hash_message = Keccak256::new();
        hash_message.update(&sequence_id.to_le_bytes());
        hash_message.update(&event.chain_id.to_le_bytes());
        hash_message.update(&event.block_number.to_le_bytes());
        hash_message.update(Utc::now().timestamp().to_le_bytes());
        request_data.intent_id = hash_message.finalize().to_vec();

        for add_data in event.additional_data {
            if add_data.key == self.contract_address_name {
                request_data.erc20_address = Address::from_slice(&add_data.value);
            } else if add_data.key == self.token_full_name {
                request_data.token_full_name = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_symbol_name {
                request_data.token_symbol_name = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_uri_name {
                request_data.token_uri = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_decimal_name {
                if add_data.value.len() != 1 {
                    return Err("Invalid token decimal length".into());
                }
                info!("Token decimal: {:?}", add_data.value[0]);
                request_data.token_decimal = add_data.value[0];
            }
        }
        let stats = self.stats.clone();
        let chain_id = request_data.chain_id;
        let erc20_address = request_data.erc20_address;
        match self.indexer.index_snapshot(request_data, stats.clone()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Ok(mut stats) = stats.lock() {
                    if let Some(item) = stats.get_mut(&(chain_id, erc20_address)) {
                        item.status = VampingStatus::Failure;
                        item.message = err.to_string();
                    }
                }
                return Err(err);
            }
        }
    }
}
