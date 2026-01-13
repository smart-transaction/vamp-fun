use alloy::signers::local::PrivateKeySigner;
use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cfg {
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
    pub ethereum_private_key: PrivateKeySigner,

    #[arg(long, env = "SOLANA_PRIVATE_KEY")]
    pub solana_private_key: String,

    #[arg(long, env = "DEFAULT_SOLANA_CLUSTER")]
    pub default_solana_cluster: String,

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
