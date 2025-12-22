use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    account_info::AccountInfo, keccak::hash, program_error::ProgramError, pubkey::Pubkey,
    secp256k1_recover::secp256k1_recover,
};
use anchor_spl::token::{Mint, Token, TokenAccount, Transfer};
use balance_util::get_balance_hash;
use libsecp256k1::Signature;
use rust_decimal::Decimal;

use crate::instructions::calculate_claim_cost::{
    calculate_claim_cost_bonding_curve, calculate_claim_cost_fixed_price,
};
use crate::{
    event::ErrorCode,
    state::vamp_state::{ClaimState, VampState},
};

#[derive(Accounts)]
#[instruction(eth_address: [u8; 20])]
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
        init,
        payer = authority,
        seeds = [b"claim", vamp_state.key().as_ref(), &eth_address],
        bump,
        space = 8 + ClaimState::INIT_SPACE,
    )]
    pub claim_state: Account<'info, ClaimState>,

    #[account(
        mut,
        seeds = [b"vault", mint_account.key().as_ref()],
        bump,
        token::mint = mint_account,
        token::authority = vamp_state,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: This is the SOL vault PDA
    #[account(
        mut,
        seeds = [b"sol_vault", mint_account.key().as_ref()],
        bump,
    )]
    pub sol_vault: UncheckedAccount<'info>,

    #[account(mut)]
    pub claimer_token_account: Account<'info, TokenAccount>,

    pub mint_account: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

fn verify_ethereum_signature(
    message: &Vec<u8>,
    signature: [u8; 65],
    expected_address: &Vec<u8>,
    signature_type: ErrorCode,
) -> Result<()> {
    const PREFIX: &str = "\x19Ethereum Signed Message:\n";
    let len = message.len();
    let len_string = len.to_string();

    let mut eth_message = Vec::with_capacity(PREFIX.len() + len_string.len() + len);
    eth_message.extend_from_slice(PREFIX.as_bytes());
    eth_message.extend_from_slice(len_string.as_bytes());
    eth_message.extend_from_slice(message);

    let message_hash = hash(&eth_message).0;
    {
        let signature = Signature::parse_standard_slice(&signature[..64]).map_err(|e| {
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
    if recovered_address != expected_address {
        return Err(signature_type.into());
    }

    Ok(())
}

pub fn buy_claim_tokens(
    ctx: Context<Claim>,
    eth_address: [u8; 20],
    balance: u64,
    solver_individual_balance_sig: [u8; 65],
    validator_individual_balance_sig: [u8; 65],
    ownership_sig: [u8; 65],
) -> Result<()> {
    let message = get_balance_hash(
        &eth_address.to_vec(),
        balance,
        &ctx.accounts.vamp_state.intent_id,
    )
    .expect("eth message hash error");

    // Verify the solver signature
    verify_ethereum_signature(
        &message,
        solver_individual_balance_sig,
        &ctx.accounts.vamp_state.solver_public_key,
        ErrorCode::InvalidSolverSignature,
    )?;

    // Verify the owner signature
    verify_ethereum_signature(
        &message,
        ownership_sig,
        &eth_address.to_vec(),
        ErrorCode::InvalidOwnerSignature,
    )?;

    // Verify the validator signature
    verify_ethereum_signature(
        &message,
        validator_individual_balance_sig,
        &ctx.accounts.vamp_state.validator_public_key,
        ErrorCode::InvalidValidatorSignature,
    )?;

    require!(
        ctx.accounts.claim_state.is_claimed == false,
        ErrorCode::TokensAlreadyClaimed
    );

    // Calculate the SOL cost using the bonding curve on whole-token units
    let decimals = ctx.accounts.mint_account.decimals as u32;
    let unit = 10u64.pow(decimals);
    let sol_balance = Decimal::new(balance as i64, decimals);
    let sol_price = Decimal::new(
        ctx.accounts.vamp_state.flat_price_per_token as i64,
        decimals,
    );
    let claim_cost = if ctx.accounts.vamp_state.use_bonding_curve {
        calculate_claim_cost_bonding_curve(
            sol_balance,
            Decimal::new(ctx.accounts.vamp_state.total_claimed as i64, decimals),
            Decimal::new(ctx.accounts.vamp_state.base_price as i64, decimals),
            Decimal::new(ctx.accounts.vamp_state.curve_slope as i64, decimals))?
    } else {
        calculate_claim_cost_fixed_price(sol_balance, sol_price)?
    }
    .checked_mul(Decimal::new(unit as i64, 0))
    .ok_or_else(|| ProgramError::ArithmeticOverflow)?
    .try_into()
    .map_err(|_| ProgramError::ArithmeticOverflow)?;

    msg!("Claim Cost for token amount {}: {}", balance, claim_cost);

    // Transfer SOL from claimer to SOL vault using system program
    let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.authority.key(),
        &ctx.accounts.sol_vault.key(),
        claim_cost,
    );

    anchor_lang::solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.sol_vault.to_account_info(),
        ],
    )?;

    // Update bonding curve state
    let vamp_state = &mut ctx.accounts.vamp_state;
    vamp_state.reserve_balance = vamp_state
        .reserve_balance
        .checked_add(claim_cost)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    let mint_key = ctx.accounts.mint_account.key();
    let seeds = &[b"vamp".as_ref(), mint_key.as_ref(), &[vamp_state.bump]];
    let signer_seeds = &[&seeds[..]];

    // Transfer tokens from vault to claimer
    anchor_spl::token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.claimer_token_account.to_account_info(),
                authority: vamp_state.to_account_info(),
            },
            signer_seeds,
        ),
        balance,
    )?;

    // Update the total claimed counter (in base units)
    vamp_state.total_claimed = vamp_state
        .total_claimed
        .checked_add(balance)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    ctx.accounts.claim_state.is_claimed = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_ethereum_signature() {
        let message = vec![
            83, 31, 60, 150, 106, 85, 124, 28, 140, 125, 105, 187, 151, 211, 104, 177, 20, 147, 53,
            87, 63, 176, 26, 228, 4, 49, 136, 174, 166, 102, 114, 231,
        ];
        let signature = [
            251, 190, 51, 170, 61, 104, 94, 173, 134, 86, 195, 233, 114, 39, 131, 218, 205, 35,
            184, 80, 233, 53, 220, 244, 27, 165, 216, 133, 6, 251, 209, 206, 62, 148, 200, 51, 176,
            66, 113, 38, 158, 246, 60, 234, 141, 183, 42, 176, 53, 65, 143, 195, 84, 99, 162, 156,
            57, 192, 188, 82, 3, 23, 55, 169, 27,
        ];
        let expected_address = vec![
            249, 139, 130, 139, 56, 155, 239, 78, 187, 181, 145, 28, 161, 126, 79, 121, 137, 201,
            6, 141,
        ];
        let res = verify_ethereum_signature(
            &message,
            signature,
            &expected_address,
            ErrorCode::InvalidSolverSignature,
        );
        assert_eq!(Ok(()), res);
    }
}
