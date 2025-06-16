use axum::{
    http::{HeaderName, HeaderValue, Method},
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::env;
use tower_http::cors::CorsLayer;

mod lightning;
mod ports;
mod routes;
mod state;
mod torrc_parser;
mod wallet;

use state::{AppState, MessageResponse, StatusResponse};

use crate::routes::eltor::get_bin_dir;
use crate::routes::ip;

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
    if let Err(e) = crate::ports::cleanup_ports_startup().await {
        eprintln!("⚠️  Port cleanup failed: {}", e);
        eprintln!("   Continuing with startup...");
    }

    // Read USE_PHOENIXD_EMBEDDED from environment (default to true)
    let use_phoenixd_embedded = env::var("USE_PHOENIXD_EMBEDDED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    println!(
        "🔧 Wallet configuration: phoenixd embedded = {}",
        use_phoenixd_embedded
    );

    // Initialize shared state
    let mut state = AppState::new(use_phoenixd_embedded);

    // Initialize Lightning node
    println!("⚡ Initializing Lightning node...");

    // Initialize from torrc
    let torrc_path = "bin/torrc";
    let lightning_node = match crate::lightning::LightningNode::from_torrc(torrc_path) {
        Ok(node) => {
            println!(
                "✅ Lightning node connected from torrc ({})",
                node.node_type()
            );
            Some(node)
        }
        Err(e) => {
            println!("⚠️  Failed to initialize Lightning node from torrc: {}", e);
            // You can choose to exit or handle the error as needed.
            // For now, we'll just continue without a Lightning node.
            None
        }
    };

    if let Some(node) = lightning_node {
        state.set_lightning_node(node);
    }

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

    // Initialize IP database
    let ip_db_path = get_bin_dir().join("IP2LOCATION-LITE-DB3.BIN");
    if ip_db_path.exists() {
        if let Err(e) = ip::init_ip_database(ip_db_path) {
            eprintln!("⚠️  Failed to initialize IP database: {}", e);
        }
    } else {
        eprintln!("⚠️  IP database not found at: {}", ip_db_path.display());
        eprintln!("   Download IP2LOCATION-LITE-DB3.BIN to enable IP geolocation");
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
        .route("/api/ip/:ip", get(ip::get_ip_location))
        .route("/api/ip/bulk", post(ip::get_bulk_ip_locations))
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
    println!("   POST /api/eltord/activate/:mode");
    println!("   POST /api/eltord/activate/:mode/:torrc_file_name");
    println!("   POST /api/eltord/deactivate");
    println!("   GET  /api/eltord/status");
    println!("   GET  /api/eltord/logs (SSE)");
    println!("   GET  /api/wallet/info");
    println!("   GET  /api/wallet/balance");
    println!("   POST /api/wallet/invoice");
    println!("   POST /api/wallet/pay");
    println!("   POST /api/wallet/offer");
    println!("   GET  /api/wallet/status");
    println!("   GET  /api/wallet/transactions");

    axum::serve(listener, app).await.unwrap();
}
