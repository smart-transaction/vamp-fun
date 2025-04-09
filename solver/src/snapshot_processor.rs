use std::{collections::HashMap, sync::Arc};

use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use log::info;
use prost::Message;
use tokio::sync::{Mutex, mpsc::Receiver};

use crate::merkle_tree::{Leaf, MerkleTree};
use crate::use_proto::proto::{AdditionalDataProto, TokenMappingProto, UserEventProto};

const TOKEN_MAPPING_NAME: &str = "TokenMapping";
const MERKLE_ROOT_NAME: &str = "MerkleRoot";
const TOKEN_METADATA_NAME: &str = "TokenMetadata";

pub struct Snapshot {
    pub merkle_tree: MerkleTree,
}

pub async fn listen_indexed_snapshot(
    mut rx: Receiver<HashMap<Address, U256>>,
    snapshot: Arc<Mutex<Snapshot>>,
) {
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
        let root = snapshot.merkle_tree.root;

        let mut user_event = UserEventProto::default();
        user_event.app_id = keccak256(TOKEN_MAPPING_NAME.as_bytes()).to_vec();
        user_event.additional_data.push(AdditionalDataProto {
            key: keccak256(MERKLE_ROOT_NAME.as_bytes()).to_vec(),
            value: root.to_vec(),
        });
        user_event.additional_data.push(AdditionalDataProto {
            key: keccak256(TOKEN_METADATA_NAME.as_bytes()).to_vec(),
            value: vec![],
        });

        let token_mapping = TokenMappingProto {
            addresses: message.iter().map(|(k, _)| k.as_bytes().to_vec()).collect(),
            amounts: message
                .iter()
                .map(|(_, v)| {
                    let mut amount_bytes = [0; 32];
                    v.to_little_endian(&mut amount_bytes);
                    amount_bytes.to_vec()
                })
                .collect(),
        };
        let mut encoded_token_mapping: Vec<u8> = Vec::new();
        if let Ok(_) = token_mapping.encode(&mut encoded_token_mapping) {
            user_event.additional_data.push(AdditionalDataProto {
                key: keccak256(TOKEN_MAPPING_NAME.as_bytes()).to_vec(),
                value: encoded_token_mapping,
            });
        }
    }
}
