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

use anyhow::{Context, Result};
use sqlx::{MySql, MySqlPool, Transaction};

pub async fn init_db(pool: &MySqlPool) -> Result<()> {
    // Make it atomic-ish: if something fails halfway, we rollback.
    let mut tx = pool.begin().await.context("db_init: begin tx")?;

    create_indexer_checkpoint(&mut tx).await?;
    seed_checkpoint_row(&mut tx).await?;
    create_indexed_events(&mut tx).await?;

    tx.commit().await.context("db_init: commit tx")?;
    Ok(())
}

async fn create_indexer_checkpoint(tx: &mut Transaction<'_, MySql>) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS indexer_checkpoint (
          id TINYINT NOT NULL PRIMARY KEY,
          last_block BIGINT UNSIGNED NOT NULL,
          last_log_index INT UNSIGNED NOT NULL,
          updated_at TIMESTAMP NOT NULL
            DEFAULT CURRENT_TIMESTAMP
            ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut **tx)
    .await
    .context("db_init: create table indexer_checkpoint")?;

    Ok(())
}

async fn seed_checkpoint_row(tx: &mut Transaction<'_, MySql>) -> Result<()> {
    // Ensure the single row (id=1) exists. Idempotent.
    sqlx::query(
        r#"
        INSERT INTO indexer_checkpoint (id, last_block, last_log_index)
        VALUES (1, 0, 0)
        ON DUPLICATE KEY UPDATE id = id
        "#,
    )
    .execute(&mut **tx)
    .await
    .context("db_init: seed indexer_checkpoint row")?;

    Ok(())
}

async fn create_indexed_events(tx: &mut Transaction<'_, MySql>) -> Result<()> {
    // Note: UNIQUE KEY uq_job_id (job_id) allows multiple NULLs in MySQL.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS indexed_events (
          id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,

          chain_id BIGINT UNSIGNED NULL,
          block_number BIGINT UNSIGNED NOT NULL,
          block_hash VARBINARY(32) NULL,

          tx_hash VARBINARY(32) NOT NULL,
          log_index INT UNSIGNED NOT NULL,

          contract_address VARBINARY(20) NOT NULL,
          topic0 VARBINARY(32) NOT NULL,

          job_id VARBINARY(32) NULL,
          data LONGBLOB NOT NULL,

          created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

          UNIQUE KEY uq_tx_log (tx_hash, log_index),
          UNIQUE KEY uq_job_id (job_id)
        )
        "#,
    )
    .execute(&mut **tx)
    .await
    .context("db_init: create table indexed_events")?;

    Ok(())
}
