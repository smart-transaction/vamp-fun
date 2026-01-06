use alloy_primitives::{Address, FixedBytes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct VampTokenIntent {
    pub chain_id: u64,
    pub block_number: u64,
    pub intent_id: FixedBytes<32>,
    pub vamp_initiator: Address,
    pub token: Address,
    pub token_name: String,
    pub token_symbol: String,
    pub token_uri: String,
}
