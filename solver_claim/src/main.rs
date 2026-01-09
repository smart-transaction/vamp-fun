use axum::{
    routing::get,
    Router,
    response::{IntoResponse, Json},
    http::StatusCode,
};
use serde::Serialize;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // 1. Define the Routes
    let app = Router::new()
        .route("/health", get(health_check));

    // 2. Define the address to listen on
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);

    // 3. Create a TCP listener
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // 4. Start the server
    axum::serve(listener, app)
        .await
        .unwrap();
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