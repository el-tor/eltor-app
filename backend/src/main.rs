use axum::{
    response::Json as ResponseJson,
    routing::{get, post},
    Router,
};
use eltor_backend::state::MessageResponse;
use std::env;
use tower_http::cors::CorsLayer;
use log::info;

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

    // Create PathConfig once at startup (blocking I/O happens here, not in request handlers)
    let path_config = eltor_backend::PathConfig::new().unwrap_or_else(|e| {
        info!("⚠️  Warning: Failed to get path config: {}", e);
        info!("   Using default paths");
        eltor_backend::PathConfig::with_overrides(
            Some(std::env::current_dir().unwrap().join("bin")),
            Some(std::env::current_dir().unwrap().join("bin/data")),
        )
        .unwrap()
    });

    // Create app state with PathConfig
    let mut state = eltor_backend::create_app_state(use_phoenixd_embedded, path_config.clone());

    // Set up custom logger to capture ALL logs (including from eltor library) BEFORE anything else
    // if let Err(e) = setup_broadcast_logger(state.clone()) {
    //     info!("⚠️  Failed to set up broadcast logger: {}", e);
    //     info!("   Eltor logs will go to stdout, only manual logs will stream to SSE");
    // }

    // Print environment variables for debugging
    info!("🔧 Backend Environment variables:");
    for (key, value) in env::vars() {
        if key.starts_with("APP_") || key.starts_with("BACKEND_") || key.starts_with("PHOENIXD_") {
            info!("   {}: {}", key, value);
        }
    }

    // Show Arti configuration
    let arti_socks_port = env::var("APP_ARTI_SOCKS_PORT").unwrap_or_else(|_| "18050".to_string());
    info!("🔧 Arti SOCKS proxy port: {}", arti_socks_port);
    
    // Show SOCKS router configuration
    let socks_router_port = env::var("APP_SOCKS_ROUTER_PORT").unwrap_or_else(|_| "18049".to_string());
    let eltord_socks_port = env::var("APP_ELTORD_SOCKS_PORT").unwrap_or_else(|_| "9150".to_string());
    info!("🔧 SOCKS Router configuration:");
    info!("   Listen port: {}", socks_router_port);
    info!("   Arti SOCKS port: {}", arti_socks_port);
    info!("   eltord SOCKS port: {}", eltord_socks_port);

    // Clean up any processes using our ports
    info!("🧹 Starting port cleanup...");
    if let Err(e) = eltor_backend::ports::cleanup_ports_startup().await {
        info!("⚠️  Port cleanup failed: {}", e);
        info!("   Continuing with startup...");
    }

    // Read environment variables
    let backend_port = env::var("BACKEND_PORT")
        .or_else(|_| env::var("PORT")) // Also check for standard PORT env var
        .unwrap_or_else(|_| "5174".to_string())
        .parse::<u16>()
        .unwrap_or(5174);

    // Kill any process using the backend port before starting
    info!("🔧 Cleaning up backend port {}...", backend_port);
    if let Err(e) = eltor_backend::ports::cleanup_backend_port(backend_port).await {
        info!("⚠️  Backend port cleanup failed: {}", e);
        info!("   Continuing with startup... server may fail to bind if port is still in use");
    }

    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1".to_string());

    info!("🔧 Backend configuration:");
    info!("   Bind Address: {}", bind_address);
    info!("   Port: {}", backend_port);
    info!("   Phoenixd embedded: {}", use_phoenixd_embedded);

    // Initialize Lightning node
    info!("⚡ Initializing Lightning node...");

    // Initialize from torrc using PathConfig (already created)
    let torrc_path = path_config.get_torrc_path(None);
    info!("🔍 Looking for torrc file at: {:?}", torrc_path);
    let lightning_node = match eltor_backend::lightning::LightningNode::from_torrc(&torrc_path).await {
        Ok(node) => {
            info!(
                "✅ Lightning node connected from torrc ({})",
                node.node_type()
            );
            Some(node)
        }
        Err(e) => {
            info!("⚠️  Failed to initialize Lightning node from torrc: {}", e);
            // You can choose to exit or handle the error as needed.
            // For now, we'll just continue without a Lightning node.
            None
        }
    };

    if let Some(node) = lightning_node {
        state.set_lightning_node(node);
    }

    // Start SOCKS router in background
    info!("🔀 Starting SOCKS Router...");
    tokio::spawn(async {
        if let Err(e) = eltor_backend::start_socks_router().await {
            info!("⚠️ SOCKS Router failed: {}", e);
        }
    });
    info!("✅ SOCKS Router started in background");

    // Initialize shared EltorManager
    let state_arc = std::sync::Arc::new(tokio::sync::RwLock::new(state.clone()));
    let eltor_manager = eltor_backend::eltor::EltorManager::new(
        state_arc,
        path_config.clone(),
    );
    state.set_eltor_manager(eltor_manager);

    // Start phoenixd if embedded mode is enabled
    if use_phoenixd_embedded {
        info!("🚀 Starting embedded phoenixd...");
        match eltor_backend::wallet::start_phoenixd(state.clone()).await {
            Ok(()) => info!("✅ Phoenixd started successfully"),
            Err(e) => {
                info!("❌ Failed to start phoenixd: {}", e);
                info!("   Continuing without embedded phoenixd...");
            }
        }
    } else {
        info!("🔗 Using external phoenixd instance");
    }

    // Initialize IP location database
    info!("🗺️  Initializing IP location database...");
    let ip_db_path = path_config.get_executable_path("IP2LOCATION-LITE-DB3.BIN");
    info!("   Looking for IP database at: {:?}", ip_db_path);
    
    // Debug: List files in the bin directory
    if let Some(bin_dir) = ip_db_path.parent() {
        info!("   📁 Contents of bin directory {:?}:", bin_dir);
        if bin_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(bin_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let metadata = entry.metadata();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    info!("      - {:?} ({} bytes)", file_name, size);
                }
            } else {
                info!("      ⚠️  Could not read directory");
            }
        } else {
            info!("      ⚠️  Directory does not exist!");
        }
    }
    
    info!("   IP database exists: {}", ip_db_path.exists());
    
    if ip_db_path.exists() {
        match ip::init_ip_database(ip_db_path.clone()) {
            Ok(()) => info!("   ✅ IP database initialized successfully"),
            Err(e) => {
                info!("   ⚠️  Failed to initialize IP database: {}", e);
                info!("   IP geolocation features will be unavailable");
            }
        }
    } else {
        info!("   ⚠️  IP database not found at: {}", ip_db_path.display());
        info!("   IP geolocation features will be unavailable");
        info!("   To enable: download IP2LOCATION-LITE-DB3.BIN and place in bin directory");
    }

    // Configure CORS to allow SSE
    let cors = CorsLayer::permissive();

    // Clone state for graceful shutdown handler
    let state_for_shutdown = state.clone();

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

    info!("🚀 El Tor Backend Server");
    info!("📡 Running on {}", display_address);
    info!("🌐 Frontend served at {}", local_url);
    info!("🔗 Health check: {}/health", local_url);
    info!("� SOCKS Router: {}:{}", bind_address, socks_router_port);
    info!("�📋 API endpoints:");
    info!("   POST /api/eltord/activate/:mode");
    info!("   POST /api/eltord/deactivate/:mode");
    info!("   GET  /api/eltord/status");
    info!("   GET  /api/eltord/logs");
    info!("   GET  /api/wallet/info");
    info!("   GET  /api/wallet/balance");
    info!("   POST /api/wallet/invoice");
    // info!("   POST /api/wallet/pay");
    info!("   POST /api/wallet/offer");
    info!("   GET  /api/wallet/status");
    info!("   GET  /api/wallet/transactions");
    info!("   PUT  /api/wallet/config");
    info!("   DELETE /api/wallet/config");
    info!("   GET  /api/wallet/configs");
    info!("   POST /api/phoenix/start");
    info!("   POST /api/phoenix/stop");
    info!("   GET  /api/debug");
    info!("📁 Static files served from frontend/dist/");
    info!("🔧 Environment variables injected into frontend:");
    info!("   BACKEND_PORT: {}", backend_port);
    info!("   BACKEND_URL: {}",
        env::var("BACKEND_URL").unwrap_or_else(|_| local_url.clone())
    );
    info!("   BIND_ADDRESS: {}", bind_address);

    // Setup graceful shutdown signal handler
    let shutdown_signal = async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        
        info!("🛑 Received shutdown signal (CTRL+C), cleaning up...");
        
        // Stop SOCKS router
        if let Err(e) = eltor_backend::stop_socks_router().await {
            info!("⚠️ Failed to stop SOCKS router: {}", e);
        }
        
        // Stop Arti process
        eltor_backend::cleanup_arti().await;
        
        // Cleanup all eltord processes
        eltor_backend::cleanup_all_eltord_processes().await;
        
        // Stop phoenixd if it was started
        if use_phoenixd_embedded {
            if let Err(e) = eltor_backend::stop_phoenixd(state_for_shutdown).await {
                info!("⚠️  Failed to stop phoenixd: {}", e);
            }
        }
        
        info!("✅ Cleanup complete, shutting down server");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .unwrap();
}
