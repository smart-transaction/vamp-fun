use redis::AsyncCommands;
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    client: Arc<redis::Client>,
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
    pub data: String,
    pub state: RequestState,
}

impl Storage {
    const REQUESTS_BY_INTENT_ID: &'static str = "vamp:intents:by_intent_id";
    const INTENT_ID_BY_SEQUENCE_ID: &'static str = "vamp:intents:sequence_id_to_intent_id";

    pub async fn new(cfg: &config::Config) -> anyhow::Result<Self> {
        let redis_url: String = cfg.get("storage.redis_url")?;
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client: Arc::new(client) })
    }

    pub async fn get_intent_in_state_new(&self, sequence_id: u64) -> 
                                                           anyhow::Result<Option<StoredRequest>> {
        let intent_id = self.resolve_intent_id_from_sequence(sequence_id).await?;
        if let Some(intent_id) = intent_id {
            log::debug!(
                    "Found intent_id: {} by sequence_id: {}. Checking the according intent...",
                    intent_id,
                    sequence_id
                );
            let mut conn = self.client.get_multiplexed_async_connection().await?;
            let serialized: Option<String> = conn.hget(Self::REQUESTS_BY_INTENT_ID, &intent_id).await.ok();

            if let Some(data) = serialized {
                log::debug!(
                    "Found intent by intent_id: {}. Checking the propper State...",
                    intent_id,
                );
                let request: StoredRequest = serde_json::from_str(&data)?;
                if let RequestState::New = request.state {
                    return Ok(Some(request));
                }
            }
        }

        Ok(None)
    }

    pub async fn get_intent_in_state_new_or_validated(&self, sequence_id: u64) -> 
                                                           anyhow::Result<Option<StoredRequest>> {
        let intent_id = self.resolve_intent_id_from_sequence(sequence_id).await?;
        if let Some(intent_id) = intent_id {
            log::debug!(
                    "Found intent_id: {} by sequence_id: {}. Checking the according intent...",
                    intent_id,
                    sequence_id
                );
            let mut conn = self.client.get_multiplexed_async_connection().await?;
            let serialized: Option<String> = conn.hget(Self::REQUESTS_BY_INTENT_ID, &intent_id).await.ok();

            if let Some(data) = serialized {
                log::debug!(
                    "Found intent by intent_id: {}. Checking the propper State...",
                    intent_id,
                );
                let request: StoredRequest = serde_json::from_str(&data)?;
                match request.state {
                    RequestState::New | RequestState::Validated => {
                        return Ok(Some(request));
                    }
                    _ => {
                        log::debug!("Intent {} is in state {:?}, not New or Validated", intent_id, request.state);
                    }
                }
            }
        }

        Ok(None)
    }
    
    pub async fn update_request_state_to_under_execution(&self, sequence_id: u64) -> anyhow::Result<()> {
        let intent_id = self.resolve_intent_id_from_sequence(sequence_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No intent_id found for sequence_id {}", sequence_id))?;

        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: String = conn.hget(Self::REQUESTS_BY_INTENT_ID, &intent_id).await?;
        let mut request: StoredRequest = serde_json::from_str(&serialized)?;

        request.state = RequestState::UnderExecution;

        let updated_serialized = serde_json::to_string(&request)?;
        let _: () = conn.hset(Self::REQUESTS_BY_INTENT_ID, &intent_id, updated_serialized).await?;

        Ok(())
    }

    async fn resolve_intent_id_from_sequence(&self, sequence_id: u64) -> anyhow::Result<Option<String>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let intent_id: Option<String> = conn.hget(Self::INTENT_ID_BY_SEQUENCE_ID, sequence_id.to_string()).await.ok();
        Ok(intent_id)
    }
    
}
