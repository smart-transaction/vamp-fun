use std::collections::HashMap;

use ethers::types::{Address, U256};
use log::info;
use tokio::sync::mpsc::Receiver;

pub async fn listen_indexed_snapshot(mut rx: Receiver<HashMap<Address, U256>>) {
  while let Some(message) = rx.recv().await {
    info!("Received indexed snapshot");
  }
}