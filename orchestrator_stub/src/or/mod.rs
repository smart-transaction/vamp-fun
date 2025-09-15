pub mod orch_grpc_service;

pub mod storage {
    pub use appchain_core::types::StoredRequest;
    use appchain_core::types::RequestState as State;
    use appchain_storage_redis::{RedisRequestStore, RequestStore};

    #[derive(Clone)]
    pub struct Storage {
        pub store: RedisRequestStore,
    }

    impl Storage {
        pub async fn new(cfg: &config::Config) -> anyhow::Result<Self> {
            let redis_url: String = cfg.get("storage.redis_url")?;
            let store = appchain_storage_redis::new_redis_store(&redis_url)?;
            Ok(Self { store })
        }

        pub async fn get_intent_in_state_new_or_validated(&self, sequence_id: u64) -> anyhow::Result<Option<StoredRequest>> {
            let intent_id = self.store.get_intent_id_by_sequence(sequence_id).await?;
            if let Some(intent_id) = intent_id {
                if let Some(request) = self.store.get_request_by_intent_id(&intent_id).await? {
                    match request.state {
                        State::New | State::Validated | State::UnderExecution => return Ok(Some(request)),
                        _ => {}
                    }
                }
            }
            Ok(None)
        }

        pub async fn update_request_state_to_under_execution(&self, sequence_id: u64) -> anyhow::Result<()> {
            let intent_id = self.store.get_intent_id_by_sequence(sequence_id).await?
                .ok_or_else(|| anyhow::anyhow!("No intent_id found for sequence_id {}", sequence_id))?;
            let _ = self.store.update_state_if(&intent_id, |s| matches!(s, State::New | State::Validated), State::UnderExecution).await?;
            Ok(())
        }
    }
}
