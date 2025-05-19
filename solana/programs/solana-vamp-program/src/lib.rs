use anchor_lang::prelude::*;
use prost::Message;

declare_id!("2tpiLJn7K3PcsGbT3A5MReKMcaLzBxK65cxapmHFJMom");

// Module declarations
mod constant;
mod event;
mod instructions;
mod state;
mod use_proto;

// Re-exports
use event::ErrorCode;
use event::TokenMintCreated;
use instructions::*;

use use_proto::vamp_fun::TokenVampingInfoProto;

#[program]
pub mod solana_vamp_program {
    use super::*;
    use hex::ToHex;

    pub fn create_token_mint(
        ctx: Context<Initialize>,
        vamp_identifier: u64,
        vamping_data: Vec<u8>,
    ) -> Result<()> {
        let vamping_info = TokenVampingInfoProto::decode(&vamping_data[..]).unwrap();
        let token_mapping_proto = vamping_info.token_mapping.unwrap_or_default();

        require!(
            token_mapping_proto.addresses.len() == token_mapping_proto.amounts.len(),
            ErrorCode::InvalidTokenMapping,
        );

        ctx.accounts.create_token_mint(
            vamp_identifier,
            vamping_info.token_name.clone(),
            vamping_info.token_symbol.clone(),
            vamping_info.token_uri.unwrap_or_default(),
            vamping_info.amount,
            vamping_info.decimal as u8,
            vamping_info.solver_public_key,
            vamping_info.validator_public_key,
            vamping_info.intent_id,
            &ctx.bumps,
        )?;

        let token_name = vamping_info.token_name.clone();
        let token_symbol = vamping_info.token_symbol.clone();

        let hex_address = format!(
            "0x{}",
            vamping_info.token_erc20_address.encode_hex::<String>()
        );

        emit!(TokenMintCreated {
            mint_account: ctx.accounts.mint_account.key(),
            token_name,
            token_symbol,
            token_erc20_address: hex_address,
            amount: vamping_info.amount
        });
        Ok(())
    }

    // TDOD: add logic to avoid double claim
    pub fn claim(
        ctx: Context<Claim>,
        eth_address: [u8; 20],
        balance: u64,
        solver_individual_balance_sig: [u8; 65],
        validator_individual_balance_sig: [u8; 65],
        ownership_sig: [u8; 65],
    ) -> Result<()> {
        claim_tokens(ctx, eth_address, balance, solver_individual_balance_sig, validator_individual_balance_sig, ownership_sig)
    }
}
