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
use stderrlog::Timestamp;
use tokio::{net::TcpListener, spawn, sync::Mutex};
use tower_http::cors::{Any, CorsLayer};

mod appchain_listener;
mod merkle_tree;
mod state_snapshot_handler;

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

    let mut state_snapshot_listener =
        appchain_listener::RabbitMQListener::new("StateSnapshot", "DefaultSolver").await?;

    let state_snapshot_handler = Arc::new(Mutex::new(
        state_snapshot_handler::StateSnapshotHandler::new(),
    ));
    spawn(async move {
        state_snapshot_listener.listen(state_snapshot_handler.clone()).await;
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
