use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use bs58;
use chrono::Utc;
use ethers::utils::keccak256;
use ethers::{
    signers::{LocalWallet, Signer},
    types::{Address, Signature, U256},
};
use log::info;
use merkle_tree::{Leaf, MerkleTree};
use mysql::TxOpts;
use mysql::prelude::Queryable;
use prost::Message;
use sha3::{Digest, Keccak256};

use crate::mysql_conn::DbConn;
use crate::request_registrator_listener::VAMPING_APP_ID;
use crate::snapshot_indexer::{TokenAmount, TokenRequestData};
use crate::stats::{IndexerProcesses, VampingStatus};
use crate::use_proto::proto::{AppChainPayloadProto, AppChainResultStatus};
use crate::use_proto::proto::{
    SubmitSolutionRequestProto, TokenMappingProto, TokenVampingInfoProto, UserEventProto,
    orchestrator_service_client::OrchestratorServiceClient,
};

fn create_cloning_solana_intent_id(txid: &str, chain_id: u64) -> Result<Vec<u8>, Box<dyn Error>> {
    if txid.is_empty() {
        return Err("Transaction ID is empty".into());
    }
    let mut hasher = Keccak256::new();
    hasher.update(bs58::decode(txid).into_vec()?);
    hasher.update(&chain_id.to_le_bytes());
    let result = hasher.finalize();
    Ok(result.to_vec())
}

async fn sign_balance(
    chain_id: u64,
    private_key: &LocalWallet,
    address: &Address,
    supply: &mut TokenAmount,
    payload: &AppChainPayloadProto,
) -> Result<Signature, Box<dyn Error>> {
    let mut hash_message = Keccak256::new();
    hash_message.update(address.as_bytes());
    let (amount, _) = convert_to_sol(supply.amount)?;
    hash_message.update(&amount.to_le_bytes());
    hash_message.update(&create_cloning_solana_intent_id(
        &payload.solana_txid,
        chain_id,
    )?);
    let hash_message = hash_message.finalize();
    let signature = private_key.sign_message(hash_message).await?;
    Ok(signature)
}

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

fn write_cloning(
    db_conn: DbConn,
    chain_id: u64,
    erc20_address: Address,
    target_txid: &str,
) -> Result<(), Box<dyn Error>> {
    let mut conn = db_conn.create_db_conn()?;
    let addr_str = format!("{:#x}", erc20_address);
    conn.exec_drop(
        "INSERT INTO clonings (chain_id, erc20_address, target_txid) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE target_txid = ?",
        (chain_id, &addr_str, target_txid, target_txid),
    )?;
    Ok(())
}

