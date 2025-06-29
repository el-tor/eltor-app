use axum::{
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use eltor_backend::state::{AppState, MessageResponse, StatusResponse};
use std::env;
use tower_http::cors::CorsLayer;

use eltor_backend::routes::eltor::get_bin_dir;
use eltor_backend::routes::ip;

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
    // Load environment variables from root .env file
    dotenv::from_path("../.env").ok();
    // Print environment variables for debugging
    println!("🔧 Backend Environment variables:");
    for (key, value) in env::vars() {
        if key.starts_with("APP_") || key.starts_with("BACKEND_") || key.starts_with("PHOENIXD_") {
            println!("   {}: {}", key, value);
        }
    }

    // Clean up any processes using our ports
    println!("🧹 Starting port cleanup...");
    if let Err(e) = eltor_backend::ports::cleanup_ports_startup().await {
        eprintln!("⚠️  Port cleanup failed: {}", e);
        eprintln!("   Continuing with startup...");
    }

    // Read environment variables
    let backend_port = env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "5174".to_string())
        .parse::<u16>()
        .unwrap_or(5174);

    let use_phoenixd_embedded = env::var("APP_ELTOR_USE_PHOENIXD_EMBEDDED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    println!("🔧 Backend configuration:");
    println!("   Port: {}", backend_port);
    println!("   Phoenixd embedded: {}", use_phoenixd_embedded);

    // Initialize shared state
    let mut state = AppState::new(use_phoenixd_embedded);

    // Initialize Lightning node
    println!("⚡ Initializing Lightning node...");

    // Initialize from torrc
    let bin_dir = get_bin_dir();
    let torrc_path = bin_dir.join("data").join("torrc");
    println!("🔍 Looking for torrc file at: {:?}", torrc_path);
    let lightning_node = match eltor_backend::lightning::LightningNode::from_torrc(&torrc_path) {
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
        match eltor_backend::wallet::start_phoenixd(state.clone()).await {
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
    let cors = CorsLayer::permissive();

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/tor/connect", post(connect_tor))
        .route("/api/tor/disconnect", post(disconnect_tor))
        .route("/api/tor/status", get(get_tor_status))
        .route("/api/ip/:ip", get(ip::get_ip_location))
        .route("/api/ip/bulk", post(ip::get_bulk_ip_locations))
        .merge(eltor_backend::routes::eltor::create_routes())
        .merge(eltor_backend::routes::wallet::create_routes())
        .layer(cors)
        .with_state(state);

    // Start the server
    let bind_address = format!("0.0.0.0:{}", backend_port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();
    println!("🚀 El Tor Backend Server");
    println!("📡 Running on http://0.0.0.0:{}", backend_port);
    println!("🔗 Health check: http://localhost:{}/health", backend_port);
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
