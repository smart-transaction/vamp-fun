use anchor_lang::prelude::*;
use prost::Message;

declare_id!("GxTHHX45PDeqMuystBAwC2vne6FacJNA4JeAC2JJc9hB");

// Module declarations
mod constants;
mod event;
mod instructions;
mod state;
mod util;

// Re-exports
pub use constants::*;
use instructions::*;
use event::ErrorCode;

// Proto definitions
pub mod vamp_fun {
    include!(concat!(env!("OUT_DIR"), "/vamp.fun.rs"));
}

use vamp_fun::TokenVampingInfoProto;
use state::vamp_state::TokenMapping;
use util::generate_merkle_root;

#[program]
pub mod solana_vamp_program {
    use super::*;

    pub fn create_token_mint(ctx: Context<Initialize>, vamping_data: Vec<u8>, vamp_fun_data: Vec<TokenMapping>) -> Result<()> {
        let vamping_info = TokenVampingInfoProto::decode(&vamping_data[..]).unwrap();
        let merkle_root: [u8; 32] = vamping_info.merkle_root[..]
            .try_into()
            .expect("Merkle root should be 32 bytes");

        let generated_root = generate_merkle_root(&vamp_fun_data);
        require!(merkle_root == generated_root, ErrorCode::InvalidMerkleProof);

        ctx.accounts.create_token_mint(
            vamp_fun_data,
            vamping_info.token_name,
            vamping_info.token_symbol,
            vamping_info.token_uri.unwrap_or_default(),
            vamping_info.amount,
            vamping_info.decimal as u8,
            &ctx.bumps,
        )?;

        Ok(())
    }

    // TDOD: add logic to avoid double claim
    pub fn claim(
        ctx: Context<Claim>,
        eth_address: [u8; 20],
        eth_signature: String,
    ) -> Result<()> {
        claim_tokens(ctx, eth_address, eth_signature)
    }
}
