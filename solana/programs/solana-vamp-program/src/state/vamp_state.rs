use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {
    pub bump: u8,          
    pub mint: Pubkey,
    #[max_len(20)]
    pub solver_public_key: Vec<u8>,
    #[max_len(20)]
    pub validator_public_key: Vec<u8>,
    pub vamp_identifier: u64,
    #[max_len(32)]
    pub intent_id: Vec<u8>,
}

#[account]
#[derive(InitSpace)]
pub struct ClaimState {
    pub is_claimed: bool,
}