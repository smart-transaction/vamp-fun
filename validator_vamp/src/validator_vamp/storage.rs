use redis::AsyncCommands;
use std::sync::Arc;
use crate::validator_vamp::config::StorageConfig;

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
    pub vamp_solution_validated_details: VampSolutionValidatedDetails,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VampSolutionValidatedDetails {
    pub solver_pubkey: String,
    pub root_cid: String,
}

impl Storage {
    const REQUESTS_BY_INTENT_ID: &'static str = "vamp:intents:by_intent_id";
    // const INTENT_ID_BY_SEQUENCE_ID: &'static str = "vamp:intents:by_sequence_id_to_intent_id";

    pub async fn new(cfg: &StorageConfig) -> anyhow::Result<Self> {
        let redis_url: String = cfg.redis_url.clone();
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client: Arc::new(client) })
    }

    pub async fn get_intent_in_state_new(&self, intent_id: &str) ->
    anyhow::Result<Option<StoredRequest>> {
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

        Ok(None)
    }

    pub async fn update_request_state_to_validated(
        &self,
        intent_id: &str,
        solver_pubkey: &str,
        root_cid: &str,
    ) -> anyhow::Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let serialized: Option<String> = conn.hget(Self::REQUESTS_BY_INTENT_ID, intent_id).await?;
        let Some(data) = serialized else {
            anyhow::bail!("Intent not found for ID: {}", intent_id);
        };

        let mut request: StoredRequest = serde_json::from_str(&data)?;
        request.state = RequestState::Validated;
        request.vamp_solution_validated_details = VampSolutionValidatedDetails {
            solver_pubkey: solver_pubkey.to_string(),
            root_cid: root_cid.to_string(),
        };

        let updated = serde_json::to_string(&request)?;
        let _: () = conn.hset(Self::REQUESTS_BY_INTENT_ID, intent_id, updated).await?;
        Ok(())
    }
}
