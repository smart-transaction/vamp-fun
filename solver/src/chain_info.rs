use std::{collections::HashMap, error::Error};

use log::info;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainInfo {
    pub name: String,
    pub chain: String,
    pub icon: Option<String>,
    pub rpc: Vec<String>,
    pub features: Option<Vec<Feature>>,
    pub faucets: Vec<String>,
    pub native_currency: Option<NativeCurrency>,
    pub info_url: Option<String>,
    pub short_name: Option<String>,
    pub chain_id: u64,
    pub network_id: u64,
    pub slip44: Option<u64>,
    pub ens: Option<Ens>,
    pub explorers: Option<Vec<Explorer>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NativeCurrency {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ens {
    pub registry: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Explorer {
    pub name: String,
    pub url: String,
    pub icon: Option<String>,
    pub standard: String,
}

pub async fn fetch_chains() -> Result<HashMap<u64, ChainInfo>, Box<dyn Error>> {
    info!("Fetching chain information from chainid.network...");
    let url = "https://chainid.network/chains.json";
    let response = reqwest::get(url).await?;
    let chains: Vec<ChainInfo> = response.json().await?;
    let mut chains_map = HashMap::new();
    for chain in chains {
        chains_map.insert(chain.chain_id, chain);
    }
    info!("Successfully fetched chain information.");

    Ok(chains_map)
}

const ETHEREUM_RPC_URL_WSS: &str = "https://red-burned-rain.quiknode.pro/";
const BASE_RPC_URL_WSS: &str = "https://red-burned-rain.base-mainnet.quiknode.pro/";
const POLYGON_RPC_URL_WSS: &str = "https://red-burned-rain.matic.quiknode.pro/";
const ARBITRUM_RPC_URL_WSS: &str = "https://red-burned-rain.arbitrum-mainnet.quiknode.pro/";

pub fn get_quicknode_mapping(api_key: &str) -> HashMap<u64, String> {
    let mut ret = HashMap::new();

    ret.insert(1, format!("{}{}", ETHEREUM_RPC_URL_WSS, api_key));
    ret.insert(8453, format!("{}{}", BASE_RPC_URL_WSS, api_key));
    ret.insert(137, format!("{}{}", POLYGON_RPC_URL_WSS, api_key));
    ret.insert(42161, format!("{}{}", ARBITRUM_RPC_URL_WSS, api_key));

    ret
}