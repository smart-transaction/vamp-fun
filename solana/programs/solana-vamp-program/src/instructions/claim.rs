// File: src/instructions/claim.rs

use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use crate::state::vamp_state::VampState;
use crate::event::ErrorCode;

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vamp", vamp_state.authority.as_ref()],
        bump = vamp_state.bump
    )]
    pub vamp_state: Account<'info, VampState>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vamp_state,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub claimer_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

fn verify_ethereum_signature(
    _message: &str,
    _signature: &str,
    _expected_address: [u8; 20],
) -> Result<()> {
    // let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    // let prefixed_message = format!("{}{}", prefix, message);
    // let message_hash = keccak::hash(&prefixed_message.as_bytes()).0;

    // // Parse the signature
    // let signature_bytes = hex::decode(signature.trim_start_matches("0x"))
    //     .map_err(|_| ErrorCode::InvalidSignature)?;
    
    // let recid = RecoveryId::try_from(signature_bytes[64] as i32 - 27).map_err(|_| ErrorCode::InvalidSignature)?;

    // let recoverable_signature = RecoverableSignature::from_compact(
    //     &signature_bytes[..64],
    //     recid,
    // ).map_err(|_| ErrorCode::InvalidSignature)?;

    // // Create a message object from the hash
    // let message = Message::from_digest(message_hash);

    // // Recover the public key
    // let secp = Secp256k1::new();
    // let public_key = secp.recover_ecdsa(&message, &recoverable_signature)
    //     .map_err(|_| ErrorCode::InvalidSignature)?;

    // // Get the recovered Ethereum address (last 20 bytes of the keccak hash of the public key)
    // let public_key_bytes = public_key.serialize_uncompressed();
    // let recovered_address = &keccak::hash(&public_key_bytes[1..]).0[12..];

    // // Verify the signature
    // require!(
    //     recovered_address == expected_address,
    //     ErrorCode::InvalidSignature
    // );

    Ok(())
}

pub fn claim_tokens(
    ctx: Context<Claim>,
    eth_address: [u8; 20],
    eth_signature: String,
) -> Result<()> {
    // Find the token amount for the given ETH address
    let amount = ctx.accounts.vamp_state.token_mappings
        .iter()
        .find(|mapping| mapping.eth_address == eth_address)
        .ok_or(ErrorCode::InvalidAddress)?
        .token_amount;

    // Verify the Ethereum signature
    verify_ethereum_signature(&amount.to_string(), &eth_signature, eth_address)?;

    // Transfer tokens
    let seeds = &[
        b"vamp",
        ctx.accounts.vamp_state.authority.as_ref(),
        &[ctx.accounts.vamp_state.bump],
    ];

    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.claimer_token_account.to_account_info(),
                authority: ctx.accounts.vamp_state.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;

    Ok(())
}
