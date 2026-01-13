use std::sync::Arc;

use crate::{cfg::Cfg, events::ClaimToken};
use anchor_client::{Client as AnchorClient, Cluster, Program};
use anchor_lang::declare_program;
use anyhow::{Context, Result, anyhow};
use array_bytes::vec2array;
use balance_util::convert_to_sol_with_dec;
use intent_id_util::fold_intent_id;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use solana_transaction_util::{
    SolanaTransaction,
    solana_vamp_program::client::{accounts, args},
};
use spl_token::ID as TOKEN_PROGRAM_ID;


declare_program!(solana_vamp_program);

fn get_program_instance(payer_keypair: Arc<Keypair>) -> Result<Program<Arc<Keypair>>> {
    // The cluster doesn't matter here, it's used only for the instructions creation.
    let anchor_client = AnchorClient::new(Cluster::Debug, payer_keypair.clone());
    Ok(anchor_client.program(solana_vamp_program::ID)?)
}
#[derive(Debug)]
pub enum ClaimDataError {
    InvalidSignatureLength
}

pub struct ClaimHandler {
    pub cfg: Arc<Cfg>,
}

impl ClaimHandler {
    pub fn new(cfg: Arc<Cfg>) -> Self {
        Self { cfg }
    }

    pub async fn handle(&self, event: ClaimToken) -> Result<()> {
        let solana_payer_keypair =
            Arc::new(Keypair::from_base58_string(&self.cfg.solana_private_key));
        let solana_program = get_program_instance(solana_payer_keypair.clone());

        let (mint_account, _) = Pubkey::find_program_address(
            &[
                b"mint",
                solana_payer_keypair.pubkey().as_ref(),
                fold_intent_id(event.intent_id.as_slice())?
                    .to_le_bytes()
                    .as_ref(),
            ],
            &solana_vamp_program::ID,
        );

        let (vamp_state, _) = Pubkey::find_program_address(
            &[b"vamp", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );

        let (vault, _) = Pubkey::find_program_address(
            &[b"vault", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );

        let (sol_vault, _) = Pubkey::find_program_address(
            &[b"sol_vault", mint_account.as_ref()],
            &solana_vamp_program::ID,
        );

        let (claim_state, _) = Pubkey::find_program_address(
            &[b"claim", vamp_state.as_ref()],
            &solana_vamp_program::ID,
        );

        let claimer_token_account = Pubkey::new_from_array(event.claimer_solana.0);

        let transaction_accounts = accounts::Claim {
            authority: solana_payer_keypair.pubkey(),
            vamp_state,
            claim_state,
            vault,
            sol_vault,
            claimer_token_account,
            mint_account,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        };

        let balance = convert_to_sol_with_dec(&event.amount, event.decimals)?;

        let transaction_args = args::Claim {
            eth_address: event.claimer.into_array(),
            balance,
            ownership_sig: vec2array::<_, 65>(event.owner_signature.to_vec())?
        };

        Ok(())
    }
}
