use std::sync::{Arc, Mutex};

use anyhow::Result;
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
use snapshot_indexer::SnapshotIndexer;
use stats::{IndexerProcesses, cleanup_stats};
use tokio::{net::TcpListener, spawn};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::{args::Args, db_init::init_db};

mod args;
mod chain_info;
mod db_init;
mod http_handler;
mod mysql_conn;
mod request_handler;
mod request_registrator_listener;
mod snapshot_indexer;
mod snapshot_processor;
mod solana_transaction;
mod stats;
mod vamper_event;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Arc::new(Args::parse());

    init_db(args.clone()).await?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Initialize RabbitMQ listener
    let mut deploy_token_listener = request_registrator_listener::RequestRegistratorListener::new(
        args.clone()
    )
    .await?;

    // Initialize SnapshotIndexer
    let indexer = Arc::new(SnapshotIndexer::new(args.clone()).await?);

    let indexing_stats = Arc::new(Mutex::new(IndexerProcesses::new()));
    let deploy_token_handler = Arc::new(request_handler::DeployTokenHandler::new(
        args.clone(),
        indexer.clone(),
        indexing_stats.clone(),
        &args.default_solana_cluster,
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

    let shared_args_copy = args.clone();
    let app = Router::new()
        .route("/", get(|| async { "Vamp.fun Solver" }))
        .route(
            "/get_claim_amount",
            get({
                async move |params| {
                    http_handler::handle_get_claim_amount(
                        params,
                        DbConn::new(
                            &shared_args_copy.mysql_host,
                            shared_args_copy.mysql_port,
                            &shared_args_copy.mysql_user,
                            &shared_args_copy.mysql_password,
                            &shared_args_copy.mysql_database,
                        ),
                    ).await
                }
            }),
        )
        .route(
            "/vamping_stats",
            get(async move |params| http_handler::handle_get_stats(params, indexing_stats)),
        )
        .layer(cors);

    let port = args.port;
    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    info!("Starting server at port {}", port);
    serve(tcp_listener, app).await.unwrap();
    Ok(())
}
