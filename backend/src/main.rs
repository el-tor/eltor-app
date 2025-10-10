use axum::{
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use eltor_backend::{setup_broadcast_logger, state::MessageResponse};
use std::env;
use tower_http::cors::CorsLayer;

use eltor_backend::routes::ip;
use eltor_backend::static_files;

async fn health_check() -> ResponseJson<MessageResponse> {
    ResponseJson(MessageResponse {
        message: "Backend server is running".to_string(),
    })
}

#[tokio::main]
async fn main() {
    // Load environment variables from root .env file
    dotenv::from_path("../.env").ok();

    // Read environment variables first to configure state properly
    let use_phoenixd_embedded = env::var("APP_ELTOR_USE_PHOENIXD_EMBEDDED")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Create app state
    let mut state = eltor_backend::create_app_state(use_phoenixd_embedded);

    // Set up custom logger to capture ALL logs (including from eltor library) BEFORE anything else
    if let Err(e) = setup_broadcast_logger(state.clone()) {
        eprintln!("⚠️  Failed to set up broadcast logger: {}", e);
        eprintln!("   Eltor logs will go to stdout, only manual logs will stream to SSE");
    }

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
        .or_else(|_| env::var("PORT")) // Also check for standard PORT env var
        .unwrap_or_else(|_| "5174".to_string())
        .parse::<u16>()
        .unwrap_or(5174);

    // Kill any process using the backend port before starting
    println!("🔧 Cleaning up backend port {}...", backend_port);
    if let Err(e) = eltor_backend::ports::cleanup_backend_port(backend_port).await {
        eprintln!("⚠️  Backend port cleanup failed: {}", e);
        eprintln!("   Continuing with startup... server may fail to bind if port is still in use");
    }

    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());

    println!("🔧 Backend configuration:");
    println!("   Bind Address: {}", bind_address);
    println!("   Port: {}", backend_port);
    println!("   Phoenixd embedded: {}", use_phoenixd_embedded);

    // Initialize Lightning node
    println!("⚡ Initializing Lightning node...");

    // Initialize from torrc using PathConfig
    let path_config = eltor_backend::PathConfig::new().unwrap_or_else(|e| {
        eprintln!("⚠️  Warning: Failed to get path config: {}", e);
        eprintln!("   Using default paths");
        eltor_backend::PathConfig::with_overrides(
            Some(std::env::current_dir().unwrap().join("bin")),
            Some(std::env::current_dir().unwrap().join("bin/data")),
        )
        .unwrap()
    });
    let torrc_path = path_config.get_torrc_path(None);
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

    // Initialize shared EltorManager
    let state_arc = std::sync::Arc::new(tokio::sync::RwLock::new(state.clone()));
    let eltor_manager = eltor_backend::eltor::EltorManager::new(
        state_arc,
        path_config.clone(),
    );
    state.set_eltor_manager(eltor_manager);

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

    // Initialize IP location database
    println!("🗺️  Initializing IP location database...");
    let ip_db_path = path_config.get_executable_path("IP2LOCATION-LITE-DB3.BIN");
    println!("   Looking for IP database at: {:?}", ip_db_path);
    
    // Debug: List files in the bin directory
    if let Some(bin_dir) = ip_db_path.parent() {
        println!("   📁 Contents of bin directory {:?}:", bin_dir);
        if bin_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(bin_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let metadata = entry.metadata();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    println!("      - {:?} ({} bytes)", file_name, size);
                }
            } else {
                println!("      ⚠️  Could not read directory");
            }
        } else {
            println!("      ⚠️  Directory does not exist!");
        }
    }
    
    println!("   IP database exists: {}", ip_db_path.exists());
    
    if ip_db_path.exists() {
        match ip::init_ip_database(ip_db_path.clone()) {
            Ok(()) => println!("   ✅ IP database initialized successfully"),
            Err(e) => {
                eprintln!("   ⚠️  Failed to initialize IP database: {}", e);
                eprintln!("   IP geolocation features will be unavailable");
            }
        }
    } else {
        eprintln!("   ⚠️  IP database not found at: {}", ip_db_path.display());
        eprintln!("   IP geolocation features will be unavailable");
        eprintln!("   To enable: download IP2LOCATION-LITE-DB3.BIN and place in bin directory");
    }

    // Configure CORS to allow SSE
    let cors = CorsLayer::permissive();

    // Build the router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/ip/:ip", get(ip::get_ip_location))
        .route("/api/ip/bulk", post(ip::get_bulk_ip_locations))
        .merge(eltor_backend::routes::eltor::create_routes())
        .merge(eltor_backend::routes::wallet::create_routes())
        .merge(eltor_backend::routes::phoenix::create_routes())
        .merge(eltor_backend::routes::debug::create_routes())
        // Serve static frontend files (this should be last to catch all non-API routes)
        .fallback(static_files::serve_static)
        .layer(cors)
        .with_state(state);

    // Start the server
    let full_bind_address = format!("{}:{}", bind_address, backend_port);
    let listener = tokio::net::TcpListener::bind(&full_bind_address)
        .await
        .unwrap();

    // Determine the display URL based on bind address
    let display_address = if bind_address == "0.0.0.0" {
        format!("http://0.0.0.0:{}", backend_port)
    } else {
        format!("http://{}:{}", bind_address, backend_port)
    };

    let local_url = if bind_address == "0.0.0.0" {
        format!("http://localhost:{}", backend_port)
    } else {
        format!("http://{}:{}", bind_address, backend_port)
    };

    println!("🚀 El Tor Backend Server");
    println!("📡 Running on {}", display_address);
    println!("🌐 Frontend served at {}", local_url);
    println!("🔗 Health check: {}/health", local_url);
    println!("📋 API endpoints:");
    println!("   POST /api/eltord/activate/:mode");
    println!("   POST /api/eltord/deactivate/:mode");
    println!("   GET  /api/eltord/status");
    println!("   GET  /api/eltord/logs");
    println!("   GET  /api/wallet/info");
    println!("   GET  /api/wallet/balance");
    println!("   POST /api/wallet/invoice");
    // println!("   POST /api/wallet/pay");
    println!("   POST /api/wallet/offer");
    println!("   GET  /api/wallet/status");
    println!("   GET  /api/wallet/transactions");
    println!("   PUT  /api/wallet/config");
    println!("   DELETE /api/wallet/config");
    println!("   GET  /api/wallet/configs");
    println!("   POST /api/phoenix/start");
    println!("   POST /api/phoenix/stop");
    println!("   GET  /api/debug");
    println!("📁 Static files served from frontend/dist/");
    println!("🔧 Environment variables injected into frontend:");
    println!("   BACKEND_PORT: {}", backend_port);
    println!(
        "   BACKEND_URL: {}",
        env::var("BACKEND_URL").unwrap_or_else(|_| local_url.clone())
    );
    println!("   BIND_ADDRESS: {}", bind_address);

    axum::serve(listener, app).await.unwrap();
}
