use anchor_client::anchor_lang::declare_program;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::{Client, Cluster};
use anyhow::Result;
use solana_sdk::{bs58, signature::Keypair, signer::Signer, system_program, sysvar};
use spl_token::ID as TOKEN_PROGRAM_ID;
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use std::sync::Arc;

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

pub struct SolanaOrchestrator;

impl SolanaOrchestrator {

    pub async fn submit_to_solana(vamping_data_bytes: Vec<u8>) -> Result<()> {
        //TODO: Will be replaced with signing on the solver side
        let key_str = "61jm122Tk5xu67ruvsHXK6fZZptmcEWFD2XRjMtCqUPr8NJqwbAkcvsREJigRVbzpNpACrE7ts2RhBapXtRxFJ3P"; // Hardcoded PoC testnet key
        let key_bytes = bs58::decode(key_str).into_vec()?; // decode base58
        log::debug!("Decoded len: {}", key_bytes.len());
        let payer = Arc::new(Keypair::from_bytes(&key_bytes)?);
        log::debug!("Decoded len: {}", payer.pubkey());

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
                token_program: TOKEN_PROGRAM_ID,
                token_metadata_program: TOKEN_METADATA_PROGRAM_ID,
                system_program: system_program::ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                rent: sysvar::rent::ID,
            })
            .args(args::CreateTokenMint {
                vamping_data: vamping_data_bytes,
            })
            .signer(payer.clone())
            .send()
            .await?;

        Ok(())
    }
}
