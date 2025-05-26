use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{Json, extract::Query, http::StatusCode};
use ethers::types::U256;
use log::error;
use mysql::{prelude::Queryable, Row, Value};
use serde::{Deserialize, Serialize};

use crate::{
    mysql_conn::DbConn,
    stats::{IndexerProcesses, IndexerStats},
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
}

pub fn handle_get_claim_amount(
    params: Query<HashMap<String, String>>,
    db_conn: DbConn,
) -> Result<Json<TokenClaimData>, StatusCode> {
    let db_conn = db_conn.create_db_conn();
    if let Err(err) = db_conn {
        log::error!("Failed to create DB connection: {:?}", err);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut db_conn = db_conn.unwrap();

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
    };

    let stmt = "SELECT holder_amount, signature FROM tokens WHERE chain_id = ? AND erc20_address = ? AND holder_address = ?";
    match db_conn.exec_first(stmt, (&chain_id, &token_address, &user_address)) {
        Ok(row) => {
            if row.is_none() {
                log::error!("No data found for the given parameters");
                return Err(StatusCode::NOT_FOUND);
            }
            let row: Row = row.unwrap();
            let amount: Option<String> = row.get(0);
            if let Some(amount) = amount {
                let num_amount = U256::from_dec_str(&amount)
                    .unwrap_or_default()
                    .checked_div(U256::from(10u64.pow(9)))
                    .unwrap_or_default();
                claim_data.amount = num_amount.to_string();
            }
            let mut solver_signature = String::new();
            let signature_value: Value = row.get(1).unwrap_or(Value::NULL);
            if signature_value != Value::NULL {
                solver_signature = row.get(1).unwrap();
            }
            claim_data.solver_signature = solver_signature.clone();
            // Temporary duplication of the solver signature, the validator signature will be added later
            claim_data.validator_signature = solver_signature;
        }
        Err(err) => {
            log::error!("Failed to execute query: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    let stmt = "SELECT target_txid, token_spl_address, mint_account_address FROM clonings WHERE chain_id = ? AND erc20_address = ?";
    match db_conn.exec_first(stmt, (&chain_id, &token_address)) {
        Ok(row) => {
            let row: Row = row.unwrap();
            let target_txid: Option<String> = row.get(0);
            let token_spl_address: Option<String> = row.get(1);
            let mint_account_address: Option<String> = row.get(2);
            claim_data.target_txid = target_txid.unwrap_or("".to_string());
            claim_data.token_spl_address = token_spl_address.unwrap_or("".to_string());
            claim_data.mint_account_address = mint_account_address.unwrap_or("".to_string());
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
