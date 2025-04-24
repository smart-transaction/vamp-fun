use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use libsecp256k1::Signature;
use solana_program::{
    account_info::AccountInfo, keccak::hash, program_error::ProgramError, pubkey::Pubkey,
    secp256k1_recover::secp256k1_recover,
};

use crate::{event::ErrorCode, state::vamp_state::VampState};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vamp", mint_account.key().as_ref()],
        bump = vamp_state.bump
    )]
    pub vamp_state: Account<'info, VampState>,

    #[account(
        mut,
        seeds = [b"vault", mint_account.key().as_ref()],
        bump,
        token::mint = mint_account,
        token::authority = vamp_state,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub claimer_token_account: Account<'info, TokenAccount>,

    pub mint_account: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

fn verify_ethereum_signature(
    message: &str,
    signature: [u8; 65],
    expected_address: [u8; 20],
) -> Result<()> {
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    let prefixed_message = format!("{}{}", prefix, message);
    let message_hash = hash(&prefixed_message.as_bytes()).0;

    {
        let signature =
            Signature::parse_standard_slice(&signature[..64]).map_err(|e| {
                msg!("Failed to parse signature: {:?}", e);
                ProgramError::InvalidArgument
            })?;

        if signature.s.is_high() {
            msg!("signature with high-s value");
            return Err(ProgramError::InvalidArgument.into());
        }
    }

    // Parse the signature
    let recid = signature[64];

    let recovered_pubkey = secp256k1_recover(&message_hash, recid - 27, &signature[..64])
        .map_err(|_| ProgramError::InvalidArgument)?;

    let public_key_bytes = recovered_pubkey.0;

    let recovered_address = &hash(&public_key_bytes).0[12..];

    // Verify the signature
    require!(
        recovered_address == expected_address,
        ErrorCode::InvalidSignature
    );

    Ok(())
}

pub fn claim_tokens(
    ctx: Context<Claim>,
    amount: u64,
    eth_address: [u8; 20],
    eth_signature: [u8; 65],
) -> Result<()> {
    // Find the token amount for the given ETH address
    // let amount = ctx.accounts.vamp_state.token_mappings
    //     .iter()
    //     .find(|mapping| mapping.eth_address == eth_address)
    //     .ok_or(ErrorCode::InvalidAddress)?
    //     .token_amount;

    // Verify the Ethereum signature
    verify_ethereum_signature(&amount.to_string(), eth_signature, eth_address)?;

    let mint_key = ctx.accounts.mint_account.key();
    let seeds = &[
        b"vamp".as_ref(),
        mint_key.as_ref(),
        &[ctx.accounts.vamp_state.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Transfer from vault to claimer
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.claimer_token_account.to_account_info(),
                authority: ctx.accounts.vamp_state.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )?;

    Ok(())
}
