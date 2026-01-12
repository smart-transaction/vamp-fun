use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use alloy::signers::Signer;
use alloy_primitives::{Address, U256};
use anchor_client::{Client as AnchorClient, Cluster, Program};
use anchor_lang::declare_program;
use anyhow::{Context, Result, anyhow};
use balance_util::get_balance_hash;
use chrono::Utc;
use intent_id_util::fold_intent_id;
use solana_sdk::signature::Keypair;
use tracing::info;

use crate::cfg::Cfg;
use crate::mysql_conn::create_db_conn;
use crate::snapshot_indexer::{TokenAmount, TokenRequestData};
use crate::solana_transaction::SolanaTransaction;
use crate::solana_transaction::solana_vamp_program::client::args;
use crate::stats::{IndexerProcesses, VampingStatus};

declare_program!(solana_vamp_program);

fn get_program_instance(payer_keypair: Arc<Keypair>) -> Result<Program<Arc<Keypair>>> {
    // The cluster doesn't matter here, it's used only for the instructions creation.
    let anchor_client = AnchorClient::new(Cluster::Debug, payer_keypair.clone());
    Ok(anchor_client.program(solana_vamp_program::ID)?)
}

pub async fn process_and_send_snapshot(
    cfg: Arc<Cfg>,
    request_data: TokenRequestData,
    amount: U256,
    original_snapshot: HashMap<Address, TokenAmount>,
    indexing_stats: Arc<RwLock<IndexerProcesses>>,
) -> Result<()> {
    info!(
        "Received indexed snapshot for intent_id: {}",
        hex::encode(&request_data.intent_id)
    );
    {
        if let Ok(mut stats) = indexing_stats.write() {
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

    let transaction_args = args::CreateTokenMint {
        vamp_identifier: fold_intent_id(&request_data.intent_id)?,
        token_decimals: decimals,
        token_name: request_data.token_full_name,
        token_symbol: request_data.token_symbol_name,
        token_erc20_address: request_data.erc20_address.as_slice().to_vec(),
        token_uri: request_data.token_uri,
        amount,
        solver_public_key: cfg.ethereum_private_key.address().as_slice().to_vec(),
        validator_public_key: cfg.ethereum_private_key.address().as_slice().to_vec(),
        intent_id: request_data.intent_id.clone(),
        paid_claiming_enabled: final_paid_claiming_enabled,
        use_bonding_curve: final_use_bonding_curve,
        curve_slope: final_curve_slope,
        base_price: final_base_price,
        max_price: final_max_price,
        flat_price_per_token: final_flat_price_per_token,
    };

    let solana_url = if cfg.default_solana_cluster == "DEVNET" {
        cfg.solana_devnet_url.clone()
    } else {
        cfg.solana_mainnet_url.clone()
    };

    let solana = SolanaTransaction::new(solana_url);

    let solana_payer_keypair = Arc::new(Keypair::from_base58_string(&cfg.solana_private_key));
    let solana_program = Arc::new(get_program_instance(solana_payer_keypair.clone())?);

    let (transaction, mint_account, vamp_state) = solana.prepare(
        solana_payer_keypair.clone(),
        solana_program.clone(),
        transaction_args,
    ).await?;

    let solana_txid = solana.submit_transaction(transaction).await?;

    info!("Solution transaction submitted: {}", solana_txid);
    write_cloning(
        &cfg,
        request_data.chain_id,
        request_data.erc20_address,
        solana_txid.to_string(),
        &mint_account.to_string(),
        &vamp_state.to_string(),
        &hex::encode(&request_data.intent_id),
    )
    .await?;

    let mut ethereum_snapshot = original_snapshot.clone();
    // Truncate values that are < 1 Gwei, compute signatures
    for (address, supply) in ethereum_snapshot.iter_mut() {
        let (amount, _) = convert_to_sol(&supply.amount)?;
        let balance_hash = get_balance_hash(
            &address.as_slice().to_vec(),
            amount,
            &request_data.intent_id,
        )
        .map_err(|e| anyhow!("get balance hash: {}", e))?;
        let signature = cfg.ethereum_private_key.sign_message(&balance_hash).await?;
        supply.signature = signature.as_bytes().to_vec();
    }

    // Writing the token supply to the database
    write_token_supply(
        &cfg,
        request_data.chain_id,
        request_data.erc20_address,
        request_data.block_number,
        &ethereum_snapshot,
        &hex::encode(&request_data.intent_id),
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
            let val: u64 = trunc_amount
                .try_into()
                .map_err(|_| anyhow!("Failed to convert to u64"))?;
            return Ok((val, 9u8 - decimals));
        }
    }
    Err(anyhow!(
        "The amount {:?} is too large to be minted on Solana",
        amount
    ))
}

async fn write_cloning(
    cfg: &Cfg,
    chain_id: u64,
    erc20_address: Address,
    target_txid: String,
    mint_account_address: &str,
    vamp_state_address: &str,
    intent_id: &str,
) -> Result<()> {
    let conn = create_db_conn(cfg)
        .await
        .map_err(|e| anyhow!("create DB connection: {}", e))?;
    let addr_str = format!("{:#x}", erc20_address);

    sqlx::query(
        r#"
            INSERT INTO clonings (
                chain_id,
                erc20_address,
                target_txid,
                mint_account_address,
                token_spl_address,
                intent_id)
            VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&chain_id)
    .bind(&addr_str)
    .bind(target_txid)
    .bind(mint_account_address)
    .bind(vamp_state_address)
    .bind(intent_id)
    .execute(&conn)
    .await
    .context("write cloning")?;

    Ok(())
}

async fn write_token_supply(
    cfg: &Cfg,
    chain_id: u64,
    erc20_address: Address,
    block_number: u64,
    token_supply: &HashMap<Address, TokenAmount>,
    intent_id: &str,
) -> Result<()> {
    let conn = create_db_conn(cfg)
        .await
        .map_err(|e| anyhow!("error connecting to database: {}", e))?;
    let mut tx = conn.begin().await.context("begin tx")?;
    let str_address = format!("{:#x}", erc20_address);

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
                    signature,
                    intent_id
                )
                VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&chain_id)
        .bind(&addr_str)
        .bind(&token_addr_str)
        .bind(supply.amount.to_string().as_str())
        .bind(hex::encode(&supply.signature).as_str())
        .bind(intent_id)
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

#[cfg(test)]
mod tests {
    use super::*;
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
