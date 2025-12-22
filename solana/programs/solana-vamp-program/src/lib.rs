use anchor_lang::prelude::*;

declare_id!("FAyBECn6ppQgRwb5R4LryAzNic3XwsCuHakVpD1X7hFW");

// Module declarations
mod constant;
mod event;
mod instructions;
mod state;

// Re-exports
use event::TokenMintCreated;
use instructions::*;
use instructions::initialize::VampingParams;

#[program]
pub mod solana_vamp_program {
    use super::*;
    use hex::ToHex;

    pub fn create_token_mint(
        ctx: Context<Initialize>,
        vamp_identifier: u64,
        _token_decimals: u8,
        token_name: String,
        token_symbol: String,
        token_erc20_address: Vec<u8>,
        token_uri: String,
        amount: u64,
        solver_public_key: Vec<u8>,
        validator_public_key: Vec<u8>,
        intent_id: Vec<u8>,
        paid_claiming_enabled: bool,
        use_bonding_curve: bool,
        curve_slope: u64,
        base_price: u64,
        max_price: u64,
        flat_price_per_token: u64
    ) -> Result<()> {
        // Vamping parameters
        let vamping_params = VampingParams {
            paid_claiming_enabled,
            use_bonding_curve,
            curve_slope,
            base_price,
            max_price: Some(max_price),
            flat_price_per_token,
        };

        ctx.accounts.create_token_mint(
            vamp_identifier,
            token_name.clone(),
            token_symbol.clone(),
            token_uri,
            amount,
            solver_public_key,
            validator_public_key,
            intent_id,
            &ctx.bumps,
            Some(vamping_params)
        )?;

        let hex_address = format!(
            "0x{}",
            token_erc20_address.encode_hex::<String>()
        );

        emit!(TokenMintCreated {
            mint_account: ctx.accounts.mint_account.key(),
            token_name,
            token_symbol,
            token_erc20_address: hex_address,
            amount: amount
        });
        Ok(())
    }

    pub fn claim(
        ctx: Context<Claim>,
        eth_address: [u8; 20],
        balance: u64,
        solver_individual_balance_sig: [u8; 65],
        validator_individual_balance_sig: [u8; 65],
        ownership_sig: [u8; 65],
    ) -> Result<()> {
        buy_claim_tokens(ctx, eth_address, balance, solver_individual_balance_sig, validator_individual_balance_sig, ownership_sig)
    }
}
