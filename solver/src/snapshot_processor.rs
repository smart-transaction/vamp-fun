use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use alloy::signers::{local::PrivateKeySigner, Signer};
use alloy_primitives::{Address, U256};
use anchor_client::Program;
use anchor_client::anchor_lang::declare_program;
use anyhow::{Context, Result, anyhow};
use balance_util::get_balance_hash;
use chrono::Utc;
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use solana_sdk::hash::Hash;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Keypair,
    signer::Signer as SolanaSigner, system_program, sysvar, transaction::Transaction,
};
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::ID as TOKEN_PROGRAM_ID;
use tracing::info;

use crate::mysql_conn::DbConn;
use crate::snapshot_indexer::{TokenAmount, TokenRequestData};
use crate::solana_transaction::SolanaTransaction;
use crate::stats::{IndexerProcesses, VampingStatus};

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
    flat_price_per_token: u64,
}

pub async fn process_and_send_snapshot(
    request_data: TokenRequestData,
    amount: U256,
    original_snapshot: std::collections::HashMap<Address, TokenAmount>,
    indexing_stats: Arc<Mutex<IndexerProcesses>>,
    db_conn: DbConn,
    eth_private_key: PrivateKeySigner,
    solana_payer_keypair: Arc<Keypair>,
    solana_program: Arc<Program<Arc<Keypair>>>,
    solana_url: &str
) -> Result<()> {
    info!(
        "Received indexed snapshot for intent_id: {}",
        hex::encode(&request_data.intent_id)
    );
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

    // Determine final vamping params with precedence: overrides > frontend/EVM (request_data) > solver defaults
    let final_paid_claiming_enabled = request_data.paid_claiming_enabled;
    let final_use_bonding_curve = request_data.use_bonding_curve;
    let final_curve_slope = request_data.curve_slope;
    let final_base_price = request_data.base_price;
    let final_max_price = 0;
    let final_flat_price_per_token = request_data.flat_price_per_token;

    // Now create the TokenVampingInfoProto with the validator address from the response
    let transaction_args = CloneTransactionArgs {
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_slice().to_vec(),
        token_uri: request_data.token_uri,
        amount,
        token_decimals: decimals,
        solver_public_key: eth_private_key.address().as_slice().to_vec(),
        validator_public_key: eth_private_key.address().as_slice().to_vec(),
        intent_id: request_data.intent_id.clone(),
        paid_claiming_enabled: final_paid_claiming_enabled,
        use_bonding_curve: final_use_bonding_curve,
        curve_slope: final_curve_slope,
        base_price: final_base_price,
        max_price: final_max_price,
        flat_price_per_token: final_flat_price_per_token,
    };

    let solana = SolanaTransaction::new(solana_url);

    let recent_blockhash = solana.get_latest_block_hash().await?;

    let (transaction, mint_account, vamp_state) = prepare_transaction(
        solana_payer_keypair.clone(),
        solana_program.clone(),
        recent_blockhash.to_bytes(),
        transaction_args,
    )?;

    let solana_txid = solana.submit_transaction(transaction).await?;

    info!("Solution transaction submitted: {}", solana_txid);
    write_cloning(
        db_conn.clone(),
        request_data.chain_id,
        request_data.erc20_address,
        solana_txid.to_string(),
        &mint_account.to_string(),
        &vamp_state.to_string(),
        "",
        &hex::encode(&request_data.intent_id),
    )
    .await?;

    let mut ethereum_snapshot = original_snapshot.clone();
    // Truncate values that are < 1 Gwei, compute signatures
    for (address, supply) in ethereum_snapshot.iter_mut() {
        let (amount, _) = convert_to_sol(&supply.amount)?;
        let balance_hash = get_balance_hash(&address.as_slice().to_vec(), amount, &request_data.intent_id)
            .map_err(|e| anyhow!("get balance hash: {}", e))?;
        let signature = eth_private_key.sign_message(&balance_hash).await?;
        supply.signature = signature.as_bytes().to_vec();
    }

    // Writing the token supply to the database
    write_token_supply(
        db_conn.clone(),
        request_data.chain_id,
        request_data.erc20_address,
        request_data.block_number,
        &ethereum_snapshot,
    )
    .await?;

    Ok(())
}

