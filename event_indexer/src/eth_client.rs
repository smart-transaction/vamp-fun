use std::sync::Arc;

use alloy_provider::Provider;

pub struct EthClient {
    pub provider: Arc<dyn Provider + Send + Sync>,
}
