use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {
    #[max_len(1000)] // Max number of token mappings
    pub token_mappings: Vec<TokenMapping>, // Tokem Napping
    pub authority: Pubkey,     // Admin address
    pub bump: u8,              // Bump for vault authority PDA
    pub mint: Pubkey,          // Token mint address (from fungible_token)
}

#[derive(Clone, Default, AnchorSerialize, AnchorDeserialize)]
pub struct TokenMapping {
    pub token_address: Pubkey,
    pub token_amount: u64,
    pub eth_address: [u8; 20],
    pub decimals: u8,
}

impl Space for TokenMapping {
    const INIT_SPACE: usize = 32 + 8 + 20; // Pubkey (32) + u64 (8) + eth_address (20)
}