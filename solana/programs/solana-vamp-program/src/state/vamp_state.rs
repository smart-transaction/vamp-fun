use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {
    pub bump: u8,          
    pub mint: Pubkey,
    #[max_len(65)]
    pub solver_public_key: Vec<u8>,
    #[max_len(65)]
    pub validator_public_key: Vec<u8>,
    pub vamp_identifier: u64,
}

#[account]
#[derive(InitSpace)]
pub struct ClaimState {
    pub is_claimed: bool,
}