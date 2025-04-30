use std::{
    error::Error,
    sync::{Arc, Mutex},
};

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
use log::{Level, error, info};
use snapshot_indexer::SnapshotIndexer;
use stats::{cleanup_stats, IndexerProcesses};
use stderrlog::Timestamp;
use tokio::{net::TcpListener, spawn};
use tower_http::cors::{Any, CorsLayer};

mod chain_info;
mod http_handler;
mod mysql_conn;
mod request_handler;
mod request_registrator_listener;
mod snapshot_indexer;
mod snapshot_processor;
mod stats;
mod use_proto;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value_t = 9000)]
    pub port: u16,

    #[arg(long)]
    pub request_registrator_url: String,

    #[arg(long)]
    pub orchestrator_url: String,

    #[arg(long, default_value = "40s")]
    pub poll_frequency_secs: String,

    #[arg(long)]
    pub mysql_user: String,

    #[arg(long)]
    pub mysql_password: String,

    #[arg(long)]
    pub mysql_host: String,

    #[arg(long, default_value_t = 3306)]
    pub mysql_port: u16,

    #[arg(long)]
    pub mysql_database: String,

    #[arg(long)]
    pub quicknode_api_key: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let poll_frequency = parse_duration::parse(&args.poll_frequency_secs)?;

    stderrlog::new()
        .verbosity(Level::Info)
        .timestamp(Timestamp::Millisecond)
        .show_module_names(true)
        .init()
        .unwrap();

    // Initialize RabbitMQ listener
    let mut deploy_token_listener = request_registrator_listener::RequestRegistratorListener::new(
        args.request_registrator_url,
        poll_frequency,
        args.mysql_host.clone(),
        args.mysql_port.to_string(),
        args.mysql_user.clone(),
        args.mysql_password.clone(),
        args.mysql_database.clone(),
    )
    .await?;

    // Initialize SnapshotIndexer
    let mut indexer = SnapshotIndexer::new(
        args.mysql_host.clone(),
        args.mysql_port.clone(),
        args.mysql_user.clone(),
        args.mysql_password.clone(),
        args.mysql_database.clone(),
        args.orchestrator_url.clone(),
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
                        args.mysql_host,
                        args.mysql_port.to_string(),
                        args.mysql_user,
                        args.mysql_password,
                        args.mysql_database,
                    )
                }
            }),
        )
        .route("/vamping_stats", get({ async move |params| {
            http_handler::handle_get_stats(params, indexing_stats)
        } }))
        .layer(cors);

    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();

    info!("Starting server at port {}", args.port);
    serve(tcp_listener, app).await.unwrap();
    Ok(())
}
