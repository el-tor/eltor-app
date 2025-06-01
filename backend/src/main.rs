use axum::{
    http::{HeaderValue, Method, HeaderName},
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use dotenv::dotenv;
use std::env;

mod state;
mod routes;
mod wallet;
mod ports;

use state::{AppState, StatusResponse, MessageResponse};

// Tor-related handlers
async fn connect_tor() -> ResponseJson<StatusResponse> {
    println!("🔗 Connecting to Tor...");
    // TODO: Implement actual Tor connection logic
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

async fn disconnect_tor() -> ResponseJson<StatusResponse> {
    println!("🔌 Disconnecting from Tor...");
    // TODO: Implement actual Tor disconnection logic
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

async fn get_tor_status() -> ResponseJson<StatusResponse> {
    // TODO: Implement actual Tor status check
    ResponseJson(StatusResponse {
        connected: false,
        circuit: None,
    })
}

async fn health_check() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Backend server is running".to_string(),
    })
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Clean up any processes using our ports
    println!("🧹 Starting port cleanup...");
    if let Err(e) = crate::ports::cleanup_ports().await {
        eprintln!("⚠️  Port cleanup failed: {}", e);
        eprintln!("   Continuing with startup...");
    }
    
    // Read USE_PHOENIXD_EMBEDDED from environment (default to true)
    let use_phoenixd_embedded = env::var("USE_PHOENIXD_EMBEDDED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    println!("🔧 Wallet configuration: phoenixd embedded = {}", use_phoenixd_embedded);

    // Initialize shared state
    let state = AppState::new(use_phoenixd_embedded);

    // Start phoenixd if embedded mode is enabled
    if use_phoenixd_embedded {
        println!("🚀 Starting embedded phoenixd...");
        match crate::wallet::start_phoenixd(state.clone()).await {
            Ok(()) => println!("✅ Phoenixd started successfully"),
            Err(e) => {
                eprintln!("❌ Failed to start phoenixd: {}", e);
                eprintln!("   Continuing without embedded phoenixd...");
            }
        }
    } else {
        println!("🔗 Using external phoenixd instance");
    }

    // Configure CORS to allow SSE
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            HeaderName::from_static("content-type"),
            HeaderName::from_static("cache-control"),
        ])
        .allow_credentials(true);

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/tor/connect", post(connect_tor))
        .route("/api/tor/disconnect", post(disconnect_tor))
        .route("/api/tor/status", get(get_tor_status))
        .merge(routes::eltor::create_routes())
        .merge(routes::wallet::create_routes())
        .layer(cors)
        .with_state(state);

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("🚀 El Tor Backend Server");
    println!("📡 Running on http://localhost:8080");
    println!("🔗 Health check: http://localhost:8080/health");
    println!("📋 API endpoints:");
    println!("   POST /api/tor/connect");
    println!("   POST /api/tor/disconnect");
    println!("   GET  /api/tor/status");
    println!("   POST /api/eltord/activate");
    println!("   POST /api/eltord/deactivate");
    println!("   GET  /api/eltord/status");
    println!("   GET  /api/eltord/logs (SSE)");
    println!("   GET  /api/wallet/balance");
    println!("   POST /api/wallet/send");
    println!("   POST /api/wallet/receive");
    println!("   GET  /api/wallet/status");
    
    axum::serve(listener, app).await.unwrap();
}
