use std::{collections::HashMap, error::Error, str::FromStr, sync::Arc};

use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, BlockNumber, Filter, H256, U256},
    utils::keccak256,
};
use log::{error, info};
use mysql::{Pool, PooledConn, TxOpts, prelude::Queryable};
use tokio::{spawn, sync::mpsc::Sender};

use crate::chain_info::{ChainInfo, fetch_chains};

pub struct SnapshotIndexer {
    chain_info: HashMap<u64, ChainInfo>,
    mysql_host: String,
    mysql_port: u16,
    mysql_user: String,
    mysql_password: String,
    mysql_database: String,
    tx: Sender<HashMap<Address, U256>>,
}

impl SnapshotIndexer {
    pub fn new(
        mysql_host: String,
        mysql_port: u16,
        mysql_user: String,
        mysql_password: String,
        mysql_database: String,
        tx: Sender<HashMap<Address, U256>>,
    ) -> Self {
        Self {
            chain_info: HashMap::new(),
            mysql_host,
            mysql_port,
            mysql_user,
            mysql_password,
            mysql_database,
            tx,
        }
    }

    pub fn create_db_conn(&self) -> Result<PooledConn, Box<dyn Error>> {
        let mysql_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.mysql_user,
            self.mysql_password,
            self.mysql_host,
            self.mysql_port,
            self.mysql_database
        );
        let mysql_display_url = format!(
            "mysql://{}:{}@{}:{}/{}",
            self.mysql_user, "********", self.mysql_host, self.mysql_port, self.mysql_database
        );
        info!(
            "Connecting to the database with URL {} ...",
            mysql_display_url
        );
        let db_conn = Pool::new(mysql_url.as_str())?.get_conn()?;
        info!("Successfully created DB connection.");

        Ok(db_conn)
    }

    pub async fn init_chain_info(&mut self) -> Result<(), Box<dyn Error>> {
        let chains = fetch_chains().await?;
        self.chain_info = chains;
        info!("Successfully fetched chain information.");
        Ok(())
    }

    pub async fn index_snapshot(
        &self,
        chain_id: u64,
        erc20_address: Address,
        block_number: u64,
    ) -> Result<(), Box<dyn Error>> {
        info!(
            "Indexing snapshot for token address: {:?} at block number: {:?}",
            erc20_address, block_number
        );

        let provider = Arc::new(self.connect_chain(chain_id).await?);
        let (mut token_supply, prev_block_number) = self.read_token_supply(erc20_address).await?;

        let tx = self.tx.clone();
        let mysql_conn = self.create_db_conn()?;
        spawn(async move {
            let blocks_step = 10000;
            let first_block = prev_block_number.unwrap_or(0);
            let latest_block = 1145842;

            info!("Processing blocks from {} to {}", first_block, latest_block);

            let event_signature = H256::from_slice(&keccak256("Transfer(address,address,uint256)"));

            for b in (first_block..latest_block).step_by(blocks_step) {
                let filter = Filter::new()
                    .from_block(BlockNumber::from(b))
                    .to_block(BlockNumber::from(b + blocks_step - 1))
                    .topic0(event_signature)
                    .address(erc20_address);

                let logs = provider.get_logs(&filter).await;
                if let Err(err) = logs {
                    error!("Failed to get logs: {:?}", err);
                    return;
                }
                let logs = logs.unwrap();
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
                }
            }
            // Writing the token supply to the database
            if let Err(err) =
                Self::write_token_supply(mysql_conn, erc20_address, block_number, &token_supply)
                    .await
            {
                error!("Failed to write token supply: {:?}", err);
                return;
            }

            // Sending the token supply to the channel
            if let Err(err) = tx.send(token_supply).await {
                error!("Failed to send token supply: {:?}", err);
            }

            info!(
                "Successfully indexed snapshot for token address: {:?}",
                erc20_address
            );
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

    async fn read_token_supply(
        &self,
        erc20_address: Address,
    ) -> Result<(HashMap<Address, U256>, Option<usize>), Box<dyn Error>> {
        let mut conn = self.create_db_conn()?;

        let mut token_supply = HashMap::new();

        // Reading the current snapshot from the database
        let stmt = "SELECT holder_address, holder_amount FROM tokens WHERE erc20_address = ?";
        let addr_str = format!("{:#x}", erc20_address);
        let result = conn.exec_iter(stmt, (&addr_str,))?;

        for row in result {
            let row = row?;
            let token_address: Option<String> = row.get(0);
            let token_supply_value: Option<String> = row.get(1);
            if let Some(token_address) = token_address {
                if let Some(token_supply_value) = token_supply_value {
                    let token_supply_value = token_supply_value.parse::<U256>()?;
                    let token_address = Address::from_str(&token_address)?;
                    token_supply.insert(token_address, token_supply_value);
                }
            }
        }

        // Reading the latest block number from the database
        let stmt = "SELECT block_number FROM epochs WHERE erc20_address = ? ORDER BY block_number DESC LIMIT 1";
        let block_num: Option<usize> = conn.exec_first(stmt, (&addr_str,))?;

        Ok((token_supply, block_num))
    }

    async fn write_token_supply(
        mut conn: PooledConn,
        erc20_address: Address,
        block_number: u64,
        token_supply: &HashMap<Address, U256>,
    ) -> Result<(), Box<dyn Error>> {
        let mut tx = conn.start_transaction(TxOpts::default())?;
        let stmt = "DELETE FROM tokens WHERE erc20_address = ?";
        let str_address = format!("{:#x}", erc20_address);
        tx.exec_drop(stmt, (&str_address,))?; // Delete existing records for the given erc20_address
        // Delete existing records for the given erc20_address

        // Insert new supplies
        for (token_address, supply) in token_supply {
            let stmt = "INSERT INTO tokens (erc20_address, holder_address, holder_amount) VALUES (?, ?, ?)";
            let addr_str = format!("{:#x}", erc20_address);
            let token_addr_str = format!("{:#x}", token_address);
            tx.exec_drop(stmt, (addr_str, token_addr_str, supply.to_string()))?;
        }
        // Insert new epoch
        let stmt = "INSERT INTO epochs (erc20_address, block_number) VALUES(?, ?)";
        tx.exec_drop(stmt, (&str_address, block_number))?;

        tx.commit()?;
        Ok(())
    }
}
