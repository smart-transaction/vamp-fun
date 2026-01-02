use alloy::sol;
use serde::{Deserialize, Serialize};

sol! {
    #[derive(Debug, Deserialize, Serialize)]
    event VampTokenIntent(
        uint64 chainId,
        uint64 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );
}
