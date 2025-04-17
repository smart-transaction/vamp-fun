// File: src/instructions/claim.rs

use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use crate::state::vamp_state::VampState;

#[error_code]
pub enum CustomError {
    #[msg("Invalid Merkle proof provided.")]
    InvalidMerkleProof,
}

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

fn verify_merkle_proof(leaf: [u8; 32], proof: &[[u8; 32]], root: [u8; 32]) -> bool {
    let mut hash = leaf;
    for p in proof.iter() {
        hash = if hash <= *p {
            keccak::hashv(&[&hash, p]).0
        } else {
            keccak::hashv(&[p, &hash]).0
        };
    }
    hash == root
}

pub fn claim_tokens(
    ctx: Context<Claim>,
    amount: u64,
    proof: Vec<[u8; 32]>,
    claimer: Pubkey,
) -> Result<()> {
    // Merkle verification
    let leaf = anchor_lang::solana_program::keccak::hashv(&[
        &claimer.to_bytes(),
        &amount.to_le_bytes(),
    ])
    .0;

    require!(
        verify_merkle_proof(leaf, &proof, ctx.accounts.vamp_state.merkle_root),
        CustomError::InvalidMerkleProof
    );

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
