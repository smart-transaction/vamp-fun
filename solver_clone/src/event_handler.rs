use std::sync::{Arc, RwLock};

use crate::cfg::Cfg;
use crate::snapshot_indexer::{SnapshotIndexer, TokenRequestData};
use crate::stats::{IndexerProcesses, VampingStatus};
use crate::events::VampTokenIntent;

use alloy_primitives::Address;
use anyhow::Result;
use tracing::info;

pub struct CloneEventHandler {
    pub cfg: Arc<Cfg>,
    pub indexer: Arc<SnapshotIndexer>,
    pub stats: Arc<RwLock<IndexerProcesses>>,
}

impl CloneEventHandler {
    pub fn new(
        cfg: Arc<Cfg>,
        indexer: Arc<SnapshotIndexer>,
        indexing_stats: Arc<RwLock<IndexerProcesses>>,
    ) -> Self {
        Self {
            cfg,
            indexer,
            stats: indexing_stats
        }
    }

    pub async fn handle(&self, event: VampTokenIntent) -> Result<()> {
        info!("DeployTokenHandler triggered");
        let mut request_data = TokenRequestData::default();
        request_data.chain_id = event.chain_id;
        request_data.block_number = event.block_number;
        // Use the intent_id from the blockchain event
        request_data.intent_id = event.intent_id.to_vec();
        request_data.erc20_address = Address::from_slice(event.token.as_slice());
        request_data.token_full_name = event.token_name;
        request_data.token_symbol_name = event.token_symbol;
        request_data.token_uri = event.token_uri;
        request_data.solana_cluster = self.cfg.default_solana_cluster.clone();
        request_data.paid_claiming_enabled = self.cfg.paid_claiming_enabled;
        request_data.use_bonding_curve = self.cfg.use_bonding_curve;
        request_data.curve_slope = self.cfg.curve_slope;
        request_data.base_price = self.cfg.base_price;
        request_data.max_price = self.cfg.max_price;
        request_data.flat_price_per_token = self.cfg.flat_price_per_token;

        // Log the final vamping parameters that will be used
        info!(
            "🎯 Final vamping parameters for intent_id: 0x{}",
            hex::encode(&request_data.intent_id)
        );
        info!(
            "   paid_claiming_enabled: {:?}",
            request_data.paid_claiming_enabled
        );
        info!("   use_bonding_curve: {:?}", request_data.use_bonding_curve);
        info!("   curve_slope: {:?}", request_data.curve_slope);
        info!("   base_price: {:?}", request_data.base_price);
        info!("   max_price: {:?}", request_data.max_price);
        info!(
            "   flat_price_per_token: {:?}",
            request_data.flat_price_per_token
        );
        let stats = self.stats.clone();
        let chain_id = request_data.chain_id;
        let erc20_address = request_data.erc20_address;
        match self
            .indexer
            .index_snapshot(request_data, stats.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Ok(mut stats) = stats.write() {
                    if let Some(item) = stats.get_mut(&(chain_id, erc20_address)) {
                        item.status = VampingStatus::Failure;
                        item.message = err.to_string();
                    }
                }
                return Err(err);
            }
        }
    }
}
