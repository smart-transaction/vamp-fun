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
}
