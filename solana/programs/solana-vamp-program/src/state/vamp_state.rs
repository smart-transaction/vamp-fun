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
    // Bonding curve parameters
    pub total_claimed: u64,          // Total number of tokens claimed so far
    pub reserve_balance: u64,        // Current reserve balance in lamports
    pub token_supply: u64,           // Current token supply
    pub curve_exponent: u64,         // Exponent for the bonding curve (e.g., 2 for quadratic)
    pub initial_price: u64,          // Initial price in lamports per token
    pub sol_vault: Pubkey,           // SOL vault account to hold collected SOL
}

#[account]
#[derive(InitSpace)]
pub struct ClaimState {
    pub is_claimed: bool,
}