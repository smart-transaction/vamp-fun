use std::sync::Arc;

use sqlx::MySqlPool;

use crate::{cfg::Cfg, eth_client::EthClient};

#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub eth: Arc<EthClient>,
    pub cfg: Arc<Cfg>,
}
