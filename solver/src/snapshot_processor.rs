use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};

use anchor_client::Program;
use anchor_client::anchor_lang::declare_program;
use balance_util::get_balance_hash;
use chrono::Utc;
use ethers::utils::keccak256;
use ethers::{
    signers::{LocalWallet, Signer},
    types::{Address, U256},
};
use tracing::info;
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use mysql::TxOpts;
use mysql::prelude::Queryable;
use prost::Message;
use sha3::Digest;
use solana_sdk::hash::Hash;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Keypair,
    signer::Signer as SolanaSigner, system_program, sysvar, transaction::Transaction,
};
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::ID as TOKEN_PROGRAM_ID;
use tonic::Status;
use tonic::transport::Channel;

use crate::mysql_conn::DbConn;
use crate::request_registrator_listener::VAMPING_APP_ID;
use crate::snapshot_indexer::{TokenAmount, TokenRequestData};
use crate::stats::{IndexerProcesses, VampingStatus};
use crate::use_proto::proto::chain_selection_proto::Chain;
use crate::use_proto::proto::{AppChainResultStatus, ChainSelectionProto, LatestBlockHashRequestProto, SolanaCluster, SubmitSolutionForValidationRequestProto, VampSolutionForValidationProto};
use crate::use_proto::proto::{
    SubmitSolutionRequestProto, UserEventProto,
    orchestrator_service_client::OrchestratorServiceClient,
    validator_service_client::ValidatorServiceClient,
};
use crate::use_proto::proto::VampSolutionValidatedDetailsProto;

