use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {    // Admin address
    pub bump: u8,              // Bump for vault authority PDA
    pub mint: Pubkey,          // Token mint address (from fungible_token)
}

#[account]
#[derive(InitSpace)]
pub struct ClaimState {
    pub is_claimed: bool,
}