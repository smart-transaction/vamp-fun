use redis::AsyncCommands;
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    client: Arc<redis::Client>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum RequestState {
    New,
    UnderExecution,
    Executed,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredRequest {
    pub sequence_id: u64,
    pub data: String,
    pub state: RequestState,
}

impl Storage {
    const REQUESTS_HASH_KEY: &'static str = "vamp:orchestrator:requests_by_sequence_id";

    pub async fn new(cfg: &config::Config) -> anyhow::Result<Self> {
        let redis_url: String = cfg.get("storage.redis_url")?;
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client: Arc::new(client) })
    }

    pub async fn get_new_request(&self, sequence_id: u64) -> anyhow::Result<Option<StoredRequest>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: Option<String> = conn.hget(Self::REQUESTS_HASH_KEY, sequence_id.to_string()).await.ok();

        if let Some(data) = serialized {
            let request: StoredRequest = serde_json::from_str(&data)?;
            if let RequestState::New = request.state {
                return Ok(Some(request));
            }
        }
        Ok(None)
    }

    pub async fn update_request_state_to_under_execution(&self, sequence_id: u64) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: String = conn.hget(Self::REQUESTS_HASH_KEY, sequence_id.to_string()).await?;
        let mut request: StoredRequest = serde_json::from_str(&serialized)?;

        request.state = RequestState::UnderExecution;

        let updated_serialized = serde_json::to_string(&request)?;
        let _: () = conn.hset(Self::REQUESTS_HASH_KEY, sequence_id.to_string(), updated_serialized).await?;

        Ok(())
    }
}
