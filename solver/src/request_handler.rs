use std::sync::{Arc, Mutex};

use crate::snapshot_indexer::{SnapshotIndexer, TokenRequestData};
use crate::stats::{IndexerProcesses, VampingStatus};
use crate::use_proto::proto::{SolanaCluster, UserEventProto};

use anyhow::{anyhow, Result};
use ethers::types::Address;
use ethers::utils::keccak256;
use tracing::info;

pub struct DeployTokenHandler {
    pub indexer: Arc<SnapshotIndexer>,
    pub contract_address_name: [u8; 32],
    pub token_full_name: [u8; 32],
    pub token_symbol_name: [u8; 32],
    pub token_uri_name: [u8; 32],
    pub token_decimal_name: [u8; 32],
    pub solana_cluster_name: [u8; 32],
    // Vamping parameters from additional_data
    pub paid_claiming_enabled_name: [u8; 32],
    pub use_bonding_curve_name: [u8; 32],
    pub curve_slope_name: [u8; 32],
    pub base_price_name: [u8; 32],
    pub max_price_name: [u8; 32],
    pub flat_price_per_token_name: [u8; 32],
    pub stats: Arc<Mutex<IndexerProcesses>>,
    pub default_solana_cluster: String,
}

const CONTRACT_ADDRESS_NAME: &str = "ERC20ContractAddress";
const TOKEN_FULL_NAME: &str = "TokenFullName";
const TOKEN_SYMBOL_NAME: &str = "TokenSymbolName";
const TOKEN_URI_NAME: &str = "TokenURI";
const TOKEN_DECIMAL_NAME: &str = "TokenDecimal";
const SOLANA_CLUSTER: &str = "SolanaCluster";
// Vamping parameters from additional_data
const PAID_CLAIMING_ENABLED: &str = "PaidClaimingEnabled";
const USE_BONDING_CURVE: &str = "UseBondingCurve";
const CURVE_SLOPE: &str = "CurveSlope";
const BASE_PRICE: &str = "BasePrice";
const MAX_PRICE: &str = "MaxPrice";
const FLAT_PRICE_PER_TOKEN: &str = "FlatPricePerToken";

impl DeployTokenHandler {
    pub fn new(indexer: Arc<SnapshotIndexer>, indexing_stats: Arc<Mutex<IndexerProcesses>>, default_solana_cluster: String) -> Self {
        let handler = Self {
            indexer,
            contract_address_name: keccak256(CONTRACT_ADDRESS_NAME.as_bytes()),
            token_full_name: keccak256(TOKEN_FULL_NAME.as_bytes()),
            token_symbol_name: keccak256(TOKEN_SYMBOL_NAME.as_bytes()),
            token_uri_name: keccak256(TOKEN_URI_NAME.as_bytes()),
            token_decimal_name: keccak256(TOKEN_DECIMAL_NAME.as_bytes()),
            solana_cluster_name: keccak256(SOLANA_CLUSTER.as_bytes()),
            paid_claiming_enabled_name: keccak256(PAID_CLAIMING_ENABLED.as_bytes()),
            use_bonding_curve_name: keccak256(USE_BONDING_CURVE.as_bytes()),
            curve_slope_name: keccak256(CURVE_SLOPE.as_bytes()),
            base_price_name: keccak256(BASE_PRICE.as_bytes()),
            max_price_name: keccak256(MAX_PRICE.as_bytes()),
            flat_price_per_token_name: keccak256(FLAT_PRICE_PER_TOKEN.as_bytes()),
            stats: indexing_stats,
            default_solana_cluster,
        };
        info!("contract_address_name: {:?}", handler.contract_address_name);
        info!("token_full_name: {:?}", handler.token_full_name);
        info!("token_symbol_name: {:?}", handler.token_symbol_name);
        info!("token_uri_name: {:?}", handler.token_uri_name);
        info!("token_decimal_name: {:?}", handler.token_decimal_name);
        info!("solana_cluster_name: {:?}", handler.solana_cluster_name);
        info!("paid_claiming_enabled_name: {:?}", handler.paid_claiming_enabled_name);
        info!("use_bonding_curve_name: {:?}", handler.use_bonding_curve_name);
        info!("curve_slope_name: {:?}", handler.curve_slope_name);
        info!("base_price_name: {:?}", handler.base_price_name);
        info!("max_price_name: {:?}", handler.max_price_name);
        info!("flat_price_per_token_name: {:?}", handler.flat_price_per_token_name);
        handler
    }

