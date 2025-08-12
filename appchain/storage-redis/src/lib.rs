use std::sync::Arc;
use anyhow::Context;
use async_trait::async_trait;
use redis::AsyncCommands;

use appchain_core::{keys, types::{StoredRequest, RequestState}};

#[async_trait]
pub trait RequestStore: Send + Sync {
    async fn next_sequence_id(&self) -> anyhow::Result<u64>;
    async fn put_request(&self, request: &StoredRequest) -> anyhow::Result<()>;
    async fn get_request_by_intent_id(&self, intent_id: &str) -> anyhow::Result<Option<StoredRequest>>;
    async fn map_sequence_to_intent(&self, sequence_id: u64, intent_id: &str) -> anyhow::Result<()>;
    async fn get_intent_id_by_sequence(&self, sequence_id: u64) -> anyhow::Result<Option<String>>;
    async fn get_last_processed_block(&self, chain_id: u64) -> anyhow::Result<u64>;
    async fn set_last_processed_block(&self, chain_id: u64, block_number: u64) -> anyhow::Result<()>;

    async fn get_request_by_sequence_id(&self, sequence_id: u64) -> anyhow::Result<StoredRequest> {
        let intent_id = self.get_intent_id_by_sequence(sequence_id).await?
            .ok_or_else(|| anyhow::anyhow!("No intent_id found for sequence_id {}", sequence_id))?;
        let Some(req) = self.get_request_by_intent_id(&intent_id).await? else {
            anyhow::bail!("Intent {} not found", intent_id);
        };
        Ok(req)
    }

    async fn update_state_if<F>(&self, intent_id: &str, predicate: F, new_state: RequestState) -> anyhow::Result<bool>
    where F: Fn(&RequestState) -> bool + Send;

    async fn update_raw_json<F>(&self, intent_id: &str, updater: F) -> anyhow::Result<()>
    where F: FnOnce(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send;
}

#[derive(Clone)]
pub struct RedisRequestStore {
    client: Arc<redis::Client>,
}

impl RedisRequestStore {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url.to_string())?;
        Ok(Self { client: Arc::new(client) })
    }
}

#[async_trait]
impl RequestStore for RedisRequestStore {
    async fn next_sequence_id(&self) -> anyhow::Result<u64> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let sequence_number: u64 = conn.incr(keys::SEQUENCE_KEY, 1).await?;
        Ok(sequence_number)
    }

    async fn put_request(&self, request: &StoredRequest) -> anyhow::Result<()> {
        let serialized = serde_json::to_string(request)?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(keys::REQUESTS_BY_INTENT_ID, &request.intent_id, &serialized).await?;
        let _: () = conn.hset(keys::INTENT_ID_BY_SEQUENCE_ID, request.sequence_id.to_string(), &request.intent_id).await?;
        Ok(())
    }

    async fn get_request_by_intent_id(&self, intent_id: &str) -> anyhow::Result<Option<StoredRequest>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: Option<String> = conn.hget(keys::REQUESTS_BY_INTENT_ID, intent_id).await.ok();
        if let Some(data) = serialized {
            let request: StoredRequest = serde_json::from_str(&data)?;
            return Ok(Some(request));
        }
        Ok(None)
    }

    async fn map_sequence_to_intent(&self, sequence_id: u64, intent_id: &str) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(keys::INTENT_ID_BY_SEQUENCE_ID, sequence_id.to_string(), intent_id).await?;
        Ok(())
    }

    async fn get_intent_id_by_sequence(&self, sequence_id: u64) -> anyhow::Result<Option<String>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let intent_id: Option<String> = conn.hget(keys::INTENT_ID_BY_SEQUENCE_ID, sequence_id.to_string()).await.ok();
        Ok(intent_id)
    }

    async fn get_last_processed_block(&self, chain_id: u64) -> anyhow::Result<u64> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let block: Option<u64> = conn.hget(keys::LAST_PROCESSED_BLOCK_HASH_KEY, chain_id.to_string()).await.ok();
        Ok(block.unwrap_or(0))
    }

    async fn set_last_processed_block(&self, chain_id: u64, block_number: u64) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(keys::LAST_PROCESSED_BLOCK_HASH_KEY, chain_id.to_string(), block_number).await?;
        Ok(())
    }

    async fn update_state_if<F>(&self, intent_id: &str, predicate: F, new_state: RequestState) -> anyhow::Result<bool>
    where F: Fn(&RequestState) -> bool + Send {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: Option<String> = conn.hget(keys::REQUESTS_BY_INTENT_ID, intent_id).await?;
        let Some(data) = serialized else { return Ok(false); };
        let mut request: StoredRequest = serde_json::from_str(&data)?;
        if !predicate(&request.state) { return Ok(false); }
        request.state = new_state;
        let updated = serde_json::to_string(&request)?;
        let _: () = conn.hset(keys::REQUESTS_BY_INTENT_ID, intent_id, updated).await?;
        Ok(true)
    }

    async fn update_raw_json<F>(&self, intent_id: &str, updater: F) -> anyhow::Result<()>
    where F: FnOnce(serde_json::Value) -> anyhow::Result<serde_json::Value> + Send {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: Option<String> = conn.hget(keys::REQUESTS_BY_INTENT_ID, intent_id).await
            .with_context(|| format!("Failed hget for intent_id {}", intent_id))?;
        let Some(data) = serialized else { anyhow::bail!("Intent not found for ID: {}", intent_id); };
        let json: serde_json::Value = serde_json::from_str(&data)?;
        let updated_json = updater(json)?;
        let updated_str = serde_json::to_string(&updated_json)?;
        let _: () = conn.hset(keys::REQUESTS_BY_INTENT_ID, intent_id, updated_str).await?;
        Ok(())
    }
}

// Helper constructor wrapper used by services
pub fn new_redis_store(redis_url: &str) -> anyhow::Result<RedisRequestStore> {
    RedisRequestStore::new(redis_url)
}

// Convenience helpers tailored to common flows
pub async fn save_new_intent<S: RequestStore>(store: &S, intent_id: &str, sequence_id: u64, json_data: &str, proto_data_hex: Option<&str>) -> anyhow::Result<()> {
    let stored_request = StoredRequest {
        intent_id: intent_id.to_string(),
        sequence_id,
        data: json_data.to_string(),
        proto_data: proto_data_hex.map(|s| s.to_string()),
        state: RequestState::New,
        schema_version: Some(1),
    };
    store.put_request(&stored_request).await
} 