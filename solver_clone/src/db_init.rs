// src/db_init.rs
//
// Database migration system for MySQL + SQLx.
// Provides idempotent migrations with tracking and helper functions for schema operations.
//
// Usage (in your main startup):
//   db_init::init_db(&pool).await?;
//
// To add a new migration:
//   1. Add a new Migration to the migrations() function
//   2. Implement the migration logic using helper functions or raw SQL
//   3. The system will automatically detect and run pending migrations

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use sqlx::{MySqlPool, Row};
use tracing::{info, warn};

use crate::{args::Args, mysql_conn::create_db_conn};

/// Represents a single database migration
struct Migration {
    /// Unique identifier for this migration (e.g., "001_create_epochs_table")
    id: &'static str,
    /// Human-readable description of what this migration does
    description: &'static str,
    /// Function that applies the migration
    up: fn(
        &MySqlPool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>,
}

impl Migration {
    fn new(
        id: &'static str,
        description: &'static str,
        up: fn(
            &MySqlPool,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + '_>>,
    ) -> Self {
        Self {
            id,
            description,
            up,
        }
    }
}

/// Main entry point for database initialization
pub async fn init_db(args: Arc<Args>) -> Result<()> {
    let pool = create_db_conn(&args)
        .await
        .map_err(|e| anyhow!("error creating DB connection: {}", e))?;

    // Ensure migration tracking table exists
    ensure_migrations_table(&pool).await?;

    // Run all pending migrations
    run_migrations(&pool).await.context("Run migrations")?;

    Ok(())
}

/// Ensures the schema_migrations table exists
async fn ensure_migrations_table(db: &MySqlPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            id VARCHAR(255) PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            INDEX applied_at_idx(applied_at)
        )
        "#,
    )
    .execute(db)
    .await
    .context("Failed to create schema_migrations table")?;

    Ok(())
}

/// Checks if a migration has already been applied
async fn is_migration_applied(db: &MySqlPool, migration_id: &str) -> Result<bool> {
    let result = sqlx::query("SELECT COUNT(*) as count FROM schema_migrations WHERE id = ?")
        .bind(migration_id)
        .fetch_one(db)
        .await?;

    let count: i64 = result.try_get("count")?;
    Ok(count > 0)
}

/// Marks a migration as applied
async fn mark_migration_applied(
    db: &MySqlPool,
    migration_id: &str,
    description: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO schema_migrations (id, description) VALUES (?, ?)")
        .bind(migration_id)
        .bind(description)
        .execute(db)
        .await
        .context("Failed to mark migration as applied")?;

    Ok(())
}

/// Defines all migrations in order
fn migrations() -> Vec<Migration> {
    vec![
        Migration::new("001_create_epochs_table", "Create epochs table", |db| {
            Box::pin(async move { migration_001_create_epochs(db).await })
        }),
        Migration::new("002_create_tokens_table", "Create tokens table", |db| {
            Box::pin(async move { migration_002_create_tokens(db).await })
        }),
        Migration::new("003_create_clonings_table", "Create clonings table", |db| {
            Box::pin(async move { migration_003_create_clonings(db).await })
        }),
        Migration::new("004_add_intent_id_to_tokens", "Add intent ID to tokens", |db| {
            Box::pin(async move { migration_004_add_intent_id_to_tokens(db).await })
        }),
    ]
}

/// Runs all pending migrations
async fn run_migrations(db: &MySqlPool) -> Result<()> {
    let all_migrations = migrations();

    for migration in all_migrations {
        if is_migration_applied(db, migration.id).await? {
            info!("Migration {} already applied, skipping", migration.id);
            continue;
        }

        info!(
            "Running migration {}: {}",
            migration.id, migration.description
        );

        // Execute migration in a transaction
        let tx = db.begin().await?;

        match (migration.up)(db).await.context(format!("Migration {} failed", migration.id)) {
            Ok(_) => {
                mark_migration_applied(db, migration.id, migration.description).await?;
                tx.commit().await?;
                info!("Migration {} completed successfully", migration.id);
            }
            Err(e) => {
                tx.rollback().await?;
                return Err(e);
            }
        }
    }

    Ok(())
}

