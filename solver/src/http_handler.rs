use std::collections::HashMap;

use axum::{Json, extract::Query, http::StatusCode};
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};

use crate::mysql_conn;

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenClaimData {
    pub token_address: String,
    pub user_address: String,
    pub amount: String,
}

pub fn handle_get_claim_amount(
    params: Query<HashMap<String, String>>,
    mysql_host: String,
    mysql_port: String,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
) -> Result<Json<TokenClaimData>, StatusCode> {
    let db_conn = mysql_conn::create_db_conn(
        &mysql_host,
        &mysql_port,
        &mysql_user,
        &mysql_password,
        &mysql_database,
    );
    if let Err(err) = db_conn {
        log::error!("Failed to create DB connection: {:?}", err);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let mut db_conn = db_conn.unwrap();

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

    let stmt = "SELECT holder_amount FROM tokens WHERE erc20_address = ? AND holder_address = ?";

    match db_conn.exec_first(stmt, (&token_address, &user_address)) {
        Ok(amount) => {
            let amount: Option<String> = amount;
            if amount.is_none() {
                log::warn!("No claim amount found for user: {}", user_address);
                return Ok(Json(TokenClaimData {
                    token_address,
                    user_address,
                    amount: "0".to_string(),
                }));
            }
            let amount = amount.unwrap();
            return Ok(Json(TokenClaimData {
                token_address,
                user_address,
                amount,
            }));
        }
        Err(err) => {
            log::error!("Failed to execute query: {:?}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}
