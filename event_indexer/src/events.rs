use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[derive(Debug, Deserialize, Serialize)]
    event VampTokenIntent(
        uint64 chain_id,
        uint64 block_number,
        bytes32 intent_id,
        address caller,
        address token,
        string token_name,
        string token_symbol,
        string token_uri
    );

    #[derive(Debug, Deserialize, Serialize)]
    event ClaimToken(
        bytes32 intent_id,
        address claimer,
        uint256 amount,
        uint8 decimals,
        bytes owner_signature,
        bytes solver_signature,
        bytes validator_signature,
        bytes32 claimer_solana
    );
}
