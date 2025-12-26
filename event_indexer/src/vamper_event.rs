use alloy::sol;

sol! {
    #[derive(Debug)]
    event VampTokenIntent(
        uint256 chainId,
        uint256 blockNumber,
        bytes32 intentId,
        address caller,
        address token,
        string tokenName,
        string tokenSymbol,
        string tokenURI
    );
}
