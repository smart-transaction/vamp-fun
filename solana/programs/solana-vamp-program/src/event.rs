use anchor_lang::prelude::*;
// Errors
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Merkle proof")]
    InvalidProof,
    #[msg("Token mint mismatch")]
    InvalidTokenMint,
    #[msg("Invalid Merkle proof provided.")]
    InvalidMerkleProof,
    #[msg("Invalid Ethereum signature provided.")]
    InvalidSignature,
    #[msg("Invalid owner signature provided.")]
    InvalidOwnerSignature,
    #[msg("Invalid solver signature provided.")]
    InvalidSolverSignature,
    #[msg("Invalid validator signature provided.")]
    InvalidValidatorSignature,
    #[msg("Invalid Ethereum address provided.")]
    InvalidAddress,
    #[msg("Invalid token mapping provided.")]
    InvalidTokenMapping,
    #[msg("Tokens already claimed.")]
    TokensAlreadyClaimed,
    #[msg("Arithmetic overflow occurred.")]
    ArithmeticOverflow,
    #[msg("Cost per token exceeds the maximum allowed")]
    PriceTooHigh,
}

#[event]
pub struct TokenMintCreated {
    pub mint_account: Pubkey,
    pub token_name: String,
    pub token_symbol: String,
    pub token_erc20_address: String,
    pub amount: u64,
}