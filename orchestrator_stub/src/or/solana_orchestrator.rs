use anchor_client::anchor_lang::declare_program;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::{Client, Cluster};
use anyhow::{anyhow, Result};
use mpl_token_metadata::ID as TOKEN_METADATA_PROGRAM_ID;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction;
use solana_sdk::{bs58, signature::Keypair, signer::Signer, system_program, sysvar};
use spl_associated_token_account::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token::ID as TOKEN_PROGRAM_ID;
use std::sync::Arc;

declare_program!(solana_vamp_program);
use solana_vamp_program::{client::accounts, client::args};

pub struct SolanaOrchestrator;

impl SolanaOrchestrator {
    fn get_solana_cluster(cluster: &str) -> Result<Cluster> {
        match cluster {
            "Devnet" => Ok(Cluster::Devnet),
            "Testnet" => Ok(Cluster::Testnet),
            "Localnet" => Ok(Cluster::Localnet),
            "Mainnet" => Ok(Cluster::Mainnet),
            "Debug" => Ok(Cluster::Debug),
            _ => Err(anyhow!("Unknown Solana cluster type"))
        }
    }

    pub async fn submit_to_solana(
        vamping_data_bytes: Vec<u8>,
        _tmp_source_token_address: Vec<u8>,
        cluster: String,
        private_key: String,
        _tmp_chain_id: u64,
        _tmp_salt: u64,
        request_id: Vec<u8>,
    ) -> Result<String> {
        //TODO: Will be replaced with signing on the solver side
        let key_bytes = bs58::decode(private_key).into_vec()?; // decode base58
        log::debug!("Decoded len: {}", key_bytes.len());
        let payer_keypair = Arc::new(Keypair::from_bytes(&key_bytes)?);
        log::debug!("payer_keypair.pubkey: {}", payer_keypair.pubkey());

        let client = Client::new_with_options(
            Self::get_solana_cluster(cluster.as_str())?,
            payer_keypair.clone(),
            CommitmentConfig::confirmed(),
        );
        let program = client.program(solana_vamp_program::ID)?;

        log::debug!("solana_vamp_program::ID: {}", solana_vamp_program::ID);
        log::debug!("token_program::ID: {}", TOKEN_PROGRAM_ID);
        log::debug!("token_metadata_program::ID: {}", TOKEN_METADATA_PROGRAM_ID);
        log::debug!(
            "associated_token_program::ID: {}",
            ASSOCIATED_TOKEN_PROGRAM_ID
        );
        log::debug!("system_program::ID: {}", system_program::ID);
        log::debug!("sysvar::rent::ID: {}", sysvar::rent::ID);

        // let seeds:&[&[u8]] = &[
        //     b"clone",
        //     &tmp_source_token_address, // Unique per ERC-20
        //     &tmp_chain_id.to_le_bytes(), // Cross-chain uniqueness
        //     &tmp_salt.to_le_bytes(),  // Optional extra entropy/versioning (multiple clones of same token at least for our development testing stage)
        // ];
        // let (destination_token_address, _bump) = Pubkey::find_program_address(seeds,
        //                                                                       &solana_vamp_program::ID);
        //
        // let (mint_account, _bump) =
        //     Pubkey::find_program_address(&[b"mint"], &solana_vamp_program::ID);
        // log::info!("mint_account: {}", mint_account);

        // let mint_keypair = Arc::new(Keypair::new());
        // log::debug!("mint_keypair.pubkey: {}", mint_keypair.pubkey());
        // let mint_account = mint_keypair.pubkey();

        let vamp_identifier = fold_request_id(&request_id)?;

        let (mint_account, _) = Pubkey::find_program_address(
            &[b"mint", payer_keypair.pubkey().as_ref(), vamp_identifier.to_le_bytes().as_ref()],
            &solana_vamp_program::ID,
        );

        let (mint_authority, _bump) =
            Pubkey::find_program_address(&[b"mint_authority"], &solana_vamp_program::ID);
        log::debug!("mint_authority: {}", mint_authority);

        let (metadata_account, _bump) = Pubkey::find_program_address(
            &[
                b"metadata",
                TOKEN_METADATA_PROGRAM_ID.as_ref(),
                mint_account.as_ref(),
            ],
            &TOKEN_METADATA_PROGRAM_ID,
        );
        log::debug!("metadata_account: {}", metadata_account);

        let (vamp_state, _bump) = Pubkey::find_program_address(
            &[b"vamp", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );
        log::debug!("vamp_state: {}", vamp_state);

        let (vault, _bump) = Pubkey::find_program_address(
            &[b"vault", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );
        log::debug!("vault: {}", vault);

        let program_instructions = program
            .request()
            .accounts(accounts::CreateTokenMint {
                authority: payer_keypair.pubkey(),
                // mint_account: destination_token_address,
                mint_account,
                metadata_account,
                vamp_state,
                vault,
                token_program: TOKEN_PROGRAM_ID,
                token_metadata_program: TOKEN_METADATA_PROGRAM_ID,
                system_program: system_program::ID,
                associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
                rent: sysvar::rent::ID,
            })
            .args(args::CreateTokenMint {
                vamp_identifier: fold_request_id(&request_id)?,
                vamping_data: vamping_data_bytes,
            })
            .instructions()?;

        // Add compute limit
        let compute_ix = ComputeBudgetInstruction::set_compute_unit_limit(2_000_000);
        let mut all_instructions = vec![compute_ix];
        all_instructions.extend(program_instructions);

        // Send the transaction manually
        let recent_blockhash = program.rpc().get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &all_instructions,
            Some(&payer_keypair.pubkey()),
            &[&*payer_keypair],
            // &[&*payer_keypair, &*mint_keypair],
            recent_blockhash,
        );

        let sig = program.rpc().send_and_confirm_transaction(&tx).await?;
        log::info!("Transaction submitted: {}", sig);

        Ok(sig.to_string())
    }
}

fn fold_request_id(request_id: &[u8]) -> Result<u64> {
    let mut hash64 = 0u64;
    for chunk in request_id.chunks(8) {
        let chunk_value = u64::from_le_bytes(chunk.try_into()?);
        hash64 ^= chunk_value; // XOR the chunks to reduce to 64 bits
    }
    Ok(hash64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_request_id_empty() {
        let request_id = vec![];
        let result = fold_request_id(&request_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_fold_request_id_single_chunk() {
        let request_id = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let result = fold_request_id(&request_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_fold_request_id_multiple_chunks() {
        let request_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
        ];
        let result = fold_request_id(&request_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // 1 XOR 2 = 3
    }

    #[test]
    fn test_fold_request_id_partial_chunk() {
        let request_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0,    // Partial second chunk
        ];
        let result = fold_request_id(&request_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_fold_request_id_large_input() {
        let request_id = vec![
            1, 0, 0, 0, 0, 0, 0, 0, // First chunk
            2, 0, 0, 0, 0, 0, 0, 0, // Second chunk
            3, 0, 0, 0, 0, 0, 0, 0, // Third chunk
        ];
        let result = fold_request_id(&request_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // 1 XOR 2 XOR 3 = 0
    }
}