fn convert_to_sol(src_amount: &U256) -> Result<(u64, u8)> {
    // Truncate the amount to gwei
    let amount = src_amount
        .checked_div(U256::from(10u64.pow(9)))
        .ok_or(anyhow!("Failed to divide amount"))?;
    // Further truncating until the value fits u64
    // Setting it to zero right now, as we are fixed on decimals = 9.
    // Will be set to 9 later when we can customize decimals On Solana
    let max_extra_decimals = 9u8;
    for decimals in 0..=max_extra_decimals {
        let trunc_amount = amount
            .checked_div(U256::from(10u64.pow(decimals as u32)))
            .ok_or(anyhow!("Failed to divide amount"))?;
        // Check that we are not losing precision
        if trunc_amount
            .checked_mul(U256::from(10u64.pow(decimals as u32)))
            .ok_or(anyhow!("Failed to multiply amount"))?
            != amount
        {
            return Err(anyhow!(
                "The amount {:?} is too large to be minted on Solana",
                amount
            ));
        }
        let max_amount = U256::from(u64::MAX);
        if trunc_amount <= max_amount {
            let val: u64 = trunc_amount.try_into().map_err(|_| anyhow!("Failed to convert to u64"))?;
            return Ok((val, 9u8 - decimals));
        }
    }
    Err(anyhow!(
        "The amount {:?} is too large to be minted on Solana",
        amount
    ))
}

async fn write_cloning(
    db_conn: DbConn,
    chain_id: u64,
    erc20_address: Address,
    target_txid: String,
    mint_account_address: &str,
    vamp_state_address: &str,
    root_intent_cid: &str,
    intent_id: &str,
) -> Result<()> {
    let conn = db_conn
        .create_db_conn()
        .await
        .map_err(|e| anyhow!("create DB com=nnection: {}", e))?;
    let addr_str = format!("{:#x}", erc20_address);

    sqlx::query(
        r#"
            INSERT INTO clonings (
                chain_id,
                erc20_address,
                target_txid,
                mint_account_address,
                token_spl_address,
                root_intent_cid,
                intent_id,
                created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NOW())
        "#,
    )
    .bind(&chain_id)
    .bind(&addr_str)
    .bind(target_txid)
    .bind(mint_account_address)
    .bind(vamp_state_address)
    .bind(root_intent_cid)
    .bind(intent_id)
    .execute(&conn)
    .await
    .context("write cloning")?;

    Ok(())
}

async fn write_token_supply(
    db_conn: DbConn,
    chain_id: u64,
    erc20_address: Address,
    block_number: u64,
    token_supply: &HashMap<Address, TokenAmount>,
) -> Result<()> {
    let conn = db_conn
        .create_db_conn()
        .await
        .map_err(|e| anyhow!("error connecting to database: {}", e))?;
    // Delete existing records for the given erc20_address
    let mut tx = conn.begin().await.context("begin tx")?;
    let str_address = format!("{:#x}", erc20_address);
    sqlx::query(
        r#"
            DELETE FROM tokens
            WHERE chain_id = ?
                AND erc20_address = ?
        "#,
    )
    .bind(&chain_id)
    .bind(&str_address)
    .execute(&mut *tx)
    .await
    .context("delete existing token supply")?;

    // Insert new supplies
    for (token_address, supply) in token_supply {
        let addr_str = format!("{:#x}", erc20_address);
        let token_addr_str = format!("{:#x}", token_address);
        sqlx::query(
            r#"
                INSERT INTO tokens (
                    chain_id,
                    erc20_address,
                    holder_address,
                    holder_amount,
                    signature
                )
                VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&chain_id)
        .bind(&addr_str)
        .bind(&token_addr_str)
        .bind(supply.amount.to_string().as_str())
        .bind(hex::encode(&supply.signature).as_str())
        .execute(&mut *tx)
        .await
        .context("insert token supply")?;
    }
    // Insert new epoch
    sqlx::query(
        r#"
            INSERT INTO epochs (
                chain_id,
                erc20_address,
                block_number)
            VALUES(?, ?, ?)
        "#,
    )
    .bind(&chain_id)
    .bind(&str_address)
    .bind(&block_number)
    .execute(&mut *tx)
    .await
    .context("insert new epoch")?;

    tx.commit().await.context("commit transaction")?;
    Ok(())
}

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

fn prepare_transaction(
    payer_keypair: Arc<Keypair>,
    program: Arc<Program<Arc<Keypair>>>,
    recent_blockhash: [u8; 32],
    transaction_args: CloneTransactionArgs,
) -> Result<(Transaction, Pubkey, Pubkey)> {
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

    let (sol_vault, _) = Pubkey::find_program_address(
        &[b"sol_vault", mint_account.as_ref()],
        &solana_vamp_program::ID,
    );
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
            flat_price_per_token: transaction_args.flat_price_per_token,
        })
        .instructions()?;

    info!(
        "🔧 Creating token mint with decimals: {}",
        transaction_args.token_decimals
    );

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

fn fold_intent_id(intent_id: &[u8]) -> Result<u64> {
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
