use appchain_core::types::{StoredRequest, RequestState};
use appchain_storage_redis::{RequestStore, RedisRequestStore};
use crate::validator_vamp::config::StorageConfig;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VampSolutionValidatedDetails {
    pub solver_pubkey: String,
    pub root_cid: String,
}

#[derive(Clone)]
pub struct Storage {
    pub store: RedisRequestStore,
}

impl Storage {
    pub async fn new(cfg: &StorageConfig) -> anyhow::Result<Self> {
        let redis_url: String = cfg.redis_url.clone();
        let store = appchain_storage_redis::new_redis_store(&redis_url)?;
        Ok(Self { store })
    }

    pub async fn get_intent_in_state_new(&self, intent_id: &str) -> anyhow::Result<Option<StoredRequest>> {
        if let Some(req) = self.store.get_request_by_intent_id(intent_id).await? {
            if let RequestState::New = req.state { return Ok(Some(req)); }
        }
        Ok(None)
    }

    pub async fn update_request_state_to_validated(
        &self,
        intent_id: &str,
        solver_pubkey: &str,
        root_cid: &str,
    ) -> anyhow::Result<()> {
        self.store.update_raw_json(intent_id, |mut json| {
            // Update state and attach vamp details while preserving unknown fields
            if let Some(obj) = json.as_object_mut() {
                obj.insert("state".to_string(), serde_json::json!("Validated"));
                obj.insert(
                    "vamp_solution_validated_details".to_string(),
                    serde_json::json!({
                        "solver_pubkey": solver_pubkey,
                        "root_cid": root_cid,
                    }),
                );
            }
            Ok(json)
        }).await
    }
}
