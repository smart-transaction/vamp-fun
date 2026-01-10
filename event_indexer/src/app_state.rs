use sqlx::MySqlPool;

use crate::{cfg::Cfg, eth_client::EthClient};

pub struct AppState {
    pub db: MySqlPool,
    pub eth: EthClient,
    pub cfg: Cfg
}
