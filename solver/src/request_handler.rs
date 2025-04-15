use std::error::Error;
use std::sync::Arc;

use crate::snapshot_indexer::{SnapshotIndexer, TokenRequestData};
use crate::use_proto::proto::UserEventProto;
use ethers::types::Address;
use ethers::utils::keccak256;
use log::info;

pub struct DeployTokenHandler {
    pub indexer: Arc<SnapshotIndexer>,
    pub contract_address_name: [u8; 32],
    pub token_full_name: [u8; 32],
    pub token_symbol_name: [u8; 32],
    pub token_uri_name: [u8; 32],
    pub token_decimal_name: [u8; 32],
}

const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";
const TOKEN_FULL_NAME: &str = "TokenFullName";
const TOKEN_SYMBOL_NAME: &str = "TokenSymbolName";
const TOKEN_URI_NAME: &str = "TokenURI";
const TOKEN_DECIMAL_NAME: &str = "TokenDecimal";

impl DeployTokenHandler {
    pub fn new(indexer: Arc<SnapshotIndexer>) -> Self {
        Self {
            indexer,
            contract_address_name: keccak256(CONTRACT_ADDRESS_NAME.as_bytes()),
            token_full_name: keccak256(TOKEN_FULL_NAME.as_bytes()),
            token_symbol_name: keccak256(TOKEN_SYMBOL_NAME.as_bytes()),
            token_uri_name: keccak256(TOKEN_URI_NAME.as_bytes()),
            token_decimal_name: keccak256(TOKEN_DECIMAL_NAME.as_bytes()),
        }
    }

    pub async fn handle(&self, sequence_id: u64, event: UserEventProto) -> Result<(), Box<dyn Error>> {
        info!("DeployTokenHandler triggered");
        let mut request_data = TokenRequestData::default();
        request_data.sequence_id = sequence_id;
        request_data.chain_id = event.chain_id;
        request_data.block_number = event.block_number;
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
        self.indexer.index_snapshot(request_data).await?;

        Ok(())
    }
}
