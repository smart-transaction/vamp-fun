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
    SubmitSolutionRequestProto, TokenMappingProto, TokenVampingInfoProto,
    UserEventProto, orchestrator_service_client::OrchestratorServiceClient,
};

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    snapshot: HashMap<Address, U256>,
    orchestrator_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Received indexed snapshot");
    // Convert the amount into a Solana format
    let amount = amount.checked_div(U256::from(10u64.pow(9)))
        .ok_or("Failed to convert amount")?;
    let amount = amount.as_u64();
    let snapshot = snapshot
        .iter()
        .map(|(k, v)| {
            let amount = v.checked_div(U256::from(10u64.pow(9)))
                .ok_or("Failed to convert amount");
            (*k, amount.unwrap_or_default().as_u64())
        })
        .collect::<HashMap<_, _>>();
    // Create the Merkle tree
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
            .map(|(_, v)| v.to_le_bytes().to_vec())
            .collect(),
    };

    let token_vamping_info = TokenVampingInfoProto {
        merkle_root: root.to_vec(),
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_bytes().to_vec(),
        token_uri: Some(request_data.token_uri),
        amount,
        decimal: request_data.token_decimal as u32,
        token_mapping: Some(token_mapping),
    };

    let mut encoded_vamping_info = Vec::new();
    token_vamping_info.encode(&mut encoded_vamping_info)?;

    let request_proto = SubmitSolutionRequestProto {
        app_id: keccak256(VAMPING_APP_ID.as_bytes()).to_vec(),
        request_sequence_id: request_data.sequence_id,
        generic_solution: encoded_vamping_info.into(),
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
