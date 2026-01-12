use std::sync::Arc;

use anchor_client::Program;
use anchor_lang::{InstructionData, ToAccountMetas, declare_program};
use anyhow::{Result, anyhow};

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Signature;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, pubkey::Pubkey, signature::Keypair,
    signer::Signer as SolanaSigner, transaction::Transaction,
};
use tracing::info;

declare_program!(solana_vamp_program);

pub struct SolanaTransaction {
    solana_url: String,
}

impl SolanaTransaction {
    pub fn new<T>(solana_url: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            solana_url: solana_url.into(),
        }
    }

    pub async fn prepare<TransactionAccounts, TransactionArgs>(
        &self,
        payer_keypair: Arc<Keypair>,
        program: Arc<Program<Arc<Keypair>>>,
        mint_account: Pubkey,
        vamp_state: Pubkey,
        transaction_accounts: TransactionAccounts,
        transaction_args: TransactionArgs,
    ) -> Result<(Transaction, Pubkey, Pubkey)>
    where
        TransactionAccounts: ToAccountMetas,
        TransactionArgs: InstructionData,
    {
        let program_instructions = program
            .request()
            .accounts(transaction_accounts)
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

    pub async fn submit_transaction(&self, transaction: Transaction) -> Result<Signature> {
        let client =
            RpcClient::new_with_commitment(&self.solana_url, CommitmentConfig::confirmed());
        let tx_sig = client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| anyhow!("Failed to send transaction: {}", e))?;
        info!("Transaction submitted: {}", tx_sig);

        Ok(tx_sig)
    }

    async fn get_latest_block_hash(&self) -> Result<Hash> {
        // TODO: Add the chain selection logic here
        let client =
            RpcClient::new_with_commitment(&self.solana_url, CommitmentConfig::confirmed());
        Ok(client
            .get_latest_blockhash()
            .map_err(|e| anyhow!("Failed to get latest blockhash: {}", e))?)
    }
}