struct CloneTransactionArgs {
    token_decimals: u8,
    token_name: String,
    token_symbol: String,
    token_erc20_address: Vec<u8>,
    token_uri: String,
    amount: u64,
    solver_public_key: Vec<u8>,
    validator_public_key: Vec<u8>,
    intent_id: Vec<u8>,
    paid_claiming_enabled: bool,
    use_bonding_curve: bool,
    curve_slope: u64,
    base_price: u64,
    max_price: u64,
    flat_price_per_token: u64
}

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    original_snapshot: std::collections::HashMap<Address, TokenAmount>,
    validator_url: String,
    orchestrator_url: String,
    indexing_stats: Arc<Mutex<IndexerProcesses>>,
    db_conn: DbConn,
    eth_private_key: LocalWallet,
    solana_payer_keypair: Arc<Keypair>,
    solana_program: Arc<Program<Arc<Keypair>>>,
    // Add solver vamping parameters as fallback values
    solver_paid_claiming_enabled: bool,
    solver_use_bonding_curve: bool,
    solver_curve_slope: u64,
    solver_base_price: u64,
    solver_flat_price_per_token: u64,
) -> Result<(), Box<dyn Error>> {
    info!("Received indexed snapshot for intent_id: {}", hex::encode(&request_data.intent_id));
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
    let (amount, decimals) = convert_to_sol(&amount)?;

    let mut user_event = UserEventProto::default();
    user_event.app_id = keccak256(VAMPING_APP_ID.as_bytes()).to_vec();

    // Build the individual_balance_entry_by_oth_address map for the proto
    let mut individual_balance_entry_by_oth_address = std::collections::HashMap::new();
    for (address, token_amount) in &original_snapshot {
        // Convert the balance to u64 using convert_to_sol
        let (balance, _) = convert_to_sol(&token_amount.amount)?;
        // Build the message: sha3::Keccak256(eth_address || balance || intent_id)
        let mut hasher = sha3::Keccak256::new();
        hasher.update(&address.0);  // Use raw 20-byte address instead of string bytes
        hasher.update(&balance.to_le_bytes());
        hasher.update(&request_data.intent_id);
        let message = hasher.finalize();
        // Sign the message with the solver's private key
        let solver_sig = eth_private_key.sign_message(message).await?;
        // Construct the IndividualBalanceEntry
        let entry = crate::use_proto::proto::IndividualBalanceEntry {
            balance,
            solver_individual_balance_sig: hex::encode(solver_sig.to_vec()),
            validator_individual_balance_sig: String::new(),
        };
        individual_balance_entry_by_oth_address.insert(format!("{:#x}", address), entry);
    }
    // Now use this map in your VampSolutionForValidationProto
    let solution_for_validation = VampSolutionForValidationProto {
        intent_id: hex::encode(request_data.intent_id.clone()),
        solver_pubkey: eth_private_key.address().to_string(),
        individual_balance_entry_by_oth_address,
    };
    
    let mut solution_for_validation_encoded = Vec::with_capacity(solution_for_validation.encoded_len());
    solution_for_validation.encode(&mut solution_for_validation_encoded)
        .map_err(|e| Status::internal(format!("Protobuf encode error: {e}")))?;
    
    let validation_request_proto = SubmitSolutionForValidationRequestProto {
        intent_id: hex::encode(request_data.intent_id.clone()),
        solution_for_validation: solution_for_validation_encoded,
    };
    
    // Send the validation request to the validator
    let mut validator_client: ValidatorServiceClient<Channel> =
        ValidatorServiceClient::connect(validator_url.clone()).await?;
    info!("Connected to validator at {}", validator_url);
    
    info!("Sending validation request to validator for intent_id: {}", hex::encode(&request_data.intent_id));
    let response = validator_client
        .submit_solution(validation_request_proto)
        .await?;
    info!("Received response from validator for intent_id: {}", hex::encode(&request_data.intent_id));
    
    let response_proto = response.into_inner();
    info!("Extracted response proto for intent_id: {}", hex::encode(&request_data.intent_id));
    
    // Check validator response status first
    let vamp_validated_details = if let Some(result) = response_proto.result {
        info!("Validator response has result for intent_id: {}", hex::encode(&request_data.intent_id));
        let status: AppChainResultStatus = AppChainResultStatus::try_from(result.status)?;
        info!("Validator response status: {:?} for intent_id: {}", status, hex::encode(&request_data.intent_id));
        match status {
            AppChainResultStatus::Ok => {
                // Handle the response: check for success and extract data
                let payload: Vec<u8> = response_proto.solution_validated_details;
                info!("Validator response payload size: {} bytes for intent_id: {}", payload.len(), hex::encode(&request_data.intent_id));
                let vamp_validated_details = VampSolutionValidatedDetailsProto::decode(&*payload)?;
                info!("Validation successful for intent_id: {}. Root CID: {}", hex::encode(&request_data.intent_id), vamp_validated_details.root_intent_cid);
                vamp_validated_details
            }
            AppChainResultStatus::EventNotFound => {
                let message = result.message.unwrap_or("Unknown error".to_string());
                return Err(format!("Validator error for intent_id {}: event not found - {}", hex::encode(&request_data.intent_id), message).into());
            }
            AppChainResultStatus::Error => {
                let message = result.message.unwrap_or("Unknown error".to_string());
                return Err(format!("Validator error for intent_id {}: {}", hex::encode(&request_data.intent_id), message).into());
            }
        }
    } else {
        return Err(format!("Validator response missing result status for intent_id: {}", hex::encode(&request_data.intent_id)).into());
    };

    // Determine final vamping params with precedence: overrides > frontend/EVM (request_data) > solver defaults
    let final_paid_claiming_enabled = request_data.paid_claiming_enabled.unwrap_or(solver_paid_claiming_enabled);
    let final_use_bonding_curve = request_data.use_bonding_curve.unwrap_or(solver_use_bonding_curve);
    let final_curve_slope = request_data.curve_slope.unwrap_or(solver_curve_slope);
    let final_base_price = request_data.base_price.unwrap_or(solver_base_price);
    let final_max_price = 0;
    let final_flat_price_per_token = request_data.flat_price_per_token.unwrap_or(solver_flat_price_per_token);

    // Now create the TokenVampingInfoProto with the validator address from the response
    let transaction_args = CloneTransactionArgs {
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_bytes().to_vec(),
        token_uri: request_data.token_uri,
        amount,
        token_decimals: decimals,
        solver_public_key: eth_private_key.address().to_fixed_bytes().to_vec(),
        validator_public_key: hex::decode(vamp_validated_details.validator_address.strip_prefix("0x").unwrap_or(&vamp_validated_details.validator_address))?,
        intent_id: request_data.intent_id.clone(),
        paid_claiming_enabled: final_paid_claiming_enabled,
        use_bonding_curve: final_use_bonding_curve,
        curve_slope: final_curve_slope,
        base_price: final_base_price,
        max_price: final_max_price,
        flat_price_per_token: final_flat_price_per_token,
    };

    // Log vamping parameters for debugging
    info!("📋 Creating vamping with parameters:");
    info!("   Paid Claiming Enabled: {}", final_paid_claiming_enabled);
    info!("   Use Bonding Curve: {}", final_use_bonding_curve);
    info!("   Curve Slope: {}", final_curve_slope);
    info!("   Base Price: {} lamports", final_base_price);
    info!("   Max Price: {:?} lamports", final_max_price);
    info!("   Flat Price Per Token: {} lamports", final_flat_price_per_token);
    info!("   Intent ID: 0x{}", hex::encode(&request_data.intent_id));

    let mut orchestrator_client: OrchestratorServiceClient<Channel> =
        OrchestratorServiceClient::connect(orchestrator_url.clone()).await?;
    info!("Connected to orchestrator at {}", orchestrator_url);

    let solana_cluster_proto = request_data
        .solana_cluster
        .unwrap_or(SolanaCluster::Devnet);

    let mut blockhash_request_proto = LatestBlockHashRequestProto::default();
    if let Some(_) = request_data.solana_cluster {
        blockhash_request_proto.chain = Some(ChainSelectionProto {
            chain: Some(Chain::SolanaCluster(solana_cluster_proto.into()).into()),
        });
    }

    let blockhash_response = orchestrator_client
        .get_latest_block_hash(blockhash_request_proto)
        .await?;
    let blockhash_response_proto = blockhash_response.into_inner();
    if let Some(result) = blockhash_response_proto.result {
        if result.status != AppChainResultStatus::Ok as i32 {
            return Err(format!(
                "Error geting the laterst block hash: {}",
                result.message.unwrap_or("Unknown error".to_string())
            )
            .into());
        }
    }
    let recent_blockhash: [u8; 32] = blockhash_response_proto
        .block_hash
        .iter()
        .as_slice()
        .try_into()?;

    let (transaction, mint_account, vamp_state) = prepare_transaction(
        solana_payer_keypair.clone(),
        solana_program.clone(),
        recent_blockhash,
        transaction_args
    )?;

    let transaction = postcard::to_allocvec(&transaction);
    let transaction = transaction
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?
        .to_vec();

    let request_proto = SubmitSolutionRequestProto {
        request_sequence_id: request_data.sequence_id,
        chain: Some(ChainSelectionProto {
            chain: Some(Chain::SolanaCluster(solana_cluster_proto.into()).into()),
        }),
        transaction: transaction.to_vec(),
    };
    let response = orchestrator_client.submit_solution(request_proto).await?;
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
                return Err(format!("Error in orchestrator response for intent_id {}: {}", hex::encode(&request_data.intent_id), message).into());
            }
            AppChainResultStatus::Ok => {
                if let Some(payload) = response_proto.payload {
                    info!("Solution transaction submitted: {}", payload.solana_txid);
                    write_cloning(
                        db_conn.clone(),
                        request_data.chain_id,
                        request_data.erc20_address,
                        &payload.solana_txid,
                        &mint_account.to_string(),
                        &vamp_state.to_string(),
                        &vamp_validated_details.root_intent_cid,
                        &hex::encode(&request_data.intent_id),
                    )?;

                    let mut ethereum_snapshot = original_snapshot.clone();
                    // Truncate values that are < 1 Gwei, compute signatures
                    for (address, supply) in ethereum_snapshot.iter_mut() {
                        let (amount, _) = convert_to_sol(&supply.amount)?;
                        let balance_hash =
                            get_balance_hash(&address.0.to_vec(), amount, &request_data.intent_id)?;
                        let signature = eth_private_key.sign_message(&balance_hash).await?;
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
                    return Err(format!("Payload not found in orchestrator response for intent_id: {}", hex::encode(&request_data.intent_id)).into());
                }
                if let Ok(mut stats) = indexing_stats.lock() {
                    if let Some(item) =
                        stats.get_mut(&(request_data.chain_id, request_data.erc20_address))
                    {
                        item.status = VampingStatus::Success;
                    }
                }
                info!("The solution is successfully executed on the orchestrator for intent_id: {}", hex::encode(&request_data.intent_id));
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
                return Err(format!("Error in orchestrator response for intent_id {}: {}", hex::encode(&request_data.intent_id), message).into());
            }
        }
    }

    Ok(())
}

