use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use alloy_primitives::U256;
use anyhow::Result;
use axum::{Json, extract::Query, http::StatusCode};
use sqlx::Row;
use tracing::error;
use serde::{Deserialize, Serialize};

use crate::{
    mysql_conn::DbConn, stats::{IndexerProcesses, IndexerStats}
};

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenClaimData {
    pub token_address: String,
    pub user_address: String,
    pub amount: String,
    pub decimals: u8,
    pub target_txid: String,
    pub solver_signature: String,
    pub validator_signature: String,
    pub mint_account_address: String,
    pub token_spl_address: String,
    pub root_intent_cid: String,
    pub intent_id: String,
}

pub async fn handle_get_claim_amount(
    params: Query<HashMap<String, String>>,
    db_conn: DbConn,
) -> Result<Json<TokenClaimData>, StatusCode> {
    let db_conn = db_conn.create_db_conn().await;
    if let Err(err) = db_conn {
        log::error!("Failed to create DB connection: {:?}", err);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let db_conn = db_conn.unwrap();

    let chain_id = match params.get("chain_id") {
        Some(chain_id) => match u64::from_str_radix(chain_id, 10) {
            Ok(id) => id,
            Err(_) => {
                log::error!("Invalid chain_id parameter");
                return Err(StatusCode::BAD_REQUEST);
            }
        },
        None => {
            log::error!("Missing chain_id parameter");
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let token_address = match params.get("token_address") {
        Some(address) => address.to_lowercase(),
        None => {
            log::error!("Missing token_address parameter");
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let user_address = match params.get("user_address") {
        Some(address) => address.to_lowercase(),
        None => {
            log::error!("Missing user_address parameter");
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let mut claim_data = TokenClaimData {
        token_address: token_address.clone(),
        user_address: user_address.clone(),
        amount: "0".to_string(),
        decimals: 9,
        target_txid: "".to_string(),
        solver_signature: "".to_string(),
        validator_signature: "".to_string(),
        mint_account_address: "".to_string(),
        token_spl_address: "".to_string(),
        root_intent_cid: "".to_string(),
        intent_id: "".to_string(),
    };

    let rows = sqlx::query(
        r#"
            SELECT holder_amount, signature
            FROM tokens
            WHERE chain_id = ?
              AND erc20_address = ?
              AND holder_address = ?"#)
        .bind(&chain_id)
        .bind(&token_address)
        .bind(&user_address)
        .fetch_all(&db_conn)
        .await;

    match rows {
        Ok(rows) => {
            if rows.is_empty() {
                log::error!("No token data found for the given parameters");
                return Err(StatusCode::NOT_FOUND);
            }
            let row = &rows[0];
            let amount = row.get::<&str, usize>(0);
            let num_amount = U256::from_str_radix(amount, 10)
                .unwrap_or_default()
                .checked_div(U256::from(10u64.pow(9)))
                .unwrap_or_default();
            claim_data.amount = num_amount.to_string();
            let solver_signature = row.get::<&str, usize>(1);
            claim_data.solver_signature = solver_signature.to_string();
            // Temporary duplication of the solver signature, the validator signature will be added later
            claim_data.validator_signature = solver_signature.to_string();
        }
        Err(err) => {
            log::error!("Failed to execute query: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let rows = sqlx::query(
        r#"
            SELECT target_txid, token_spl_address, mint_account_address, root_intent_cid, intent_id
            FROM clonings
            WHERE chain_id = ?
              AND erc20_address = ?
            ORDER BY created_at DESC LIMIT 1
        "#
    )
    .bind(&chain_id)
    .bind(&token_address)
    .fetch_all(&db_conn)
    .await;
    match rows {
        Ok(rows) => {
            if rows.is_empty() {
                log::error!("No cloning data found for the given parameters");
                return Err(StatusCode::NOT_FOUND);
            }
            let row = &rows[0];
            let target_txid = row.get::<&str, usize>(0);
            let token_spl_address = row.get::<&str, usize>(1);
            let mint_account_address = row.get::<&str, usize>(2);
            let root_intent_cid = row.get::<&str, usize>(3);
            let intent_id = row.get::<&str, usize>(4);
            claim_data.target_txid = target_txid.to_string();
            claim_data.token_spl_address = token_spl_address.to_string();
            claim_data.mint_account_address = mint_account_address.to_string();
            claim_data.root_intent_cid = root_intent_cid.to_string();
            claim_data.intent_id = intent_id.to_string();
        }
        Err(err) => {
            log::error!("Failed to execute query: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    return Ok(Json(claim_data));
}

pub fn handle_get_stats(
    params: Query<HashMap<String, String>>,
    stats: Arc<Mutex<IndexerProcesses>>,
) -> Result<Json<IndexerStats>, StatusCode> {
    let mut stats = stats.lock().map_err(|err| {
        error!("Lock error: {}", err);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let chain_id = params.get("chain_id");
    let erc20_address = params.get("erc20_address");
    if chain_id == None || erc20_address == None {
        return Err(StatusCode::BAD_REQUEST);
    }
    let chain_id = u64::from_str_radix(chain_id.unwrap(), 10);
    let erc20_address = erc20_address.unwrap().as_str().parse();
    if chain_id.is_err() || erc20_address.is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match stats.get_mut(&(chain_id.unwrap(), erc20_address.unwrap())) {
        Some(item) => {
            return Ok(Json(item.clone()));
        }
        None => {
            return Err(StatusCode::NOT_FOUND);
        }
    }
}

// 21363 | 0xb69a656b2be8aa0b3859b24eed3c22db206ee966
