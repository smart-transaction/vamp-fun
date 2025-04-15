use anchor_lang::prelude::*;
// Errors
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid Merkle proof")]
    InvalidProof,
    #[msg("Token mint mismatch")]
    InvalidTokenMint,
}