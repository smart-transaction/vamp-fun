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
use tracing::info;

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

pub struct SolanaTransaction {
    solana_url: String,
}

impl SolanaTransaction {
    pub fn new<T>(solana_url: T) -> Self
    where
        T: Into<String>,
    {
        Self { solana_url: solana_url.into() }
    }

    pub async fn prepare(
        &self,
        payer_keypair: Arc<Keypair>,
        program: Arc<Program<Arc<Keypair>>>,
        transaction_args: args::CreateTokenMint,
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
            .args(transaction_args)
            .instructions()?;

        // Add compute limit
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);
        let mut all_instructions = vec![compute_ix];
        all_instructions.extend(program_instructions);

        let recent_blockhash = self.get_latest_block_hash().await?.to_bytes();
        let tx = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&payer_keypair.pubkey()),
            &[&*payer_keypair],
            Hash::new_from_array(recent_blockhash),
        );
        Ok((tx, mint_account, vamp_state))
    }

    async fn get_latest_block_hash(&self) -> Result<Hash> {
        // TODO: Add the chain selection logic here
        let client = RpcClient::new_with_commitment(&self.solana_url, CommitmentConfig::confirmed());
        Ok(client
            .get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get latest blockhash: {}", e))?)
    }

    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<Signature> {
        let client = RpcClient::new_with_commitment(&self.solana_url, CommitmentConfig::confirmed());
        let tx_sig = client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
        info!("Transaction submitted: {}", tx_sig);

        Ok(tx_sig)
    }
}
