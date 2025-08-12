pub mod types {
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
    pub enum RequestState {
        New,
        Validated,
        UnderExecution,
        Executed,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    pub struct StoredRequest {
        pub intent_id: String,
        pub sequence_id: u64,
        pub data: String,
        pub proto_data: Option<String>,
        pub state: RequestState,
        #[serde(default)]
        pub schema_version: Option<u32>,
    }
}

pub mod keys {
    pub const REQUESTS_BY_INTENT_ID: &str = "vamp:intents:by_intent_id";
    pub const INTENT_ID_BY_SEQUENCE_ID: &str = "vamp:intents:sequence_id_to_intent_id";
    pub const SEQUENCE_KEY: &str = "vamp:intents:global:sequence_id";
    pub const LAST_PROCESSED_BLOCK_HASH_KEY: &str = "vamp:intents:state:last_processed_block_by_chain_id";
} 