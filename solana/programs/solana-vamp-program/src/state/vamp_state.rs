use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {
    pub merkle_root: [u8; 32], // Root of the Merkle tree
    pub authority: Pubkey,     // Admin address
    pub bump: u8,              // Bump for vault authority PDA
    pub mint: Pubkey,          // Token mint address (from fungible_token)
}