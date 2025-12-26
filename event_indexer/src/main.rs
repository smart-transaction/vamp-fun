use std::{net::SocketAddr, sync::Arc};

use alloy_provider::ProviderBuilder;
use anyhow::{Result, anyhow};
use clap::Parser;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Serialize;
use sqlx::MySqlPool;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use urlencoding::encode;

use crate::{app_state::AppState, cfg::Cfg, eth_client::EthClient, indexer::{ensure_checkpoint_row, indexer_loop}};

mod app_state;
mod cfg;
mod eth_client;
mod event_publisher;
mod indexer;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

fn get_mysql_url(args: Arc<Cfg>) -> Result<String> {
    let encoded_password = encode(&args.mysql_password);
    let mysql_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        args.mysql_user, encoded_password, args.mysql_host, args.mysql_port, args.mysql_db
    );
    Ok(mysql_url)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Logging: controlled via RUST_LOG, e.g. RUST_LOG=info,indexer=debug
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Arc::new(Cfg::parse());

    let mysql_url = get_mysql_url(args.clone())?;

    let db = MySqlPool::connect(&mysql_url).await.map_err(|e| anyhow!("connect mysql: {}", e))?;
    ensure_checkpoint_row(&db).await?;

    let provider = ProviderBuilder::new()
        .connect(&args.eth_rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect provider: {e}"))?;

    let eth = Arc::new(EthClient { provider: Arc::new(provider) });

    let state = AppState { db, eth, cfg: args.clone() };

    tokio::spawn(indexer_loop(state.clone()));

    // HTTP server
    let app = Router::new()
        .route("/health", get(health))
        .with_state(state);

    let addr: SocketAddr = format!("0.0.0.0:{}", args.port).parse().expect("valid listen addr");
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind to address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

    info!("shutdown complete");

    Ok(())
}

async fn health(State(_state): State<AppState>) -> impl IntoResponse {
    // You can extend this to check db connectivity, provider liveness, etc.
    let body = axum::Json(HealthResponse { status: "ok" });
    (StatusCode::OK, body)
}

// Graceful shutdown on Ctrl+C or SIGTERM
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        sigterm.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("shutdown signal received");
}
