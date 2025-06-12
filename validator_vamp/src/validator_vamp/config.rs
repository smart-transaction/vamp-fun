use std::fs;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub binding_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    pub redis_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IpfsConfig {
    pub api_url: String,
    pub gateway_url: String,
    pub pin: bool,
    pub enable_mfs_copy: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub grpc: GrpcConfig,
    pub storage: StorageConfig,
    pub ipfs: IpfsConfig,
}

pub fn load_config(config_file_path: &str) -> Config {
    let config_content = fs::read_to_string(config_file_path)
        .expect("Failed to read config file");
    toml::from_str(&config_content).expect("Invalid config format")
}