fn convert_to_sol(src_amount: &U256) -> Result<(u64, u8), Box<dyn Error>> {
    // Truncate the amount to gwei
    let amount = src_amount
        .checked_div(U256::from(10u64.pow(9)))
        .ok_or("Failed to divide amount")?;
    // Further truncating until the value fits u64
    // Setting it to zero right now, as we are fixed on decimals = 9.
    // Will be set to 9 later when we can customize decimals On Solana
    let max_extra_decimals = 9u8;
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
    mint_account_address: &str,
    vamp_state_address: &str,
    root_intent_cid: &str,
    intent_id: &str,
) -> Result<(), Box<dyn Error>> {
    let mut conn = db_conn.create_db_conn()?;
    let addr_str = format!("{:#x}", erc20_address);
    conn.exec_drop(
        "INSERT INTO clonings (chain_id, erc20_address, target_txid, mint_account_address, token_spl_address, root_intent_cid, intent_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, NOW())",
        (chain_id, &addr_str, target_txid, mint_account_address, vamp_state_address, root_intent_cid, intent_id),
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

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

fn prepare_transaction(
    payer_keypair: Arc<Keypair>,
    program: Arc<Program<Arc<Keypair>>>,
    recent_blockhash: [u8; 32],
    transaction_args: CloneTransactionArgs,
) -> Result<(Transaction, Pubkey, Pubkey), Box<dyn Error>> {
    let vamp_identifier = fold_intent_id(&transaction_args.intent_id)?;

    let (mint_account, _) = Pubkey::find_program_address(
        &[
            b"mint",
            payer_keypair.pubkey().as_ref(),
            vamp_identifier.to_le_bytes().as_ref(),
        ],
        &solana_vamp_program::ID,
    );

    let (metadata_account, _bump) = Pubkey::find_program_address(
        &[
            b"metadata",
            TOKEN_METADATA_PROGRAM_ID.as_ref(),
            mint_account.as_ref(),
        ],
        &TOKEN_METADATA_PROGRAM_ID,
    );

    let (vamp_state, _) =
        Pubkey::find_program_address(&[b"vamp", mint_account.as_ref()], &solana_vamp_program::ID);

    let (vault, _) =
        Pubkey::find_program_address(&[b"vault", mint_account.as_ref()], &solana_vamp_program::ID);
    
    let (sol_vault, _) =
        Pubkey::find_program_address(&[b"sol_vault", mint_account.as_ref()], &solana_vamp_program::ID);
    let program_instructions = program
        .request()
        .accounts(accounts::CreateTokenMint {
            authority: payer_keypair.pubkey(),
            // mint_account: destination_token_address,
            mint_account,
            metadata_account,
            vamp_state,
            vault,
            sol_vault,
            token_program: TOKEN_PROGRAM_ID,
            token_metadata_program: TOKEN_METADATA_PROGRAM_ID,
            system_program: system_program::ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            rent: sysvar::rent::ID,
        })
        .args(args::CreateTokenMint {
            vamp_identifier,
            token_decimals: transaction_args.token_decimals,
            token_name: transaction_args.token_name,
            token_symbol: transaction_args.token_symbol,
            token_erc20_address: transaction_args.token_erc20_address,
            token_uri: transaction_args.token_uri,
            amount: transaction_args.amount,
            solver_public_key: transaction_args.solver_public_key,
            validator_public_key: transaction_args.validator_public_key,
            intent_id: transaction_args.intent_id,
            paid_claiming_enabled: transaction_args.paid_claiming_enabled,
            use_bonding_curve: transaction_args.use_bonding_curve,
            curve_slope: transaction_args.curve_slope,
            base_price: transaction_args.base_price,
            max_price: transaction_args.max_price,
            flat_price_per_token: transaction_args.flat_price_per_token
        })
        .instructions()?;

    info!("🔧 Creating token mint with decimals: {}", transaction_args.token_decimals);

    // Add compute limit
    let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);
    let mut all_instructions = vec![compute_ix];
    all_instructions.extend(program_instructions);

    let tx = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer_keypair.pubkey()),
        &[&*payer_keypair],
        Hash::new_from_array(recent_blockhash),
    );
    Ok((tx, mint_account, vamp_state))
}

