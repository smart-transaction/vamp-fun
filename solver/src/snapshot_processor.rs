use std::collections::HashMap;
use std::error::Error;

use chrono::Utc;
use ethers::types::{Address, U256};
use ethers::utils::keccak256;
use log::info;
use merkle_tree::{Leaf, MerkleTree};
use prost::Message;

use crate::request_registrator_listener::VAMPING_APP_ID;
use crate::snapshot_indexer::TokenRequestData;
use crate::use_proto::proto::AppChainResultStatus;
use crate::use_proto::proto::{
    SubmitSolutionRequestProto, TokenMappingProto, TokenVampingInfoProto, UserEventProto,
    orchestrator_service_client::OrchestratorServiceClient,
};

fn convert_to_sol(src_amount: U256) -> Result<(u64, u8), Box<dyn Error>> {
    // Truncate the amount to gwei
    let amount = src_amount
        .checked_div(U256::from(10u64.pow(9)))
        .ok_or("Failed to divide amount")?;
    // Further truncating until the value fits u64
    // Setting it to zero right now, as we are fixed on decimals = 9.
    // Will be set to 9 later when we can customize decimals On Solana 
    let max_extra_decimals = 0u8;
    for decimals in 0..=max_extra_decimals {
        let trunc_amount = amount
            .checked_div(U256::from(10u64.pow(decimals as u32)))
            .ok_or("Failed to divide amount")?;
        // Check that we are not losing precision
        if trunc_amount
            .checked_mul(U256::from(10u64.pow(decimals as u32)))
            .ok_or("Failed to multiply amount")?
            != amount
        {
            return Err(format!(
                "The amount {:?} is too large to be minted on Solana",
                amount
            )
            .into());
        }
        let max_amount = U256::from(u64::MAX);
        if trunc_amount <= max_amount {
            return Ok((trunc_amount.as_u64(), 9u8 - decimals));
        }
    }
    Err(format!(
        "The amount {:?} is too large to be minted on Solana",
        amount
    )
    .into())
}

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    snapshot: HashMap<Address, U256>,
    orchestrator_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Received indexed snapshot");
    // Convert the amount into a Solana format
    let (amount, decimals) = convert_to_sol(amount)?;
    let snapshot = snapshot
        .iter()
        .map(|(k, v)| {
            let amount = v.checked_div(U256::from(10u64.pow(18 - decimals as u32)));
            (*k, amount.unwrap_or_default().as_u64())
        })
        .collect::<HashMap<_, _>>();
    // Create the Merkle tree
    let leaves = snapshot
        .iter()
        .map(|(k, v)| {
            let leaf = Leaf {
                account: k.to_fixed_bytes(),
                amount: *v,
                decimals,
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
        amounts: snapshot.iter().map(|(_, v)| *v).collect(),
    };

    let salt = Utc::now().timestamp() as u64;

    let token_vamping_info = TokenVampingInfoProto {
        merkle_root: root.to_vec(),
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_bytes().to_vec(),
        token_uri: Some(request_data.token_uri),
        amount,
        decimal: decimals as u32,
        token_mapping: Some(token_mapping),
        chain_id: request_data.chain_id,
        salt,
    };

    let mut encoded_vamping_info = Vec::new();
    token_vamping_info.encode(&mut encoded_vamping_info)?;

    let request_proto = SubmitSolutionRequestProto {
        app_id: keccak256(VAMPING_APP_ID.as_bytes()).to_vec(),
        request_sequence_id: request_data.sequence_id,
        generic_solution: encoded_vamping_info.into(),
        chain_id: request_data.chain_id,
        token_ers20_address: request_data.erc20_address.as_bytes().to_vec(),
        salt,
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

#[test]
fn test_convert_to_sol() {
    // Test case: Valid conversion
    let amount = U256::from(1_000_000_000_000_000_000u128);
    let result = convert_to_sol(amount).unwrap();
    assert_eq!(result, (1000000000, 9));

    // Test case: Large amount that fits into u64
    let amount = U256::from(10_000_000_000_000_000_000u128);
    let result = convert_to_sol(amount).unwrap();
    assert_eq!(result, (10000000000, 9));

    // Test case: Small amount conversion
    let amount = U256::from(123);
    let result = convert_to_sol(amount).unwrap();
    assert_eq!(result, (0, 9));

    // Test case: Maximum allowed amount
    let amount = U256::from(u64::MAX as u128 * 10u128.pow(9));
    let result = convert_to_sol(amount).unwrap();
    assert_eq!(result, (u64::MAX, 9));

    // Test case: Amount too large to fit into u64
    let amount = U256::from(u128::MAX);
    let result = convert_to_sol(amount);
    assert!(result.is_err());

    // Test case: Another too large amount
    let amount = U256::from((u64::MAX as u128 + 1) * 10u128.pow(9));
    let result = convert_to_sol(amount);
    assert!(result.is_err());

    // Test case: Zero amount
    let amount = U256::zero();
    let result = convert_to_sol(amount).unwrap();
    assert_eq!(result, (0, 9));
}
