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
    const REQUEST_KEY_PREFIX: &'static str = "vamp:orchestrator:request:";

    pub async fn new(cfg: &config::Config) -> anyhow::Result<Self> {
        let redis_url: String = cfg.get("storage.redis_url")?;
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client: Arc::new(client) })
    }

    pub async fn get_new_request(&self, sequence_id: u64) -> anyhow::Result<Option<StoredRequest>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let key = Self::request_key(sequence_id);
        let serialized: Option<String> = conn.get(&key).await.ok();

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
        let key = Self::request_key(sequence_id);

        let serialized: String = conn.get(&key).await?;
        let mut request: StoredRequest = serde_json::from_str(&serialized)?;

        request.state = RequestState::UnderExecution;

        let updated_serialized = serde_json::to_string(&request)?;
        let _: () = conn.set(key, updated_serialized).await?;

        Ok(())
    }

    #[inline]
    fn request_key(sequence_id: u64) -> String {
        format!("{}{}", Self::REQUEST_KEY_PREFIX, sequence_id)
    }
}
