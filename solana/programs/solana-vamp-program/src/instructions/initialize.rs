use anchor_lang::prelude::*;
use crate::state::vamp_state::{Counter, TokenMapping, VampState};
use anchor_spl::metadata::{
    create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    Metadata,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = authority,
        seeds = [b"counter"],
        bump,
        space = 8 + Counter::INIT_SPACE,
    )]
    pub counter_account: Account<'info, Counter>,

    #[account(
        init,
        payer = authority,
        seeds = [b"mint", authority.key().as_ref(), &counter_account.counter.to_le_bytes()],
        bump,
        mint::decimals = 9,
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
        space = 10000 // space = ANCHOR_DISCRIMINATOR + VampState::INIT_SPACE
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
    pub vault: Account<'info, TokenAccount>, // Vamp's token vault

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    pub fn create_token_mint(
        &mut self,
        token_mappings: Vec<TokenMapping>,
        token_name: String,
        token_symbol: String,
        token_uri: String,
        amount: u64,
        _token_decimals: u8,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[&[b"mint", self.authority.key.as_ref(), &self.counter_account.counter.to_le_bytes(), &[bumps.mint_account]]];

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
            token_mappings,
            authority: self.authority.key(),
            bump: bumps.vamp_state,
            mint: self.mint_account.key(),
        });

        Ok(())
    }
}

