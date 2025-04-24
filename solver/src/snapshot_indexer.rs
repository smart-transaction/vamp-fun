use std::{cmp::min, collections::HashMap, error::Error, str::FromStr, sync::Arc};

use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, Filter, H256, U256},
    utils::keccak256,
};
use log::{error, info};
use mysql::{PooledConn, TxOpts, prelude::Queryable};
use tokio::spawn;

use crate::{
    chain_info::{fetch_chains, ChainInfo}, mysql_conn::create_db_conn, snapshot_processor::process_and_send_snapshot
};

#[derive(Default)]
pub struct TokenRequestData {
    pub sequence_id: u64,
    pub chain_id: u64,
    pub erc20_address: Address,
    pub token_full_name: String,
    pub token_symbol_name: String,
    pub token_uri: String,
    pub token_decimal: u8,
    pub block_number: u64,
}

pub struct SnapshotIndexer {
    chain_info: HashMap<u64, ChainInfo>,
    mysql_host: String,
    mysql_port: u16,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
    orchestrator_url: String,
}

impl SnapshotIndexer {
    pub fn new(
        mysql_host: String,
        mysql_port: u16,
        mysql_user: String,
        mysql_password: String,
        mysql_database: String,
        orchestrator_url: String,
    ) -> Self {
        Self {
            chain_info: HashMap::new(),
            mysql_host,
            mysql_port,
            mysql_user,
            mysql_password,
            mysql_database,
            orchestrator_url,
        }
    }

    pub async fn init_chain_info(&mut self) -> Result<(), Box<dyn Error>> {
        let chains = fetch_chains().await?;
        self.chain_info = chains;
        Ok(())
    }

    pub async fn index_snapshot(
        &self,
        request_data: TokenRequestData,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "Indexing snapshot for token address: {:?} at block number: {:?}",
            request_data.erc20_address, request_data.block_number
        );

        let provider = Arc::new(self.connect_chain(request_data.chain_id).await?);

        let mysql_conn = create_db_conn(
            &self.mysql_host,
            &self.mysql_port.to_string(),
            &self.mysql_user,
            &self.mysql_password,
            &self.mysql_database,
        )?;
        let (mut token_supply, prev_block_number) = Self::read_token_supply(
            mysql_conn,
            request_data.chain_id,
            request_data.erc20_address,
        )?;

        let mut total_amount = token_supply.iter()
            .map(|(_, v)| *v)
            .fold(U256::zero(), |acc, x| acc.checked_add(x).unwrap());

        let mysql_conn = create_db_conn(
            &self.mysql_host,
            &self.mysql_port.to_string(),
            &self.mysql_user,
            &self.mysql_password,
            &self.mysql_database,
        )?;
        let orchestrator_url = self.orchestrator_url.clone();
        spawn(async move {
            let blocks_step = 10000;
            let first_block = prev_block_number.unwrap_or(0) + 1;
            let latest_block = request_data.block_number as usize;

            let event_signature = H256::from_slice(&keccak256("Transfer(address,address,uint256)"));

            for b in (first_block..latest_block + 1).step_by(blocks_step) {
                let block_from = b;
                let block_to = min(b + blocks_step - 1, latest_block);
                info!("Processing blocks from {} to {}", block_from, block_to);
                // Creating a filter for the Transfer event
                let filter = Filter::new()
                    .from_block(block_from)
                    .to_block(block_to)
                    .topic0(event_signature)
                    .address(request_data.erc20_address);

                let logs = provider.get_logs(&filter).await;
                if let Err(err) = logs {
                    error!("Failed to get logs: {:?}", err);
                    return;
                }
                let logs = logs.unwrap();
                info!("Processing {} transfers", logs.len());
                for log in logs {
                    let from = log.topics.get(1).unwrap();
                    let to = log.topics.get(2).unwrap();
                    let value = U256::from(log.data.0.to_vec().as_slice());
                    let from_address = Address::from_slice(&from[12..]);
                    let to_address = Address::from_slice(&to[12..]);
                    if from_address != Address::zero() {
                        if let Some(v) = token_supply.get(&from_address) {
                            token_supply.insert(from_address, v.checked_sub(value).unwrap());
                        }
                    }
                    if to_address != Address::zero() {
                        match token_supply.get(&to_address) {
                            Some(v) => {
                                token_supply.insert(to_address, v.checked_add(value).unwrap());
                            }
                            None => {
                                token_supply.insert(to_address, value);
                            }
                        }
                    }
                    total_amount = total_amount.checked_add(value).unwrap();
                }
            }

            // Create a temporarily truncated token supply, due to a limited transaction size
            let tmp_limit = 16usize;
            if token_supply.len() > tmp_limit {
                error!("Token supply exceeds the limit of {} entries", tmp_limit);
                return;
            }

            // Writing the token supply to the database
            if let Err(err) = Self::write_token_supply(
                mysql_conn,
                request_data.chain_id,
                request_data.erc20_address,
                request_data.block_number,
                &token_supply,
            ) {
                error!("Failed to write token supply: {:?}", err);
                return;
            }

            info!(
                "Successfully indexed snapshot for token address: {:?}",
                request_data.erc20_address
            );

            // Sending the token supply to processor
            if let Err(err) = process_and_send_snapshot(request_data, total_amount, token_supply, orchestrator_url).await {
                error!("Failed to process and send snapshot: {:?}", err);
            }
        });

