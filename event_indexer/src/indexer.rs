use alloy::sol_types::SolEvent;
use serde::Serialize;
use std::{fmt::Debug, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{info, warn};

use anyhow::{Context, Result};
use sqlx::{MySql, MySqlPool, Row, Transaction};

use alloy_primitives::{Address, B256};
use alloy_rpc_types::{Filter, Log};

use crate::{app_state::AppState, eth_client::EthClient, events::VampTokenIntent};

pub async fn indexer_loop<Event>(state: AppState, contract: Address) -> Result<()>
where Event: Debug + SolEvent + Serialize {
    info!("indexer loop started, contract: {}, event: {}", contract, Event::SIGNATURE);

    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);
    let poll_interval = Duration::from_secs(state.cfg.poll_secs);
    let mut current_block = get_last_block(&state.db)
        .await?
        .saturating_sub(state.cfg.overlap_blocks);

    loop {
        match indexer_tick::<Event>(&state, current_block, contract).await {
            Ok(next_block) => {
                backoff = Duration::from_secs(1);
                current_block = next_block;
                sleep(poll_interval).await;
            }
            Err(err) => {
                warn!("indexer tick failed; backing off: {:#?}", err);
                sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
}

pub async fn indexer_tick<Event>(state: &AppState, last_block: u64, contract: Address) -> anyhow::Result<u64>
where Event: Debug + SolEvent + Serialize {
    // Determine finalized head (head - confirmations)
    let head = state.eth.provider.get_block_number().await?;
    if head <= state.cfg.confirmations {
        return Ok(state.cfg.deployment_block);
    }
    let finalized = head - state.cfg.confirmations;

    // Compute from/to with overlap and deployment_block
    let mut from = last_block;
    if from < state.cfg.deployment_block {
        from = state.cfg.deployment_block;
    }

    if from > finalized {
        // nothing new finalized
        return Ok(from);
    }

    // We do bounded ranges so eth_getLogs doesn't blow up on large spans.
    let mut current_from = from;

    let topic0 = VampTokenIntent::SIGNATURE_HASH;

    let mut current_to: u64 = state.cfg.deployment_block;
    while current_from <= finalized {
        current_to = (current_from + state.cfg.max_block_range - 1).min(finalized);

        let logs = fetch_logs(
            contract,
            topic0,
            state.eth.clone(),
            current_from,
            current_to,
        )
        .await?;
        if !logs.is_empty() {
            persist_logs_and_advance_checkpoint::<Event>(state, &logs).await?;
        } else {
            update_checkpoint_empty_log(&state.db, current_to).await?;
        }

        current_from = current_to + 1;
    }

    Ok(current_to + 1)
}

pub async fn fetch_logs(
    contract: Address,
    topic0: B256,
    eth: Arc<EthClient>,
    from: u64,
    to: u64,
) -> Result<Vec<Log>> {
    let filter = Filter::new()
        .address(contract)
        .event_signature(topic0)
        .from_block(from)
        .to_block(to);

    eth.provider.get_logs(&filter).await.context("getting logs")
}

pub async fn persist_logs_and_advance_checkpoint<Event>(
    state: &AppState,
    logs: &[Log],
) -> anyhow::Result<()>
where Event: Debug + SolEvent + Serialize {
    // Sort logs by (block_number, log_index) so checkpoint advancement is correct.
    let mut logs_sorted = logs.to_vec();
    logs_sorted.sort_by_key(|l| {
        let bn = l.block_number.unwrap_or_default();
        let li = l.log_index.unwrap_or_default();
        (bn, li)
    });

    let mut tx = state.db.begin().await.context("begin tx")?;

    // Insert idempotently
    for l in &logs_sorted {
        if !log_exists(
            &state.db,
            l.block_number.context("log missing block_number")?,
            l.log_index.context("log missing log_index")?,
        )
        .await?
        {
            state.publisher.publish::<Event>(&l.inner).await?;
            insert_event_idempotent(&mut tx, state.cfg.chain_id, l).await?;
            info!(
                l.block_number,
                l.log_index,
                "Publishing and storing the event"
            );
        }
    }

    // Advance checkpoint to the last log we successfully considered.
    // For robustness you might track last block/log_index even if duplicates.
    let last = logs_sorted.last().expect("non-empty");
    let last_block = last.block_number.context("log missing block_number")?;
    let last_log_index = last.log_index.context("log missing log_index")?;

    update_checkpoint(&mut tx, last_block, last_log_index).await?;

    tx.commit().await.context("commit tx")?;

    Ok(())
}

pub async fn insert_event_idempotent(
    tx: &mut Transaction<'_, MySql>,
    chain_id: u64,
    l: &Log,
) -> anyhow::Result<()> {
    // Extract essential fields
    let block_number = l.block_number.context("log missing block_number")?;
    let tx_hash = l.transaction_hash.context("log missing tx_hash")?;
    let log_index = l.log_index.context("log missing log_index")?;
    let block_hash = l.block_hash; // optional
    let address = l.address();
    let topic0 = l.topics().get(0).copied().context("log missing topic0")?;
    let data = l.data().data.clone();

    // Optional job_id extraction:
    // - If your event has indexed jobId, it may be in topics[1]
    // - If it’s non-indexed, you’ll decode from `data`
    // For now, we use topics[1] if present.
    let job_id: Option<B256> = l.topics().get(1).copied();

    // Use INSERT ... ON DUPLICATE KEY UPDATE to achieve idempotency.
    // We "do nothing" updates (keep existing row) by setting columns to themselves.
    sqlx::query(
        r#"
        INSERT INTO indexed_events (
          chain_id, block_number, block_hash,
          tx_hash, log_index,
          contract_address, topic0,
          job_id, data
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON DUPLICATE KEY UPDATE
          id = id
        "#,
    )
    .bind(chain_id as i64)
    .bind(block_number as i64)
    .bind(block_hash.map(|h| h.0.to_vec()))
    .bind(tx_hash.0.to_vec())
    .bind(log_index as i64)
    .bind(address.0.to_vec())
    .bind(topic0.0.to_vec())
    .bind(job_id.map(|j| j.0.to_vec()))
    .bind(data.to_vec())
    .execute(&mut **tx)
    .await
    .context("insert indexed_events")?;

    Ok(())
}

pub async fn ensure_checkpoint_row(db: &MySqlPool) -> anyhow::Result<()> {
    // Ensure row exists (id=1). In production, prefer migrations.
    sqlx::query(
        r#"
        INSERT INTO indexer_checkpoint (id, last_block, last_log_index)
        VALUES (1, 0, 0)
        ON DUPLICATE KEY UPDATE id = id
        "#,
    )
    .execute(db)
    .await
    .context("ensure checkpoint row")?;
    Ok(())
}

async fn get_last_block(db: &MySqlPool) -> anyhow::Result<u64> {
    let row = sqlx::query(
        r#"
        SELECT last_block as last_block
        FROM indexer_checkpoint
        WHERE id = 1
        "#,
    )
    .fetch_one(db)
    .await
    .context("get last block")?;

    Ok(row.get::<u64, usize>(0) as u64)
}

pub async fn update_checkpoint(
    tx: &mut Transaction<'_, MySql>,
    last_block: u64,
    last_log_index: u64,
) -> anyhow::Result<()> {
    // Monotonic update: only move forward.
    // This prevents accidental rewinds if something odd happens.
    sqlx::query(
        r#"
        UPDATE indexer_checkpoint
        SET
          last_block = GREATEST(last_block, ?),
          last_log_index = CASE
            WHEN last_block < ? THEN ?
            WHEN last_block = ? THEN GREATEST(last_log_index, ?)
            ELSE last_log_index
          END
        WHERE id = 1
        "#,
    )
    .bind(last_block as i64)
    .bind(last_block as i64)
    .bind(last_log_index as i64)
    .bind(last_block as i64)
    .bind(last_log_index as i64)
    .execute(&mut **tx)
    .await
    .context("update checkpoint")?;

    Ok(())
}

pub async fn update_checkpoint_empty_log(db: &MySqlPool, last_block: u64) -> anyhow::Result<()> {
    // Monotonic update: only move forward.
    // This prevents accidental rewinds if something odd happens.
    sqlx::query(
        r#"
        UPDATE indexer_checkpoint
        SET
          last_block = GREATEST(last_block, ?)
        WHERE id = 1
        "#,
    )
    .bind(last_block as i64)
    .execute(db)
    .await
    .context("update checkpoint for empty log")?;

    Ok(())
}

pub async fn log_exists(db: &MySqlPool, last_block: u64, last_log_index: u64) -> Result<bool> {
    let rows = sqlx::query(
        r#"
        SELECT id
        FROM indexed_events
        WHERE block_number = ?
          AND log_index = ?
        "#,
    )
    .bind(last_block)
    .bind(last_log_index)
    .fetch_all(db)
    .await
    .context("check if log exists")?;

    Ok(!rows.is_empty())
}
