use crate::rr::storage::Storage;
use crate::utils::crypto::calculate_hash;
use ethers::providers::{Provider, Ws};
use ethers::prelude::*;
use futures_util::StreamExt;

pub struct EthereumListener {
    storage: Storage,
    provider: Provider<Ws>,
    contract_address: Address,
}

impl EthereumListener {
    pub async fn new(storage: Storage, cfg: &config::Config) -> anyhow::Result<Self> {
        let provider_url: String = cfg.get("ethereum.rpc_url")?;
        let provider = Provider::<Ws>::connect(provider_url).await?;
        let contract_address = cfg.get::<String>("ethereum.contract_address")?.parse()?;
        Ok(Self { storage, provider, contract_address })
    }

    pub async fn listen(&self) -> anyhow::Result<()> {
        let filter = Filter::new().address(self.contract_address);
        let mut stream = self.provider.subscribe_logs(&filter).await?;

        while let Some(log) = stream.next().await {
            let json_bytes = serde_json::to_vec(&log)?;
            let sequence_id = self.storage.next_sequence_id().await?;
            let event_hash = calculate_hash(&json_bytes);

            self.storage.save_new_request(&sequence_id, &json_bytes).await?;

            log::info!("Stored event seq_id={} hash={}", sequence_id, event_hash);
        }

        Ok(())
    }
}