fn fold_intent_id(intent_id: &[u8]) -> Result<u64, Box<dyn Error>> {
    let mut hash64 = 0u64;
    for chunk in intent_id.chunks(8) {
        let chunk_value = u64::from_le_bytes(chunk.try_into()?);
        hash64 ^= chunk_value; // XOR the chunks to reduce to 64 bits
    }
    Ok(hash64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_intent_id_empty() {
        let intent_id = vec![];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_fold_intent_id_single_chunk() {
        let intent_id = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_fold_intent_id_multiple_chunks() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // 1 XOR 2 = 3
    }

    #[test]
    fn test_fold_intent_id_partial_chunk() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, // Partial second chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_fold_intent_id_large_input() {
        let intent_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
            3, 0, 0, 0, 0, 0, 0, 0, // Third chunk
        ];
        let result = fold_intent_id(&intent_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 1 XOR 2 XOR 3 = 0
    }

    #[test]
    fn test_convert_to_sol_small_value() {
        let res = convert_to_sol(&U256::from(123456789777000000111u128));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (123456789777, 9));
    }

    #[test]
    fn test_convert_to_sol_large_value() {
        let res = convert_to_sol(&U256::from(123123123456789123000000000000000111u128));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (12312312345678912300, 2));
    }

    #[test]
    fn test_convert_to_sol_too_large_value() {
        let res = convert_to_sol(&U256::from(123123123456789123555555000000000111u128));
        assert!(res.is_err());
    }
}
