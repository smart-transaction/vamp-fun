use anchor_client::{Client, Cluster};
use solana_sdk::signature::{Keypair};
use std::sync::Arc;
use anchor_client::anchor_lang::declare_program;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anyhow::Result;
use solana_sdk::{signer::Signer, system_program};

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

pub struct SolanaOrchestrator;

impl SolanaOrchestrator {
    pub async fn submit_to_solana(vamping_data_bytes: Vec<u8>) -> Result<()> {

        // TODO-KG: Replace with actual payer keypair
        let payer = Arc::new(Keypair::new());

        let client = Client::new_with_options(
            Cluster::Testnet,
            payer.clone(),
            CommitmentConfig::confirmed(),
        );
        let program = client.program(solana_vamp_program::ID)?;

        program
            .request()
            .accounts(accounts::CreateTokenMint {
                authority: payer.pubkey(),
                mint_account: payer.pubkey(),
                metadata_account: payer.pubkey(),
                vamp_state: payer.pubkey(),
                vault: payer.pubkey(),
                token_program: solana_vamp_program::ID,
                token_metadata_program: solana_vamp_program::ID,
                system_program: system_program::ID,
                associated_token_program: solana_vamp_program::ID,
                rent: payer.pubkey(),
            })
            .args(args::CreateTokenMint {
                vamping_data: vamping_data_bytes,
            })
            .send().await?;

        Ok(())
    }
}
