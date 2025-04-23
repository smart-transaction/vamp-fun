use anchor_lang::prelude::*;
use prost::Message;

declare_id!("5zKTcVqXKk1vYGZpK47BvMo8fwtUrofroCdzSK931wVc");

// Module declarations
mod constants;
mod event;
mod instructions;
mod state;
mod use_proto;
mod util;

// Re-exports
pub use constants::*;
use event::ErrorCode;
use instructions::*;

use use_proto::vamp_fun::TokenVampingInfoProto;
use util::verify_merkle_root;

#[program]
pub mod solana_vamp_program {
    use crate::util::convert_token_mapping;

    use super::*;

    pub fn create_token_mint(ctx: Context<Initialize>, vamping_data: Vec<u8>) -> Result<()> {
        let vamping_info = TokenVampingInfoProto::decode(&vamping_data[..]).unwrap();
        let merkle_root: [u8; 32] = vamping_info.merkle_root[..]
            .try_into()
            .expect("Merkle root should be 32 bytes");

        let token_mapping_proto = vamping_info.token_mapping.unwrap_or_default();

        msg!("Token mapping accounts: {}", token_mapping_proto.addresses.len());
        msg!("Token mapping amounts: {}", token_mapping_proto.amounts.len());

        msg!("Decimal: {}", vamping_info.decimal);

        require!(
            token_mapping_proto.addresses.len() == token_mapping_proto.amounts.len(),
            ErrorCode::InvalidTokenMapping,
        );

        require!(
            verify_merkle_root(
                &token_mapping_proto,
                vamping_info.decimal as u8,
                &merkle_root
            )
            .map_err(|_| { ErrorCode::InvalidMerkleProof })?,
            ErrorCode::InvalidMerkleProof
        );

        let token_mapping = convert_token_mapping(&token_mapping_proto, vamping_info.decimal as u8)
            .map_err(|_| ErrorCode::InvalidTokenMapping)?;

        ctx.accounts.create_token_mint(
            token_mapping,
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
    pub fn claim(ctx: Context<Claim>, eth_address: [u8; 20], eth_signature: String) -> Result<()> {
        claim_tokens(ctx, eth_address, eth_signature)
    }
}
