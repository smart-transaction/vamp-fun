use anchor_lang::prelude::*;
use crate::constants::*;
use crate::state::vamp_state::VampState;
use anchor_spl::metadata::{
    create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
    Metadata,
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, Mint, MintTo, Token, TokenAccount},
};

// Accounts
#[derive(Accounts)]
#[instruction(_token_decimals: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = _token_decimals,
        mint::authority = authority.key(),
    )]
    pub mint_account: Account<'info, Mint>,

    /// CHECK: Validate address by deriving pda
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
        seeds = [b"vamp", authority.key().as_ref()],
        bump,
        space = ANCHOR_DISCRIMINATOR + VampState::INIT_SPACE
    )]
    pub vamp_state: Account<'info, VampState>,

    #[account(
        init,
        payer = authority,
        token::mint = mint_account,
        token::authority = vamp_state,
    )]
    pub vault: Account<'info, TokenAccount>, // Vamp's token vault

    pub token_program: Program<'info, Token>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Initialize<'info> {
    // Initialize Vamp with Merkle root and token vault
    pub fn create_token_mint(
        &mut self,
        merkle_root: [u8; 32],
        token_name: String,
        token_symbol: String,
        token_uri: String,
        amount: u64,
        _token_decimals: u8,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        create_metadata_accounts_v3(
            CpiContext::new(
                self.token_metadata_program.to_account_info(),
                CreateMetadataAccountsV3 {
                    metadata: self.metadata_account.to_account_info(),
                    mint: self.mint_account.to_account_info(),
                    mint_authority: self.authority.to_account_info(),
                    update_authority: self.authority.to_account_info(),
                    payer: self.authority.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                    rent: self.rent.to_account_info(),
                },
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
            false, // Is mutable
            true,  // Update authority is signer
            None,  // Collection details
        )?;

        msg!("Token created successfully.");

        mint_to(
            CpiContext::new(
                self.token_program.to_account_info(),
                MintTo {
                    mint: self.mint_account.to_account_info(),
                    to: self.vault.to_account_info(),
                    authority: self.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        self.vamp_state.set_inner(VampState {
            merkle_root,
            authority: self.authority.key(),
            bump: bumps.vamp_state,
            mint: self.mint_account.key(),
        });

        Ok(())
    }
}

