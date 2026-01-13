use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaimToken {
    pub intent_id: FixedBytes<32>,
    pub claimer: Address,
    pub amount: U256,
    pub decimals: u8,
    pub owner_signature: Bytes,
    pub solver_signature: Bytes,
    pub validator_signature: Bytes,
    pub claimer_solana: FixedBytes<32>,
}
