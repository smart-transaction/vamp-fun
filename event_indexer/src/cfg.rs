use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cfg {
    #[arg(long, env = "PORT", default_value_t = 8080)]
    pub port: u16,

    #[arg(long, env = "MYSQL_HOST")]
    pub mysql_host: String,

    #[arg(long, env = "MYSQL_PORT", default_value_t = 3306)]
    pub mysql_port: u16,

    #[arg(long, env = "MYSQL_USER")]
    pub mysql_user: String,

    #[arg(long, env = "MYSQL_PASSWORD")]
    pub mysql_password: String,

    #[arg(long, env = "MYSQL_DB")]
    pub mysql_db: String,

    #[arg(long, env = "ETH_RPC_URL")]
    pub eth_rpc_url: String,

    #[arg(long, env = "VAMP_CLONE_CONTRACT_ADDRESS")]
    pub vamp_clone_contract_address: String,

    #[arg(long, env = "VAMP_CLAIM_CONTRACT_ADDRESS")]
    pub vamp_claim_contract_address: String,

    #[arg(long, env = "CONFIRMATIONS", default_value_t = 12)]
    pub confirmations: u64,

    #[arg(long, env = "OVERLAP_BLOCKS", default_value_t = 50)]
    pub overlap_blocks: u64,
    
    #[arg(long, env = "MAX_BLOCK_RANGE", default_value_t = 2000)]
    pub max_block_range: u64,
    
    #[arg(long, env = "POLL_SECS", default_value_t = 5)]
    pub poll_secs: u64,
    
    #[arg(long, env = "DEPLOYMENT_BLOCK", default_value_t = 0)]
    pub deployment_block: u64,

    #[arg(long, env = "CHAIN_ID")]
    pub chain_id: u64,

    #[arg(long, env = "AMQP_HOST")]
    pub amqp_host: String,

    #[arg(long, env = "AMQP_PORT")]
    pub amqp_port: u16,

    #[arg(long, env = "AMQP_USER")]
    pub amqp_user: String,

    #[arg(long, env = "AMQP_PASSWORD")]
    pub amqp_password: String,

    #[arg(long, env = "EXCHANGE_NAME")]
    pub exchange_name: String,

    #[arg(long, env = "ROUTING_KEY")]
    pub routing_key: String,
}