// ============================================================================
// Helper Functions for Schema Operations
// ============================================================================

/// Checks if a table exists in the database
async fn table_exists(db: &MySqlPool, table_name: &str) -> Result<bool> {
    let result = sqlx::query(
        "SELECT COUNT(*) as count FROM information_schema.tables 
         WHERE table_schema = DATABASE() AND table_name = ?",
    )
    .bind(table_name)
    .fetch_one(db)
    .await?;

    let count: i64 = result.try_get("count")?;
    Ok(count > 0)
}

/// Checks if a column exists in a table
async fn column_exists(db: &MySqlPool, table_name: &str, column_name: &str) -> Result<bool> {
    let result = sqlx::query(
        "SELECT COUNT(*) as count FROM information_schema.columns 
         WHERE table_schema = DATABASE() AND table_name = ? AND column_name = ?",
    )
    .bind(table_name)
    .bind(column_name)
    .fetch_one(db)
    .await?;

    let count: i64 = result.try_get("count")?;
    Ok(count > 0)
}

/// Checks if an index exists on a table
async fn index_exists(db: &MySqlPool, table_name: &str, index_name: &str) -> Result<bool> {
    let result = sqlx::query(
        "SELECT COUNT(*) as count FROM information_schema.statistics 
         WHERE table_schema = DATABASE() AND table_name = ? AND index_name = ?",
    )
    .bind(table_name)
    .bind(index_name)
    .fetch_one(db)
    .await?;

    let count: i64 = result.try_get("count")?;
    Ok(count > 0)
}

/// Adds a column to a table if it doesn't exist
async fn add_column_if_not_exists(
    db: &MySqlPool,
    table_name: &str,
    column_name: &str,
    column_definition: &str,
) -> Result<()> {
    if column_exists(db, table_name, column_name).await? {
        info!(
            "Column {}.{} already exists, skipping",
            table_name, column_name
        );
        return Ok(());
    }

    let query = format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        table_name, column_name, column_definition
    );
    sqlx::query(&query).execute(db).await.context(format!(
        "Failed to add column {}.{}",
        table_name, column_name
    ))?;

    info!("Added column {}.{}", table_name, column_name);
    Ok(())
}

/// Modifies a column in a table
async fn modify_column(
    db: &MySqlPool,
    table_name: &str,
    column_name: &str,
    new_definition: &str,
) -> Result<()> {
    let query = format!(
        "ALTER TABLE {} MODIFY COLUMN {} {}",
        table_name, column_name, new_definition
    );
    sqlx::query(&query).execute(db).await.context(format!(
        "Failed to modify column {}.{}",
        table_name, column_name
    ))?;

    info!("Modified column {}.{}", table_name, column_name);
    Ok(())
}

/// Drops a column from a table if it exists
async fn drop_column_if_exists(db: &MySqlPool, table_name: &str, column_name: &str) -> Result<()> {
    if !column_exists(db, table_name, column_name).await? {
        info!(
            "Column {}.{} does not exist, skipping drop",
            table_name, column_name
        );
        return Ok(());
    }

    let query = format!("ALTER TABLE {} DROP COLUMN {}", table_name, column_name);
    sqlx::query(&query).execute(db).await.context(format!(
        "Failed to drop column {}.{}",
        table_name, column_name
    ))?;

    info!("Dropped column {}.{}", table_name, column_name);
    Ok(())
}

/// Adds an index to a table if it doesn't exist
async fn add_index_if_not_exists(
    db: &MySqlPool,
    table_name: &str,
    index_name: &str,
    columns: &str,
) -> Result<()> {
    if index_exists(db, table_name, index_name).await? {
        info!(
            "Index {} on {} already exists, skipping",
            index_name, table_name
        );
        return Ok(());
    }

    let query = format!(
        "CREATE INDEX {} ON {} ({})",
        index_name, table_name, columns
    );
    sqlx::query(&query).execute(db).await.context(format!(
        "Failed to create index {} on {}",
        index_name, table_name
    ))?;

    info!("Created index {} on {}", index_name, table_name);
    Ok(())
}

