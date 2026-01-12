use std::sync::Arc;

use anchor_client::Program;
use anchor_lang::declare_program;
use anyhow::{Result, anyhow};

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Signature;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Keypair,
    signer::Signer as SolanaSigner, system_program, sysvar, transaction::Transaction,
};
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::ID as TOKEN_PROGRAM_ID;

pub struct CloneTransactionArgs {
    pub token_decimals: u8,
    pub token_name: String,
    pub token_symbol: String,
    pub token_erc20_address: Vec<u8>,
    pub token_uri: String,
    pub amount: u64,
    pub solver_public_key: Vec<u8>,
    pub validator_public_key: Vec<u8>,
    pub intent_id: Vec<u8>,
    pub vamp_identifier: u64,
    pub paid_claiming_enabled: bool,
    pub use_bonding_curve: bool,
    pub curve_slope: u64,
    pub base_price: u64,
    pub max_price: u64,
    pub flat_price_per_token: u64,
}

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};
use tracing::info;

pub struct SolanaTransaction {
    url: String,
}

impl SolanaTransaction {
    pub fn new<T>(url: T) -> Self
    where
        T: Into<String>,
    {
        Self { url: url.into() }
    }

    pub fn prepare(
        &self,
        payer_keypair: Arc<Keypair>,
        program: Arc<Program<Arc<Keypair>>>,
        recent_blockhash: [u8; 32],
        transaction_args: CloneTransactionArgs,
    ) -> Result<(Transaction, Pubkey, Pubkey)> {
        let (mint_account, _) = Pubkey::find_program_address(
            &[
                b"mint",
                payer_keypair.pubkey().as_ref(),
                transaction_args.vamp_identifier.to_le_bytes().as_ref(),
            ],
            &solana_vamp_program::ID,
        );

        let (metadata_account, _bump) = Pubkey::find_program_address(
            &[
                b"metadata",
                TOKEN_METADATA_PROGRAM_ID.as_ref(),
                mint_account.as_ref(),
            ],
            &TOKEN_METADATA_PROGRAM_ID,
        );

        let (vamp_state, _) =
            Pubkey::find_program_address(&[b"vamp", mint_account.as_ref()], &solana_vamp_program::ID);

        let (vault, _) =
            Pubkey::find_program_address(&[b"vault", mint_account.as_ref()], &solana_vamp_program::ID);

        let (sol_vault, _) = Pubkey::find_program_address(
            &[b"sol_vault", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );
        let program_instructions = program
            .request()
            .accounts(accounts::CreateTokenMint {
                authority: payer_keypair.pubkey(),
                // mint_account: destination_token_address,
                mint_account,
                metadata_account,
                vamp_state,
                vault,
                sol_vault,
                token_program: TOKEN_PROGRAM_ID,
                token_metadata_program: TOKEN_METADATA_PROGRAM_ID,
                system_program: system_program::ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                rent: sysvar::rent::ID,
            })
            .args(args::CreateTokenMint {
                vamp_identifier: transaction_args.vamp_identifier,
                token_decimals: transaction_args.token_decimals,
                token_name: transaction_args.token_name,
                token_symbol: transaction_args.token_symbol,
                token_erc20_address: transaction_args.token_erc20_address,
                token_uri: transaction_args.token_uri,
                amount: transaction_args.amount,
                solver_public_key: transaction_args.solver_public_key,
                validator_public_key: transaction_args.validator_public_key,
                intent_id: transaction_args.intent_id,
                paid_claiming_enabled: transaction_args.paid_claiming_enabled,
                use_bonding_curve: transaction_args.use_bonding_curve,
                curve_slope: transaction_args.curve_slope,
                base_price: transaction_args.base_price,
                max_price: transaction_args.max_price,
                flat_price_per_token: transaction_args.flat_price_per_token,
            })
            .instructions()?;

        info!(
            "🔧 Creating token mint with decimals: {}",
            transaction_args.token_decimals
        );

        // Add compute limit
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);
        let mut all_instructions = vec![compute_ix];
        all_instructions.extend(program_instructions);

        let tx = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&payer_keypair.pubkey()),
            &[&*payer_keypair],
            Hash::new_from_array(recent_blockhash),
        );
        Ok((tx, mint_account, vamp_state))
    }

    pub async fn get_latest_block_hash(&self) -> Result<Hash> {
        // TODO: Add the chain selection logic here
        let client = RpcClient::new_with_commitment(&self.url, CommitmentConfig::confirmed());
        Ok(client
            .get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get latest blockhash: {}", e))?)
    }

    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<Signature> {
        let client = RpcClient::new_with_commitment(&self.url, CommitmentConfig::confirmed());
        let tx_sig = client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
        log::info!("Transaction submitted: {}", tx_sig);

        Ok(tx_sig)
    }
}