fn write_token_supply(
    db_conn: DbConn,
    chain_id: u64,
    erc20_address: Address,
    block_number: u64,
    token_supply: &HashMap<Address, TokenAmount>,
) -> Result<(), Box<dyn Error>> {
    let mut conn = db_conn.create_db_conn()?;
    // Delete existing records for the given erc20_address
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let stmt = "DELETE FROM tokens WHERE chain_id = ? AND erc20_address = ?";
    let str_address = format!("{:#x}", erc20_address);
    tx.exec_drop(stmt, (chain_id, &str_address))?; // Delete existing records for the given erc20_address

    // Insert new supplies
    for (token_address, supply) in token_supply {
        let stmt = "INSERT INTO tokens (chain_id, erc20_address, holder_address, holder_amount, signature) VALUES (?, ?, ?, ?, ?)";
        let addr_str = format!("{:#x}", erc20_address);
        let token_addr_str = format!("{:#x}", token_address);
        tx.exec_drop(
            stmt,
            (
                chain_id,
                addr_str,
                token_addr_str,
                supply.amount.to_string(),
                hex::encode(&supply.signature),
            ),
        )?;
    }
    // Insert new epoch
    let stmt = "INSERT INTO epochs (chain_id, erc20_address, block_number) VALUES(?, ?, ?)";
    tx.exec_drop(stmt, (chain_id, &str_address, block_number))?;

    tx.commit()?;
    Ok(())
}

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    original_snapshot: HashMap<Address, TokenAmount>,
    orchestrator_url: String,
    indexing_stats: Arc<Mutex<IndexerProcesses>>,
    db_conn: DbConn,
    private_key: LocalWallet,
) -> Result<(), Box<dyn Error>> {
    info!("Received indexed snapshot");
    {
        if let Ok(mut stats) = indexing_stats.lock() {
            if let Some(item) = stats.get_mut(&(request_data.chain_id, request_data.erc20_address))
            {
                item.current_timestamp = Utc::now().timestamp();
                item.status = VampingStatus::SendingToSolana;
            }
        }
    }
    // Convert the amount into a Solana format
    let (amount, decimals) = convert_to_sol(amount)?;
    let solana_snapshot = original_snapshot
        .iter()
        .map(|(k, v)| {
            let amount = v
                .amount
                .checked_div(U256::from(10u64.pow(18 - decimals as u32)));
            (*k, amount.unwrap_or_default().as_u64())
        })
        .collect::<HashMap<_, _>>();
    // Create the Merkle tree
    let leaves = solana_snapshot
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

    let salt = Utc::now().timestamp() as u64;

    let token_vamping_info = TokenVampingInfoProto {
        merkle_root: root.to_vec(),
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_bytes().to_vec(),
        token_uri: Some(request_data.token_uri),
        amount,
        decimal: decimals as u32,
        token_mapping: Some(TokenMappingProto {
            addresses: Vec::new(),
            amounts: Vec::new(),
        }),
        chain_id: request_data.chain_id,
        salt,
        solver_public_key: private_key.address().to_fixed_bytes().to_vec(),
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
    let response = client.submit_solution(request_proto).await?;
    let response_proto = response.into_inner();
    if let Some(result) = response_proto.result.to_owned() {
        let status: AppChainResultStatus = AppChainResultStatus::try_from(result.status)?;
        match status {
            AppChainResultStatus::Error => {
                let message = result.message.unwrap_or("Unknown error".to_string());
                if let Ok(mut stats) = indexing_stats.lock() {
                    if let Some(item) =
                        stats.get_mut(&(request_data.chain_id, request_data.erc20_address))
                    {
                        item.status = VampingStatus::Failure;
                        item.message = message.clone();
                    }
                }
                return Err(format!("Error in orchestrator response: {}", message).into());
            }
            AppChainResultStatus::Ok => {
                if let Some(payload) = response_proto.payload {
                    write_cloning(
                        db_conn.clone(),
                        request_data.chain_id,
                        request_data.erc20_address,
                        &payload.solana_txid,
                    )?;

                    let mut ethereum_snapshot = original_snapshot.clone();
                    // Truncate values that are < 1 Gwei, compute signatures
                    for (address, supply) in ethereum_snapshot.iter_mut() {
                        let signature = sign_balance(
                            request_data.chain_id,
                            &private_key,
                            address,
                            supply,
                            &payload,
                        )
                        .await?;
                        supply.signature = signature.to_vec();
                    }

                    // Writing the token supply to the database
                    write_token_supply(
                        db_conn.clone(),
                        request_data.chain_id,
                        request_data.erc20_address,
                        request_data.block_number,
                        &ethereum_snapshot,
                    )?;
                } else {
                    return Err("Payload not found in orchestrator response".into());
                }
                if let Ok(mut stats) = indexing_stats.lock() {
                    if let Some(item) =
                        stats.get_mut(&(request_data.chain_id, request_data.erc20_address))
                    {
                        item.status = VampingStatus::Success;
                    }
                }
                info!("The solver decision is successfully sent to the orchestrator.");
            }
            AppChainResultStatus::EventNotFound => {
                let message = format!(
                    "Orchestrator error: event {} not found",
                    request_data.sequence_id
                );
                let stats = indexing_stats.lock();
                if let Ok(mut stats) = stats {
                    if let Some(item) =
                        stats.get_mut(&(request_data.chain_id, request_data.erc20_address))
                    {
                        item.status = VampingStatus::Failure;
                        item.message = "Orchestrator error: event not found".to_string();
                    }
                }
                return Err(format!("Error in orchestrator response: {}", message).into());
            }
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

#[test]
fn test_create_cloning_intent_id() {
    // Test case: Valid txid and chain_id
    let txid = "3KMf5zj7q2Zk";
    let chain_id = 1u64;
    let result = create_cloning_solana_intent_id(txid, chain_id).unwrap();
    assert_eq!(result.len(), 32); // Keccak256 hash should be 32 bytes

    // Test case: Invalid txid (non-base58 string)
    let txid = "invalid_txid!";
    let chain_id = 1u64;
    let result = create_cloning_solana_intent_id(txid, chain_id);
    assert!(result.is_err());

    // Test case: Empty txid
    let txid = "";
    let chain_id = 1u64;
    let result = create_cloning_solana_intent_id(txid, chain_id);
    assert!(result.is_err());

    // Test case: Large chain_id
    let txid = "3KMf5zj7q2Zk";
    let chain_id = u64::MAX;
    let result = create_cloning_solana_intent_id(txid, chain_id).unwrap();
    assert_eq!(result.len(), 32); // Keccak256 hash should still be 32 bytes
}

#[tokio::test]
async fn test_sign_balance() {
    use ethers::core::k256::ecdsa::SigningKey;
    use std::str::FromStr;

    // Test setup
    let key = SigningKey::from_slice(
        &hex::decode("1ec9f456c48500dc267137437abffe4307cb2a9b54f1933b56315e5dec3683f5").unwrap(),
    )
    .unwrap();
    let private_key = LocalWallet::from(key);
    let chain_id = 1u64;
    let address = Address::from_str("0x589A698b7b7dA0Bec545177D3963A2741105C7C9").unwrap();
    let mut supply = TokenAmount {
        amount: U256::from(1_000_000_000_000_000_000u128),
        signature: Vec::new(),
    };
    let payload = AppChainPayloadProto {
        solana_txid: "3KMf5zj7q2Zk".to_string(),
        ..Default::default()
    };

    // Test case: Valid inputs
    let signature = sign_balance(chain_id, &private_key, &address, &mut supply, &payload).await;
    assert!(signature.is_ok());
    assert_eq!(
        signature.unwrap().to_string(),
        "1e96cf5740155208dff397042f4878f33d0a93a1e48fdc308a07361334a1e62c3d701d8882c5e73e84ff1eb1c1c2610dbae54a1992d61bca664955f00d6ad4c31b"
    );

    // Test case: Invalid Solana transaction ID in payload
    let invalid_payload = AppChainPayloadProto {
        solana_txid: "invalid_txid!".to_string(),
        ..Default::default()
    };
    let result = sign_balance(
        chain_id,
        &private_key,
        &address,
        &mut supply,
        &invalid_payload,
    )
    .await;
    assert!(result.is_err());

    // Test case: Empty Solana transaction ID in payload
    let empty_payload = AppChainPayloadProto {
        solana_txid: "".to_string(),
        ..Default::default()
    };
    let result = sign_balance(
        chain_id,
        &private_key,
        &address,
        &mut supply,
        &empty_payload,
    )
    .await;
    assert!(result.is_err());
}
