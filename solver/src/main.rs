use std::sync::{Arc, Mutex, RwLock};

use anyhow::{anyhow, Result};
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
use mysql_conn::DbConn;
use reqwest::StatusCode;
use snapshot_indexer::SnapshotIndexer;
use stats::{IndexerProcesses, cleanup_stats};
use tokio::{net::TcpListener, spawn};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::args::Args;

mod args;
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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let poll_frequency = parse_duration::parse(&args.poll_frequency_secs)?;
    let shared_args = Arc::new(RwLock::new(args));

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Initialize RabbitMQ listener
    let args_inst = shared_args.read().map_err(|e| anyhow!("Error accessing args: {}", e))?;
    let mut deploy_token_listener = request_registrator_listener::RequestRegistratorListener::new(
        args_inst.request_registrator_url.clone(),
        poll_frequency,
        DbConn::new(
            args_inst.mysql_host.clone(),
            args_inst.mysql_port.to_string(),
            args_inst.mysql_user.clone(),
            args_inst.mysql_password.clone(),
            args_inst.mysql_database.clone(),
        ),
    )
    .await?;

    // Initialize SnapshotIndexer
    let mut indexer = SnapshotIndexer::new(shared_args.clone())?;
    let quicknode_api_key = args_inst.quicknode_api_key.clone();
    if let Some(quicknode_api_key) = quicknode_api_key {
        if quicknode_api_key.len() == 0 {
            indexer.init_chain_info(None).await?;
        } else {
            indexer.init_chain_info(Some(quicknode_api_key)).await?;
        }
    }

    let indexer = Arc::new(indexer);

    let indexing_stats = Arc::new(Mutex::new(IndexerProcesses::new()));
    let deploy_token_handler = Arc::new(request_handler::DeployTokenHandler::new(
        indexer.clone(),
        indexing_stats.clone(),
        args_inst.default_solana_cluster.clone(),
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

    let shared_args_copy = shared_args.clone();
    let app = Router::new()
        .route("/", get(|| async { "Vamp.fun Solver" }))
        .route(
            "/get_claim_amount",
            get({
                async move |params| {
                    if let Ok(args_inst) = shared_args_copy.read() {
                        http_handler::handle_get_claim_amount(
                            params,
                            DbConn::new(
                                args_inst.mysql_host.clone(),
                                args_inst.mysql_port.to_string(),
                                args_inst.mysql_user.clone(),
                                args_inst.mysql_password.clone(),
                                args_inst.mysql_database.clone(),
                            ),
                        )
                    } else {
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }),
        )
        .route(
            "/vamping_stats",
            get(async move |params| http_handler::handle_get_stats(params, indexing_stats)),
        )
        .layer(cors);

    let port = args_inst.port;
    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    info!("Starting server at port {}", port);
    serve(tcp_listener, app).await.unwrap();
    Ok(())
}
