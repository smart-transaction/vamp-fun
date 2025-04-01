use std::{error::Error, sync::Arc};

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
use log::{Level, info};
use snapshot_indexer::SnapshotIndexer;
use stderrlog::Timestamp;
use tokio::{net::TcpListener, spawn, sync::Mutex};
use tower_http::cors::{Any, CorsLayer};

mod appchain_listener;
mod merkle_tree;
mod request_handlers;
mod snapshot_indexer;
mod use_proto;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(long, default_value_t = 9000)]
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    stderrlog::new()
        .verbosity(Level::Info)
        .timestamp(Timestamp::Millisecond)
        .init()
        .unwrap();

    let mut deploy_token_listener =
        appchain_listener::RabbitMQListener::new("DeployToken", "DefaultSolver").await?;

    let indexer = Arc::new(SnapshotIndexer::new());

    let deploy_token_handler =
        Arc::new(Mutex::new(request_handlers::DeployTokenHandler::new(indexer.clone())));

    spawn(async move {
        deploy_token_listener
            .listen(deploy_token_handler.clone())
            .await;
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
