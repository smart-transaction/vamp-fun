use anchor_lang::prelude::*;
use prost::Message;

declare_id!("CABA3ibLCuTDcTF4DQXuHK54LscXM5vBg7nWx1rzPaJH");

// Module declarations
mod event;
mod instructions;
mod state;
mod use_proto;
mod util;

// Re-exports
use event::TokenMintCreated;
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
        let token_mapping_proto = vamping_info.token_mapping.unwrap_or_default();

        require!(
            token_mapping_proto.addresses.len() == token_mapping_proto.amounts.len(),
            ErrorCode::InvalidTokenMapping,
        );

        let token_mapping = convert_token_mapping(&token_mapping_proto, vamping_info.decimal as u8)
            .map_err(|_| ErrorCode::InvalidTokenMapping)?;

        ctx.accounts.create_token_mint(
            token_mapping,
            vamping_info.token_name.clone(),
            vamping_info.token_symbol.clone(),
            vamping_info.token_uri.unwrap_or_default(),
            vamping_info.amount,
            vamping_info.decimal as u8,
            &ctx.bumps,
        )?;

        let token_name = vamping_info.token_name.clone();
        let token_symbol = vamping_info.token_symbol.clone();
        
        emit!(TokenMintCreated {
            mint_account: ctx.accounts.mint_account.key(),
            token_name,
            token_symbol,
            amount: vamping_info.amount
        });
        Ok(())
    }

    // TDOD: add logic to avoid double claim
    pub fn claim(ctx: Context<Claim>, amount: u64, eth_address: [u8; 20], eth_signature: [u8; 65]) -> Result<()> {
        claim_tokens(ctx, amount, eth_address, eth_signature)
    }
}
