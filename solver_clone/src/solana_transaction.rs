use anyhow::{Result, anyhow};

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{hash::Hash, signature::Signature, transaction::Transaction};

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
