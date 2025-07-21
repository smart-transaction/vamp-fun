use anchor_lang::prelude::*;
use crate::constant::ANCHOR_DISCRIMINATOR;
use crate::state::vamp_state::VampState;
use anchor_spl::metadata::{
    create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    Metadata,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

// Controls how quickly price rises
const CURVE_SLOPE: u64 = 5;

#[derive(Accounts)]
#[instruction(vamp_identifier: u64, token_decimals: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [b"mint", authority.key().as_ref(), &vamp_identifier.to_le_bytes()],
        bump,
        mint::decimals = token_decimals,
        mint::authority = mint_account.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    /// CHECK: This is safe because we're deriving the PDA
    #[account(
        mut,
        seeds = [b"metadata", token_metadata_program.key().as_ref(), mint_account.key().as_ref()],
        bump,
        seeds::program = token_metadata_program.key(),
    )]
    pub metadata_account: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [b"vamp", mint_account.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + VampState::INIT_SPACE
    )]
    pub vamp_state: Account<'info, VampState>,

    #[account(
        init,
        payer = authority,
        token::mint = mint_account,
        token::authority = vamp_state,
        seeds = [b"vault", mint_account.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: This is safe because we're creating a SOL vault PDA
    #[account(
        init,
        payer = authority,
        seeds = [b"sol_vault", mint_account.key().as_ref()],
        bump,
        space = 0, // SOL accounts don't need space
    )]
    pub sol_vault: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    pub fn create_token_mint(
        &mut self,
        vamp_identifier: u64,
        token_name: String,
        token_symbol: String,
        token_uri: String,
        amount: u64,
        _token_decimals: u8,
        solver_public_key: Vec<u8>,
        validator_public_key: Vec<u8>,
        intent_id: Vec<u8>,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", self.authority.key.as_ref(), &vamp_identifier.to_le_bytes(), &[bumps.mint_account]]];

        create_metadata_accounts_v3(
            CpiContext::new_with_signer(
                self.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: self.metadata_account.to_account_info(),
                    mint: self.mint_account.to_account_info(),
                    mint_authority: self.mint_account.to_account_info(),
                    update_authority: self.mint_account.to_account_info(),
                    payer: self.authority.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                    rent: self.rent.to_account_info(),
                },
                signer_seeds,
            ),
            DataV2 {
                name: token_name,
                symbol: token_symbol,
                uri: token_uri,
                seller_fee_basis_points: 0,
                creators: None,
                collection: None,
                uses: None,
            },
            false,
            true,
            None,
        )?;

        msg!("Token created successfully.");

        mint_to(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.mint_account.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.mint_account.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        self.vamp_state.set_inner(VampState {
            bump: bumps.vamp_state,
            mint: self.mint_account.key(),
            solver_public_key,
            validator_public_key,
            vamp_identifier,
            intent_id,
            total_claimed: 0,
            reserve_balance: 0,
            token_supply: amount,
            curve_exponent: 2,
            sol_vault: self.sol_vault.key(),
            curve_slope: CURVE_SLOPE,             // Controls how quickly price rises
            base_price: 2_000,          // ~0.000002 SOL per token (2,000 lamports)
            max_price: Some(50_000),    // Hard cap ~0.00005 SOL per token (UX-friendly)
        });

        Ok(())
    }
}

