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
    #[msg("Invalid Ethereum address provided.")]
    InvalidAddress,
}