    pub async fn handle(&self, sequence_id: u64, event: UserEventProto) -> Result<()> {
        info!("DeployTokenHandler triggered");
        let mut request_data = TokenRequestData::default();
        request_data.sequence_id = sequence_id;
        request_data.chain_id = event.chain_id;
        request_data.block_number = event.block_number;
        // Use the intent_id from the blockchain event
        request_data.intent_id = event.intent_id;

        for add_data in event.additional_data {
            if add_data.key == self.contract_address_name {
                request_data.erc20_address = Address::from_slice(&add_data.value);
            } else if add_data.key == self.token_full_name {
                request_data.token_full_name = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_symbol_name {
                request_data.token_symbol_name = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_uri_name {
                request_data.token_uri = String::from_utf8(add_data.value).unwrap();
            } else if add_data.key == self.token_decimal_name {
                if add_data.value.len() != 1 {
                    return Err(anyhow!("Invalid token decimal length"));
                }
                info!("Token decimal: {:?}", add_data.value[0]);
                request_data.token_decimal = add_data.value[0];
            } else if add_data.key == self.solana_cluster_name {
                request_data.solana_cluster = SolanaCluster::from_str_name(String::from_utf8(add_data.value).unwrap().as_str());
            } else if add_data.key == self.paid_claiming_enabled_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw paid_claiming_enabled bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert raw bytes to u64 (little endian)
                    let mut bytes = add_data.value.clone();
                    // Pad to 8 bytes for u64
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    let value = u64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
                    request_data.paid_claiming_enabled = Some(value != 0);
                    info!("🔧 Parsed paid_claiming_enabled: {} (raw bytes: {:?}, value: {})", value != 0, add_data.value, value);
                } else {
                    info!("🔧 paid_claiming_enabled not provided, will use solver default");
                }
            } else if add_data.key == self.use_bonding_curve_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw use_bonding_curve bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert raw bytes to u64 (little endian)
                    let mut bytes = add_data.value.clone();
                    // Pad to 8 bytes for u64
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    let value = u64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
                    request_data.use_bonding_curve = Some(value != 0);
                    info!("🔧 Parsed use_bonding_curve: {} (raw bytes: {:?}, value: {})", value != 0, add_data.value, value);
                } else {
                    info!("🔧 use_bonding_curve not provided, will use solver default");
                }
            } else if add_data.key == self.curve_slope_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw curve_slope bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert raw bytes to u64 (little endian)
                    let mut bytes = add_data.value.clone();
                    // Pad to 8 bytes for u64
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    let curve_slope = u64::from_le_bytes(bytes.try_into().unwrap_or([1, 0, 0, 0, 0, 0, 0, 0]));
                    request_data.curve_slope = Some(curve_slope);
                    info!("🔧 Parsed curve_slope: {} (raw bytes: {:?})", curve_slope, add_data.value);
                } else {
                    info!("🔧 curve_slope not provided, will use solver default");
                }
            } else if add_data.key == self.base_price_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw base_price bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert raw bytes to u64 (little endian)
                    let mut bytes = add_data.value.clone();
                    // Pad to 8 bytes for u64
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    let base_price = u64::from_le_bytes(bytes.try_into().unwrap_or([1, 0, 0, 0, 0, 0, 0, 0]));
                    request_data.base_price = Some(base_price);
                    info!("🔧 Parsed base_price: {} (raw bytes: {:?})", base_price, add_data.value);
                } else {
                    info!("🔧 base_price not provided, will use solver default");
                }
                            } else if add_data.key == self.max_price_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw max_price bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert variable-length bytes to u64 (big endian)
                    let mut result = 0u64;
                    for (i, &byte) in add_data.value.iter().enumerate() {
                        result += (byte as u64) << ((add_data.value.len() - 1 - i) * 8);
                    }
                    let max_price = if result == 0 {
                        None
                    } else {
                        Some(result)
                    };
                    request_data.max_price = max_price;
                    info!("🔧 Parsed max_price: {:?} (raw bytes: {:?}, hex: 0x{:x})", max_price, add_data.value, result);
                } else {
                    info!("🔧 max_price not provided, will use solver default");
                }
            } else if add_data.key == self.flat_price_per_token_name {
                // Handle raw bytes from frontend (toHex() sends raw bytes, not hex strings)
                info!("🔧 Raw flat_price_per_token bytes: {:?}", add_data.value);
                // If empty, don't set the value (let solver defaults be used)
                if !add_data.value.is_empty() {
                    // Convert raw bytes to u64 (little endian)
                    let mut bytes = add_data.value.clone();
                    // Pad to 8 bytes for u64
                    while bytes.len() < 8 {
                        bytes.push(0);
                    }
                    let flat_price_per_token = u64::from_le_bytes(bytes.try_into().unwrap_or([1, 0, 0, 0, 0, 0, 0, 0]));
                    request_data.flat_price_per_token = Some(flat_price_per_token);
                    info!("🔧 Parsed flat_price_per_token: {} (raw bytes: {:?})", flat_price_per_token, add_data.value);
                } else {
                    info!("🔧 flat_price_per_token not provided, will use solver default");
                }
            }
        }
        // Check if the solana cluster is present in the request. If not, setting up the default one.
        if request_data.solana_cluster.is_none() {
            request_data.solana_cluster = SolanaCluster::from_str_name(self.default_solana_cluster.as_str());
        }
        
        // Log the final vamping parameters that will be used
        info!("🎯 Final vamping parameters for intent_id: 0x{}", hex::encode(&request_data.intent_id));
        info!("   paid_claiming_enabled: {:?}", request_data.paid_claiming_enabled);
        info!("   use_bonding_curve: {:?}", request_data.use_bonding_curve);
        info!("   curve_slope: {:?}", request_data.curve_slope);
        info!("   base_price: {:?}", request_data.base_price);
        info!("   max_price: {:?}", request_data.max_price);
        info!("   flat_price_per_token: {:?}", request_data.flat_price_per_token);
        let stats = self.stats.clone();
        let chain_id = request_data.chain_id;
        let erc20_address = request_data.erc20_address;
        match self.indexer.index_snapshot(request_data, stats.clone()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                if let Ok(mut stats) = stats.lock() {
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
