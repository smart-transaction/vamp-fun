use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VampState {
    #[max_len(1000)] // Max number of token mappings
    pub token_mappings: Vec<TokenMapping>, // Root of the Merkle tree
    pub authority: Pubkey,     // Admin address
    pub bump: u8,              // Bump for vault authority PDA
    pub mint: Pubkey,          // Token mint address (from fungible_token)
}

#[derive(Clone, AnchorSerialize, AnchorDeserialize)]
pub struct TokenMapping {
    pub token_address: Pubkey,
    pub token_amount: u64,
    pub eth_address: [u8; 20],
}

impl Space for TokenMapping {
    const INIT_SPACE: usize = 32 + 8 + 20; // Pubkey (32) + u64 (8) + eth_address (20)
}