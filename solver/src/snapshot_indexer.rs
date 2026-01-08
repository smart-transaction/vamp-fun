use std::{
    cmp::{max, min},
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};

use alloy::{
    network::Ethereum,
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter, sol_types::SolEvent,
};
use alloy_primitives::{Address, U256};
use anchor_client::{Client as AnchorClient, Cluster, Program};
use anchor_lang::declare_program;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use solana_sdk::signature::Keypair;
use sqlx::Row;
use tokio::spawn;
use tracing::{error, info, warn};

use crate::{
    args::Args, chain_info::{ChainInfo, fetch_chains, get_quicknode_mapping}, events::Transfer, mysql_conn::create_db_conn, snapshot_processor::process_and_send_snapshot, stats::{IndexerProcesses, IndexerStats, VampingStatus}
};

#[derive(Default)]
pub struct TokenRequestData {
    pub sequence_id: u64,
    pub chain_id: u64,
    pub erc20_address: Address,
    pub token_full_name: String,
    pub token_symbol_name: String,
    pub token_uri: String,
    pub block_number: u64,
    pub intent_id: Vec<u8>,
    pub solana_cluster: String,
    // Vamping parameters from additional_data (optional, fallback to solver config if not provided)
    pub paid_claiming_enabled: bool,
    pub use_bonding_curve: bool,
    pub curve_slope: u64,
    pub base_price: u64,
    pub max_price: u64,
    pub flat_price_per_token: u64,
}

#[derive(Debug, Default, Clone)]
pub struct TokenAmount {
    pub amount: U256,
    pub signature: Vec<u8>,
}

declare_program!(solana_vamp_program);

fn get_program_instance(payer_keypair: Arc<Keypair>) -> Result<Program<Arc<Keypair>>> {
    // The cluster doesn't matter here, it's used only for the instructions creation.
    let anchor_client = AnchorClient::new(Cluster::Debug, payer_keypair.clone());
    Ok(anchor_client.program(solana_vamp_program::ID)?)
}

pub struct SnapshotIndexer {
    cfg: Arc<Args>,
    chain_info: HashMap<u64, ChainInfo>,
    quicknode_chains: HashMap<u64, String>,
    solana_payer_keypair: Arc<Keypair>,
    solana_program: Arc<Program<Arc<Keypair>>>,
    solana_url: String,
}

const BLOCK_STEP: u64 = 9990;

impl SnapshotIndexer {
    pub async fn new(args: Arc<Args>) -> Result<Self> {
        let solana_payer_keypair = Arc::new(Keypair::from_base58_string(&args.solana_private_key));
        let solana_program = Arc::new(get_program_instance(solana_payer_keypair.clone())?);
        let chains = fetch_chains()
            .await
            .map_err(|e| anyhow!("Error chains fetching: {}", e))?;
        let chain_info = chains;
        let quicknode_chains = if let Some(api_key) = args.quicknode_api_key.clone() {
            get_quicknode_mapping(&api_key)
        } else {
            HashMap::new()
        };
        let res = Self {
            cfg: args.clone(),
            chain_info,
            quicknode_chains,
            solana_payer_keypair,
            solana_program,
            solana_url: if args.default_solana_cluster == "DEVNET" {
                args.solana_devnet_url.clone()
            } else {
                args.solana_mainnet_url.clone()
            },
        };
        Ok(res)
    }

