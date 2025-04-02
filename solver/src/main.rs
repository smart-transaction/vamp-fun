use std::{collections::HashMap, error::Error, sync::Arc};

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
use ethers::types::{Address, U256};
use log::{Level, info};
use snapshot_indexer::SnapshotIndexer;
use stderrlog::Timestamp;
use tokio::{net::TcpListener, spawn, sync::{mpsc, Mutex}};
use tower_http::cors::{Any, CorsLayer};

mod appchain_listener;
mod chain_info;
mod merkle_tree;
mod request_handlers;
mod snapshot_indexer;
mod snapshot_processor;
mod use_proto;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value_t = 9000)]
    pub port: u16,

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    stderrlog::new()
        .verbosity(Level::Info)
        .timestamp(Timestamp::Millisecond)
        .init()
        .unwrap();

    // Initialize RabbitMQ listener
    let mut deploy_token_listener =
        appchain_listener::RabbitMQListener::new("DeployToken", "DefaultSolver").await?;

    let (tx, rx) = mpsc::channel::<HashMap<Address, U256>>(100);

    // Initialize SnapshotIndexer
    let mut indexer = SnapshotIndexer::new(
        args.mysql_host,
        args.mysql_port,
        args.mysql_user,
        args.mysql_password,
        args.mysql_database,
        tx.clone(),
    );
    indexer.init_chain_info().await?;

    let indexer = Arc::new(indexer);

    let deploy_token_handler = Arc::new(Mutex::new(request_handlers::DeployTokenHandler::new(
        indexer.clone(),
    )));

    spawn(async move {
        deploy_token_listener
            .listen(deploy_token_handler.clone())
            .await;
    });

    spawn(async move {
        snapshot_processor::listen_indexed_snapshot(rx).await;
    });

    // Start HTTP server
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers([ACCEPT, ACCEPT_LANGUAGE, CONTENT_LANGUAGE, CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(|| async { "Vamp.fun Solver" }))
        .layer(cors);

    let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();

    info!("Starting server at port {}", args.port);
    serve(tcp_listener, app).await.unwrap();
    Ok(())
}