/// Drops an index from a table if it exists
async fn drop_index_if_exists(db: &MySqlPool, table_name: &str, index_name: &str) -> Result<()> {
    if !index_exists(db, table_name, index_name).await? {
        info!(
            "Index {} on {} does not exist, skipping drop",
            index_name, table_name
        );
        return Ok(());
    }

    let query = format!("DROP INDEX {} ON {}", index_name, table_name);
    sqlx::query(&query).execute(db).await.context(format!(
        "Failed to drop index {} on {}",
        index_name, table_name
    ))?;

    info!("Dropped index {} on {}", index_name, table_name);
    Ok(())
}

/// Creates a table if it doesn't exist
async fn create_table_if_not_exists(
    db: &MySqlPool,
    table_name: &str,
    table_definition: &str,
) -> Result<()> {
    if table_exists(db, table_name).await? {
        info!("Table {} already exists, skipping creation", table_name);
        return Ok(());
    }

    let query = format!("CREATE TABLE {} {}", table_name, table_definition);
    sqlx::query(&query)
        .execute(db)
        .await
        .context(format!("Failed to create table {}", table_name))?;

    info!("Created table {}", table_name);
    Ok(())
}

/// Drops a table if it exists
async fn drop_table_if_exists(db: &MySqlPool, table_name: &str) -> Result<()> {
    if !table_exists(db, table_name).await? {
        info!("Table {} does not exist, skipping drop", table_name);
        return Ok(());
    }

    let query = format!("DROP TABLE {}", table_name);
    sqlx::query(&query)
        .execute(db)
        .await
        .context(format!("Failed to drop table {}", table_name))?;

    warn!("Dropped table {}", table_name);
    Ok(())
}

// ============================================================================
// Migration Implementations
// ============================================================================

/// Migration 001: Create epochs table
async fn migration_001_create_epochs(db: &MySqlPool) -> Result<()> {
    create_table_if_not_exists(
        db,
        "epochs",
        r#"(
            chain_id BIGINT UNSIGNED NOT NULL,
            block_number BIGINT UNSIGNED NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .await
}

/// Migration 002: Create tokens table
async fn migration_002_create_tokens(db: &MySqlPool) -> Result<()> {
    create_table_if_not_exists(
        db,
        "tokens",
        r#"(
            chain_id BIGINT NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            holder_address CHAR(42) NOT NULL,
            holder_amount VARCHAR(78) NOT NULL,
            signature VARCHAR(255),
            ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            INDEX chain_id_idx(chain_id),
            INDEX erc20_address_idx(erc20_address),
            INDEX holder_address_idx(holder_address)
        )"#,
    )
    .await
}

/// Migration 003: Create clonings table
async fn migration_003_create_clonings(db: &MySqlPool) -> Result<()> {
    create_table_if_not_exists(
        db,
        "clonings",
        r#"(
            chain_id BIGINT UNSIGNED NOT NULL,
            erc20_address CHAR(42) NOT NULL,
            target_txid VARCHAR(128) NOT NULL,
            mint_account_address VARCHAR(128) NOT NULL,
            token_spl_address VARCHAR(128) NOT NULL,
            intent_id VARCHAR(255) NOT NULL,
            ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            INDEX chain_id_idx(chain_id),
            INDEX erc20_address_idx(erc20_address)
        )"#,
    )
    .await
}

/// Migration 004: Add an intent_id to tokens table
async fn migration_004_add_intent_id_to_tokens(db: &MySqlPool) -> Result<()> {
    let col_res = add_column_if_not_exists(
        db,
        "tokens",
        "intent_id",
        "VARCHAR(255) NOT NULL DEFAULT ''",
    )
    .await;
    let idx_res = add_index_if_not_exists(
        db,
        "tokens",
        "idx_intent_id",
        "intent_id"
    ).await;
    if col_res.is_err() {
        return col_res;
    } else if idx_res.is_err() {
        return idx_res;
    }
    Ok(())
}
