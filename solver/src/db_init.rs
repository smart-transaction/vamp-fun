// src/db_init.rs
//
// A small DB initializer for MySQL + SQLx.
// Runs at startup, ensures tables exist, and seeds the checkpoint row.
//
// Usage (in your main startup):
//   db_init::init_db(&pool).await?;
//
// Notes:
// - This is intentionally "lightweight migrations" (CREATE TABLE IF NOT EXISTS).
// - If you later need schema evolution, consider sqlx migrations.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use sqlx::MySqlPool;

use crate::{args::Args, mysql_conn::DbConn};

pub async fn init_db(args: Arc<Args>) -> Result<()> {
    let db_conn = DbConn::new(
        args.mysql_host.clone(),
        args.mysql_port.to_string(),
        args.mysql_user.clone(),
        args.mysql_password.clone(),
        args.mysql_database.clone(),
    );

    let pool = db_conn
        .create_db_conn()
        .await
        .map_err(|e| anyhow!("error creating DB connection: {}", e))?;

    create_epochs(&pool).await?;
    create_tokens(&pool).await?;
    create_clonings(&pool).await?;

    Ok(())
}

async fn create_epochs(db: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS epochs(
            chain_id BIGINT NOT NULL,
            block_number BIGINT NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            ts TIMESTAMP DEFAULT current_timestamp
        )
        "#,
    )
    .execute(db)
    .await
    .context("db_init: create table epochs")?;

    Ok(())
}

async fn create_tokens(db: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tokens(
            chain_id BIGINT NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            holder_address CHAR(42) NOT NULL,
            holder_amount VARCHAR(78) NOT NULL,
            signature VARCHAR(255),
            INDEX chain_id_idx(chain_id),
            INDEX erc20_address_idx(erc20_address),
            INDEX holder_address_idx(holder_address)
        )
        "#,
    )
    .execute(db)
    .await
    .context("db_init: create table indexed_events")?;

    Ok(())
}

async fn create_clonings(db: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS clonings(
            chain_id BIGINT NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            target_txid VARCHAR(128) NOT NULL,
            mint_account_address VARCHAR(128) NOT NULL,
            token_spl_address VARCHAR(128) NOT NULL,
            INDEX chain_id_idx(chain_id),
            INDEX erc20_address_idx(erc20_address)
        )
        "#,
    )
    .execute(db)
    .await
    .context("db_init: create table indexed_events")?;

    Ok(())
}
