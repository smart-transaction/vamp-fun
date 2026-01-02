use clap::Parser;
use ethers::signers::LocalWallet;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, env = "PORT", default_value_t = 9000)]
    pub port: u16,

    #[arg(long, env = "SOLANA_DEVNET_URL")]
    pub solana_devnet_url: String,

    #[arg(long, env = "SOLANA_MAINNET_URL")]
    pub solana_mainnet_url: String,

    #[arg(long, env = "MYSQL_USER")]
    pub mysql_user: String,

    #[arg(long, env = "MYSQL_PASSWORD")]
    pub mysql_password: String,

    #[arg(long, env = "MYSQL_HOST")]
    pub mysql_host: String,

    #[arg(long, env = "MYSQL_PORT", default_value_t = 3306)]
    pub mysql_port: u16,

    #[arg(long, env = "MYSQL_DATABASE")]
    pub mysql_database: String,

    #[arg(long, env = "QUICKNODE_API_KEY")]
    pub quicknode_api_key: Option<String>,

    #[arg(long, env = "ETHEREUM_PRIVATE_KEY")]
    pub ethereum_private_key: LocalWallet,

    #[arg(long, env = "SOLANA_PRIVATE_KEY")]
    pub solana_private_key: String,

    #[arg(long, env = "DEFAULT_SOLANA_CLUSTER")]
    pub default_solana_cluster: String,

    // Vamping configuration parameters
    #[arg(long, env = "PAID_CLAIMING_ENABLED", default_value_t = false, num_args(0..=1), value_parser = clap::value_parser!(bool))]
    pub paid_claiming_enabled: bool,

    #[arg(long, env = "USE_BONDING_CURVE", default_value_t = false, num_args(0..=1), value_parser = clap::value_parser!(bool))]
    pub use_bonding_curve: bool,

    #[arg(long, env = "CURVE_SLOPE", default_value_t = 1)]
    pub curve_slope: u64,

    #[arg(long, env = "BASE_PRICE", default_value_t = 1)]
    pub base_price: u64,

    #[arg(long, env = "MAX_PRICE", default_value_t = 1000)]
    pub max_price: u64,

    #[arg(long, env = "FLAT_PRICE_PER_TOKEN", default_value_t = 1)]
    pub flat_price_per_token: u64,

    // RabbitMQ queue params
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

    #[arg(long, env = "QUEUE_NAME")]
    pub queue_name: String,

    #[arg(long, env = "ROUTING_KEY")]
    pub routing_key: String,
}
