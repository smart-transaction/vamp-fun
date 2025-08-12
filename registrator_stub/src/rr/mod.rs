pub mod ethereum_listener;
pub mod rr_grpc_service;

// Re-export a lightweight storage facade that uses the shared Redis store
pub mod storage {
    pub use appchain_core::types::StoredRequest;
    use appchain_storage_redis::{RedisRequestStore, RequestStore};

    #[derive(Clone)]
    pub struct Storage {
        pub store: RedisRequestStore,
        pub chain_id: u64,
    }

    impl Storage {
        pub async fn new(cfg: &config::Config, chain_id: u64) -> anyhow::Result<Self> {
            let redis_url: String = cfg.get("storage.redis_url")?;
            let store = appchain_storage_redis::new_redis_store(&redis_url)?;
            Ok(Self { store, chain_id })
        }

        pub async fn next_sequence_id(&self) -> anyhow::Result<u64> {
            self.store.next_sequence_id().await
        }

        pub async fn get_request_by_sequence_id(&self, sequence_id: u64) -> anyhow::Result<StoredRequest> {
            self.store.get_request_by_sequence_id(sequence_id).await
        }

        pub async fn save_new_intent(&self, intent_id: &str, sequence_id: u64, json_data: &str, proto_data: &str) -> anyhow::Result<()> {
            appchain_storage_redis::save_new_intent(&self.store, intent_id, sequence_id, json_data, Some(proto_data)).await
        }

        pub async fn get_last_processed_block(&self) -> anyhow::Result<u64> {
            self.store.get_last_processed_block(self.chain_id).await
        }

        pub async fn set_last_processed_block(&self, block_number: u64) -> anyhow::Result<()> {
            self.store.set_last_processed_block(self.chain_id, block_number).await
        }
    }
}
