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
    Validated,
    UnderExecution, 
    Executed,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredRequest {
    pub intent_id: String,
    pub sequence_id: u64,
    pub data: String,  // JSON format for readability
    pub proto_data: String,  // Hex-encoded Protobuf for gRPC
    pub state: RequestState,
}

impl Storage {
    const SEQUENCE_KEY: &'static str = "vamp:intents:global:sequence_id";
    const LAST_PROCESSED_BLOCK_HASH_KEY: &'static str = "vamp:intents:state:last_processed_block_by_chain_id";
    const REQUESTS_BY_INTENT_ID: &'static str = "vamp:intents:by_intent_id";
    const INTENT_ID_BY_SEQUENCE_ID: &'static str = "vamp:intents:by_sequence_id_to_intent_id";

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

    /// Fetches a request by its sequence_id
    pub async fn get_request_by_sequence_id(&self, sequence_id: u64) -> anyhow::Result<StoredRequest> {
        let intent_id = self.resolve_intent_id_from_sequence(sequence_id).await?
            .ok_or_else(|| anyhow::anyhow!("No intent_id found for sequence_id {}", sequence_id))?;

        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: String = conn.hget(Self::REQUESTS_BY_INTENT_ID, intent_id).await?;
        let request: StoredRequest = serde_json::from_str(&serialized)?;
        Ok(request)
    }
    
    async fn resolve_intent_id_from_sequence(&self, sequence_id: u64) -> anyhow::Result<Option<String>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let intent_id: Option<String> = conn.hget(Self::INTENT_ID_BY_SEQUENCE_ID, sequence_id.to_string()).await.ok();
        Ok(intent_id)
    }

    /// Stores a new intent in the 'NEW' state
    pub async fn save_new_intent(&self, intent_id: &str, sequence_id: u64, json_data: &str, proto_data: &str) -> 
                                                                                  anyhow::Result<()> {
        let stored_request = StoredRequest {
            intent_id: intent_id.to_string(),
            sequence_id,
            data: json_data.to_string(),
            proto_data: proto_data.to_string(),
            state: RequestState::New,
        };
        let serialized = serde_json::to_string(&stored_request)?;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let _: () = conn.hset(Self::REQUESTS_BY_INTENT_ID, intent_id, &serialized).await?;
        let _: () = conn.hset(Self::INTENT_ID_BY_SEQUENCE_ID, sequence_id.to_string(), intent_id).await?;
        Ok(())
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