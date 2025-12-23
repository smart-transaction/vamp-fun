use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use anchor_client::{Client, Cluster, Program};
use anchor_lang::declare_program;
use axum::{
    Router,
    http::{
        Method,
        header::{ACCEPT, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, CONTENT_TYPE},
    },
    routing::get,
    serve,
};
use clap::Parser;
use ethers::signers::LocalWallet;
use mysql_conn::DbConn;
use snapshot_indexer::SnapshotIndexer;
use solana_sdk::signature::Keypair;
use stats::{IndexerProcesses, cleanup_stats};
use tokio::{net::TcpListener, spawn};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod chain_info;
mod http_handler;
mod mysql_conn;
mod request_handler;
mod request_registrator_listener;
mod send_transaction;
mod snapshot_indexer;
mod snapshot_processor;
mod stats;
mod use_proto;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, env = "PORT", default_value_t = 9000)]
    pub port: u16,

    #[arg(long, env = "REQUEST_REGISTRATOR_URL")]
    pub request_registrator_url: String,

    #[arg(long, env = "VALIDATOR_URL")]
    pub validator_url: String,
    
    #[arg(long, env = "ORCHESTRATOR_URL")]
    pub orchestrator_url: String,

    #[arg(long, env = "POLL_FREQUENCY_SECS", default_value = "40s")]
    pub poll_frequency_secs: String,

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let poll_frequency = parse_duration::parse(&args.poll_frequency_secs)?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Initialize RabbitMQ listener
    let mut deploy_token_listener = request_registrator_listener::RequestRegistratorListener::new(
        args.request_registrator_url,
        poll_frequency,
        DbConn::new(
            args.mysql_host.clone(),
            args.mysql_port.to_string(),
            args.mysql_user.clone(),
            args.mysql_password.clone(),
            args.mysql_database.clone(),
        ),
    )
    .await?;

    // Initialize Solana client
    let solana_payer_keypair = Arc::new(Keypair::from_base58_string(&args.solana_private_key));
    let solana_program = Arc::new(get_program_instance(solana_payer_keypair.clone())?);

    // Initialize SnapshotIndexer
    let mut indexer = SnapshotIndexer::new(
        DbConn::new(
            args.mysql_host.clone(),
            args.mysql_port.to_string(),
            args.mysql_user.clone(),
            args.mysql_password.clone(),
            args.mysql_database.clone(),
        ),
        args.validator_url.clone(),
        args.orchestrator_url.clone(),
        args.private_key.clone(),
        solana_payer_keypair.clone(),
        solana_program.clone(),
        args.paid_claiming_enabled,
        args.use_bonding_curve,
        args.curve_slope,
        args.base_price,
        args.flat_price_per_token,
        // pass overrides
        args.override_paid_claiming_enabled,
        args.override_use_bonding_curve,
        args.override_curve_slope,
        args.override_base_price,
        args.override_max_price,
        args.override_flat_price_per_token,
    );
    if let Some(quicknode_api_key) = args.quicknode_api_key {
        if quicknode_api_key.len() == 0 {
            indexer.init_chain_info(None).await?;
        } else {
            indexer.init_chain_info(Some(quicknode_api_key)).await?;
        }
    } else {
        indexer.init_chain_info(None).await?;
    }

    let indexer = Arc::new(indexer);

    let indexing_stats = Arc::new(Mutex::new(IndexerProcesses::new()));
    let deploy_token_handler = Arc::new(request_handler::DeployTokenHandler::new(
        indexer.clone(),
        indexing_stats.clone(),
        args.default_solana_cluster.clone(),
    ));

    spawn(async move {
        if let Err(err) = deploy_token_listener.listen(deploy_token_handler).await {
            error!("Failed to listen to request registrator: {:?}", err);
        }
    });

    // Set up stats cleanup
    let indexing_stats_copy = indexing_stats.clone();
    spawn(async move {
        cleanup_stats(indexing_stats_copy).await;
    });

    // Start HTTP server
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([ACCEPT, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(|| async { "Vamp.fun Solver" }))
        .route(
            "/get_claim_amount",
            get({
                async move |params| {
                    http_handler::handle_get_claim_amount(
                        params,
                        DbConn::new(
                            args.mysql_host.clone(),
                            args.mysql_port.to_string(),
                            args.mysql_user.clone(),
                            args.mysql_password.clone(),
                            args.mysql_database.clone(),
                        ),
                    )
                }
            }),
        )
        .route(
            "/vamping_stats",
            get(async move |params| http_handler::handle_get_stats(params, indexing_stats)),
        )
        .layer(cors);

    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();

    info!("Starting server at port {}", args.port);
    serve(tcp_listener, app).await.unwrap();
    Ok(())
}

declare_program!(solana_vamp_program);

fn get_program_instance(
    payer_keypair: Arc<Keypair>,
) -> Result<Program<Arc<Keypair>>, Box<dyn Error>> {
    // The cluster doesn't matter here, it's used only for the instructions creation.
    let anchor_client = Client::new(Cluster::Debug, payer_keypair.clone());
    Ok(anchor_client.program(solana_vamp_program::ID)?)
}
