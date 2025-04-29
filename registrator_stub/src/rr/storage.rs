use redis::AsyncCommands;
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    client: Arc<redis::Client>,
    chain_id: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum RequestState {
    New,
    UnderExecution(String), // solver_id
    Executed(String), // solver_id,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredRequest {
    pub sequence_id: u64,
    pub data: String,
    pub state: RequestState,
}

impl Storage {
    const SEQUENCE_KEY: &'static str = "vamp:intents:global:sequence_id";
    const LAST_PROCESSED_BLOCK_HASH_KEY: &'static str = "vamp:intents:state:last_processed_block_by_chain_id";
    const REQUESTS_HASH_KEY: &'static str = "vamp:intents:by_sequence_id";

    pub async fn new(cfg: &config::Config, chain_id: u64) -> anyhow::Result<Self> {
        let redis_url: String = cfg.get("storage.redis_url")?;
        log::info!("Connecting to Redis at {}", redis_url);
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client: Arc::new(client), chain_id })
    }

    /// Atomically generates a sequential sequence_id using Redis INCR command
    pub async fn next_sequence_id(&self) -> anyhow::Result<u64> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let sequence_number: u64 = conn.incr(Self::SEQUENCE_KEY, 1).await?;
        Ok(sequence_number)
    }

    /// Stores a new request in the 'NEW' state
    pub async fn save_new_request(&self, sequence_id: &u64, data: &str) -> anyhow::Result<()> {
        let stored_request = StoredRequest {
            sequence_id: *sequence_id,
            data: data.to_string(),
            state: RequestState::New,
        };

        let serialized = serde_json::to_string(&stored_request)?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(Self::REQUESTS_HASH_KEY, sequence_id.to_string(), serialized).await?;
        Ok(())
    }

    /// Fetches a request by its sequence_id
    pub async fn get_request_by_sequence_id(&self, sequence_id: &u64) -> anyhow::Result<StoredRequest> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: String = conn.hget(Self::REQUESTS_HASH_KEY, sequence_id.to_string()).await?;
        let request: StoredRequest = serde_json::from_str(&serialized)?;
        Ok(request)
    }

    /// Gets the last processed Ethereum block for the configured chain
    pub async fn get_last_processed_block(&self) -> anyhow::Result<u64> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let block: Option<u64> = conn.hget(Self::LAST_PROCESSED_BLOCK_HASH_KEY, self.chain_id.to_string()).await.ok();
        Ok(block.unwrap_or(0))
    }

    /// Sets the last processed Ethereum block for the configured chain
    pub async fn set_last_processed_block(&self, block_number: u64) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(Self::LAST_PROCESSED_BLOCK_HASH_KEY, self.chain_id.to_string(), block_number).await?;
        Ok(())
    }
}