        Ok(())
    }

    async fn connect_chain(&self, chain_id: u64) -> Result<Provider<Http>, Box<dyn Error>> {
        // TODO: Add specific chains that require special handling.
        let chain_info = self.chain_info.get(&chain_id).ok_or(format!(
            "Chain ID {} is not registered in the chainid network",
            chain_id
        ))?;
        for chain_url in &chain_info.rpc {
            let provider = Provider::<Http>::try_from(chain_url.as_str())?;
            if let Err(err) = provider.get_block_number().await {
                error!(
                    "Failed to connect to the chain with ID {}: {}",
                    chain_id, err
                );
                continue;
            }
            return Ok(provider);
        }
        Err("Failed to connect to any RPC URL for the specified chain ID".into())
    }

    pub fn read_token_supply(
        mut conn: PooledConn,
        chain_id: u64,
        erc20_address: Address,
    ) -> Result<(HashMap<Address, U256>, Option<usize>), Box<dyn Error>> {
        let mut token_supply = HashMap::new();

        // Reading the current snapshot from the database
        let stmt = "SELECT holder_address, holder_amount FROM tokens WHERE chain_id = ? AND erc20_address = ?";
        let addr_str = format!("{:#x}", erc20_address);
        let result = conn.exec_iter(stmt, (chain_id, &addr_str))?;

        for row in result {
            let row = row?;
            let token_address: Option<String> = row.get(0);
            let token_supply_value: Option<String> = row.get(1);
            if let Some(token_address) = token_address {
                if let Some(token_supply_value) = token_supply_value {
                    let token_supply_value = U256::from_dec_str(&token_supply_value)?;
                    let token_address = Address::from_str(&token_address)?;
                    token_supply.insert(token_address, token_supply_value);
                }
            }
        }

        // Reading the latest block number from the database
        let stmt = "SELECT block_number FROM epochs WHERE chain_id = ? AND erc20_address = ? ORDER BY block_number DESC LIMIT 1";
        let block_num: Option<usize> = conn.exec_first(stmt, (chain_id, &addr_str))?;

        Ok((token_supply, block_num))
    }

    fn write_token_supply(
        mut conn: PooledConn,
        chain_id: u64,
        erc20_address: Address,
        block_number: u64,
        token_supply: &HashMap<Address, U256>,
    ) -> Result<(), Box<dyn Error>> {
        // Delete existing records for the given erc20_address
        let mut tx = conn.start_transaction(TxOpts::default())?;
        let stmt = "DELETE FROM tokens WHERE chain_id = ? AND erc20_address = ?";
        let str_address = format!("{:#x}", erc20_address);
        tx.exec_drop(stmt, (chain_id, &str_address))?; // Delete existing records for the given erc20_address

        // Insert new supplies
        for (token_address, supply) in token_supply {
            let stmt = "INSERT INTO tokens (chain_id, erc20_address, holder_address, holder_amount) VALUES (?, ?, ?, ?)";
            let addr_str = format!("{:#x}", erc20_address);
            let token_addr_str = format!("{:#x}", token_address);
            tx.exec_drop(
                stmt,
                (chain_id, addr_str, token_addr_str, supply.to_string()),
            )?;
        }
        // Insert new epoch
        let stmt = "INSERT INTO epochs (chain_id, erc20_address, block_number) VALUES(?, ?, ?)";
        tx.exec_drop(stmt, (chain_id, &str_address, block_number))?;

        tx.commit()?;
        Ok(())
    }
}
