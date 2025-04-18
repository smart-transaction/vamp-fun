use anchor_client::anchor_lang::declare_program;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::{Client, Cluster};
use anyhow::Result;
use solana_sdk::{bs58, signature::Keypair, signer::Signer, system_program, sysvar};
use spl_token::ID as TOKEN_PROGRAM_ID;
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use std::sync::Arc;
use solana_sdk::pubkey::Pubkey;

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

pub struct SolanaOrchestrator;

impl SolanaOrchestrator {

    pub async fn submit_to_solana(vamping_data_bytes: Vec<u8>) -> Result<()> {
        //TODO: Will be replaced with signing on the solver side
        let key_str = "61jm122Tk5xu67ruvsHXK6fZZptmcEWFD2XRjMtCqUPr8NJqwbAkcvsREJigRVbzpNpACrE7ts2RhBapXtRxFJ3P"; // Hardcoded PoC testnet key
        let key_bytes = bs58::decode(key_str).into_vec()?; // decode base58
        log::debug!("Decoded len: {}", key_bytes.len());
        let payer_keypair = Arc::new(Keypair::from_bytes(&key_bytes)?);
        log::debug!("Decoded len: {}", payer_keypair.pubkey());

        let mint_keypair = Arc::new(Keypair::new());
        let vault_keypair = Arc::new(Keypair::new());

        let client = Client::new_with_options(
            Cluster::Devnet,
            payer_keypair.clone(),
            CommitmentConfig::confirmed(),
        );
        let program = client.program(solana_vamp_program::ID)?;

        // log::info!("solana_vamp_program::ID: {}", solana_vamp_program::ID);
        // log::info!("token_program::ID: {}", TOKEN_PROGRAM_ID);
        // log::info!("token_metadata_program::ID: {}", TOKEN_METADATA_PROGRAM_ID);
        // log::info!("associated_token_program::ID: {}", ASSOCIATED_TOKEN_PROGRAM_ID);
        // log::info!("system_program::ID: {}", system_program::ID);
        // log::info!("sysvar::rent::ID: {}", sysvar::rent::ID);

        let (metadata_account, _bump) = Pubkey::find_program_address(
            &[
                b"metadata",
                TOKEN_METADATA_PROGRAM_ID.as_ref(),
                mint_keypair.pubkey().as_ref() // should be the same as mintKeypair.publicKey
            ],
            &TOKEN_METADATA_PROGRAM_ID,
        );

        let (vamp_state, _bump) = Pubkey::find_program_address(
            &[b"vamp", payer_keypair.pubkey().as_ref()],
            &solana_vamp_program::ID,
        );

        program
            .request()
            .accounts(accounts::CreateTokenMint {
                authority: payer_keypair.pubkey(),
                mint_account: mint_keypair.pubkey(),
                metadata_account,
                vamp_state,
                vault: vault_keypair.pubkey(),
                token_program: TOKEN_PROGRAM_ID,
                token_metadata_program: TOKEN_METADATA_PROGRAM_ID,
                system_program: system_program::ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                rent: sysvar::rent::ID,
            })
            .args(args::CreateTokenMint {
                vamping_data: vamping_data_bytes,
            })
            .signer(payer_keypair.clone())
            .signer(mint_keypair.clone())
            .signer(vault_keypair.clone())
            .send()
            .await?;

        Ok(())
    }
}
