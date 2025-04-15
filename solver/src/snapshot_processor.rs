use std::collections::HashMap;

use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use log::info;
use prost::Message;

use crate::merkle_tree::{Leaf, MerkleTree};
use crate::request_registrator_listener::VAMPING_APP_ID;
use crate::snapshot_indexer::TokenRequestData;
use crate::use_proto::proto::AppChainResultStatus;
use crate::use_proto::proto::{
    AdditionalDataProto, SolverDecisionRequestProto, TokenMappingProto, TokenVampingInfoProto,
    UserEventProto, orchestrator_service_client::OrchestratorServiceClient,
};

const TOKEN_VAMPING_INFO_NAME: &str = "TokenVampingInfo";

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    snapshot: HashMap<Address, U256>,
    orchestrator_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Received indexed snapshot");
    let leaves = snapshot
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
    let root = merkle_tree.root;

    let mut user_event = UserEventProto::default();
    user_event.app_id = keccak256(VAMPING_APP_ID.as_bytes()).to_vec();

    let token_mapping = TokenMappingProto {
        addresses: snapshot
            .iter()
            .map(|(k, _)| k.as_bytes().to_vec())
            .collect(),
        amounts: snapshot
            .iter()
            .map(|(_, v)| {
                let mut amount_bytes = [0; 32];
                v.to_little_endian(&mut amount_bytes);
                amount_bytes.to_vec()
            })
            .collect(),
    };

    let mut amount_bytes = [0; 32];
    amount.to_little_endian(&mut amount_bytes);

    let token_vamping_info = TokenVampingInfoProto {
        merkle_root: root.to_vec(),
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_bytes().to_vec(),
        token_uri: Some(request_data.token_uri),
        amount: amount_bytes.to_vec(),
        decimal: request_data.token_decimal as u32,
        token_mapping: Some(token_mapping),
    };

    let mut encoded_vamping_info: Vec<u8> = Vec::new();
    token_vamping_info.encode(&mut encoded_vamping_info)?;
    user_event.additional_data.push(AdditionalDataProto {
        key: keccak256(TOKEN_VAMPING_INFO_NAME.as_bytes()).to_vec(),
        value: encoded_vamping_info,
    });

    let request_proto = SolverDecisionRequestProto {
        app_id: keccak256(VAMPING_APP_ID.as_bytes()).to_vec(),
        sequence_id: request_data.sequence_id,
        event: Some(user_event),
    };

    let mut client = OrchestratorServiceClient::connect(orchestrator_url.clone()).await?;
    info!("Connected to orchestrator at {}", orchestrator_url);
    let response = client.solver_decision(request_proto).await?;
    let response_proto = response.into_inner();
    if let Some(result) = response_proto.result.to_owned() {
        if result.status == AppChainResultStatus::Error as i32 {
            if let Some(message) = result.message {
                return Err(format!("Error in orchestrator response: {}", message).into());
            } else {
                return Err("Error in orchestrator response: Unknown error".into());
            }
        } else {
            info!("The solver decision is successfully sent to the orchestrator.");
        }
    }

    Ok(())
}