    pub async fn index_snapshot(
        &self,
        request_data: TokenRequestData,
        stats: Arc<Mutex<IndexerProcesses>>,
    ) -> Result<()> {
        info!(
            "Indexing snapshot for token address: {:?} at block number: {:?}",
            request_data.erc20_address, request_data.block_number
        );

        let provider = Arc::new(self.connect_chain(request_data.chain_id).await?);

        let (mut token_supply, prev_block_number) = self
            .read_token_supply(request_data.chain_id, request_data.erc20_address)
            .await?;

        let mut total_amount = token_supply
            .iter()
            .map(|(_, v)| v.amount)
            .fold(U256::ZERO, |acc, x| acc.checked_add(x).unwrap_or_default());

        {
            if let Ok(mut stats) = stats.lock() {
                let mut item = IndexerStats::default();
                item.chain_id = request_data.chain_id;
                item.token_address = request_data.erc20_address;
                item.status = VampingStatus::Indexing;
                item.start_timestamp = Utc::now().timestamp();
                item.current_timestamp = Utc::now().timestamp();
                stats.insert((request_data.chain_id, request_data.erc20_address), item);
            }
        }
        let cfg = self.cfg.clone();
        let solana_payer_keypair = self.solana_payer_keypair.clone();
        let solana_program = self.solana_program.clone();
        let solana_url = self.solana_url.clone();

        spawn(async move {
            let first_block = prev_block_number.unwrap_or(0) + 1;
            let latest_block = request_data.block_number;
            {
                if let Ok(mut stats) = stats.lock() {
                    let item = stats
                        .get_mut(&(request_data.chain_id, request_data.erc20_address))
                        .unwrap();
                    item.current_timestamp = Utc::now().timestamp();
                    item.start_block = first_block as u64;
                    item.end_block = max(latest_block, first_block) as u64;
                }
            }

            for b in (first_block..latest_block + 1).step_by(BLOCK_STEP as usize) {
                let block_from = b;
                let block_to = min(b + BLOCK_STEP - 1, latest_block);
                info!("Processing blocks from {} to {}", block_from, block_to);
                // Creating a filter for the Transfer event
                let filter = Filter::new()
                    .from_block(block_from)
                    .to_block(block_to)
                    .event_signature(Transfer::SIGNATURE_HASH)
                    .address(request_data.erc20_address);

                let logs = provider.get_logs(&filter).await;
                if let Err(err) = logs {
                    error!("Failed to get logs: {:?}", err);
                    return;
                }
                let logs = logs.unwrap();
                info!("Processing {} transfers", logs.len());
                for log in logs {
                    let from = log.topics()[1];
                    let to = log.topics()[2];
                    let value = U256::from_be_slice(log.data().data.as_ref());
                    let from_address = Address::from_slice(&from[12..]);
                    let to_address = Address::from_slice(&to[12..]);
                    if to_address != Address::ZERO {
                        match token_supply.get_mut(&to_address) {
                            Some(v) => {
                                v.amount = v.amount.checked_add(value).unwrap();
                            }
                            None => {
                                // Fix: Create new entry with the transfer amount instead of zero
                                token_supply.insert(
                                    to_address,
                                    TokenAmount {
                                        amount: value,
                                        signature: Vec::new(),
                                    },
                                );
                            }
                        }
                    }
                    if from_address != Address::ZERO {
                        if let Some(v) = token_supply.get_mut(&from_address) {
                            // Checking the substraction. If None then truncate the result to 0
                            if let Some(new_amount) = v.amount.checked_sub(value) {
                                v.amount = new_amount;
                            } else {
                                // If the amount is less than the value, set it to zero
                                warn!(
                                    "Token amount for address {:?} = {} is less than the deducted value {}. Setting to zero.",
                                    from_address, v.amount, value
                                );
                                v.amount = U256::ZERO;
                            }
                        }
                    }
                    total_amount = total_amount.checked_add(value).unwrap();
                }
                // Update stats
                {
                    if let Ok(mut stats) = stats.lock() {
                        let item = stats
                            .get_mut(&(request_data.chain_id, request_data.erc20_address))
                            .unwrap();
                        item.current_timestamp = Utc::now().timestamp();
                        item.blocks_done = block_to as u64;
                    }
                }
            }

            info!(
                "Successfully indexed snapshot for token address: {:?}",
                request_data.erc20_address
            );

            // Sending the token supply to processor
            let chain_id = request_data.chain_id.clone();
            let erc20_address = request_data.erc20_address.clone();

            if let Err(err) = process_and_send_snapshot(
                cfg,
                request_data,
                total_amount,
                token_supply,
                stats.clone(),
                solana_payer_keypair,
                solana_program,
                &solana_url,
            )
            .await
            {
                error!("Failed to process and send snapshot: {:?}", err);
                if let Ok(mut stats) = stats.lock() {
                    let item = stats.get_mut(&(chain_id, erc20_address)).unwrap();
                    item.status = VampingStatus::Failure;
                    item.message = err.to_string();
                }
            }
        });

        Ok(())
    }

    async fn connect_chain(&self, chain_id: u64) -> Result<Box<dyn Provider<Ethereum>>> {
        if let Some(quicknode_url) = self.quicknode_chains.get(&chain_id) {
            let url: reqwest::Url = quicknode_url.parse()?;
            let provider = ProviderBuilder::new().connect_http(url);
            let _ = provider.get_block_number().await?;
            return Ok(Box::new(provider));
        }

        let chain_info = self
            .chain_info
            .get(&chain_id)
            .ok_or(format!(
                "Chain ID {} is not registered in the chainid network",
                chain_id
            ))
            .map_err(|e| anyhow!("Error getting chain info: {}", e))?;
        for chain_url in &chain_info.rpc {
            let url: reqwest::Url = match chain_url.parse() {
                Ok(u) => u,
                Err(_) => continue,
            };
            let provider = ProviderBuilder::new().connect_http(url);
            if let Err(err) = provider.get_block_number().await {
                error!(
                    "Failed to connect to the chain with ID {}: {}",
                    chain_id, err
                );
                continue;
            }
            return Ok(Box::new(provider));
        }
        Err(anyhow!(
            "Failed to connect to any RPC URL for the specified chain ID"
        ))
    }

    pub async fn read_token_supply(
        &self,
        chain_id: u64,
        erc20_address: Address,
    ) -> Result<(HashMap<Address, TokenAmount>, Option<u64>)> {
        let mut token_supply = HashMap::new();
        let conn = create_db_conn(&self.cfg)
            .await
            .map_err(|e| anyhow!("Error creating DB connection: {}", e))?;
        // Reading the current snapshot from the database
        let addr_str = format!("{:#x}", erc20_address);
        let rows = sqlx::query(
            r#"
                SELECT holder_address, holder_amount, signature
                FROM tokens
                WHERE chain_id = ?
                  AND erc20_address = ?
            "#,
        )
        .bind(&chain_id)
        .bind(&addr_str)
        .fetch_all(&conn)
        .await
        .context("fetch token supply")?;

        for row in rows {
            let token_address = row.get::<&str, usize>(0);
            let token_supply_value = row.get::<&str, usize>(1);
            let solver_signature = row.get::<&str, usize>(2);
            let token_supply_value = U256::from_str_radix(&token_supply_value, 10)?;
            let token_address = Address::from_str(&token_address)?;
            token_supply.insert(
                token_address,
                TokenAmount {
                    amount: token_supply_value,
                    signature: hex::decode(&solver_signature)?,
                },
            );
        }

        // Reading the latest block number from the database
        let row = sqlx::query(
            r#"
                SELECT block_number
                FROM epochs
                WHERE chain_id = ?
                    AND erc20_address = ?
                ORDER BY block_number DESC
                LIMIT 1
            "#,
        )
        .bind(&chain_id)
        .bind(&addr_str)
        .fetch_optional(&conn)
        .await
        .context("Fetch latest block")?;

        Ok((token_supply, row.map(|r| r.get::<u64, usize>(0))))
    }
}
