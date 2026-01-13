use anyhow::Result;
use axum::{
    routing::get,
    Router,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use clap::Parser;
use serde::Serialize;
use tokio::spawn;
use tracing::error;
use std::{net::SocketAddr, sync::Arc};

use crate::{cfg::Cfg, event_handler::ClaimHandler, event_subscriber::EventSubscriber};

mod cfg;
mod event_handler;
mod event_subscriber;
mod events;

#[tokio::main]
async fn main() -> Result<()> {
    // Create an instance of the config, parsing args
    let cfg = Arc::new(Cfg::parse());

    // Create the subscriber
    let mut event_subscriber = EventSubscriber::new(cfg.clone()).await?;

    let claim_handler = Arc::new(ClaimHandler::new(cfg.clone()));

    spawn(async move {
        if let Err(err) = event_subscriber.listen(claim_handler).await {
            error!("Error on events listering: {}", err);
        }
    });

    // Define the Routes
    let app = Router::new()
        .route("/health", get(health_check));

    // Define the address to listen on
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    // Create a TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // Start the server
    axum::serve(listener, app)
        .await?;

    Ok(())
}

// --- Handlers ---

// A simple struct to represent the JSON response
#[derive(Serialize)]
struct HealthResponse {
    status: String,
    message: String,
}

/// Handler for GET /health
async fn health_check() -> impl IntoResponse {
    let response = HealthResponse {
        status: "success".to_string(),
        message: "Service is healthy".to_string(),
    };

    // Return a tuple of (HTTP Status, JSON Body)
    (StatusCode::OK, Json(response))
}