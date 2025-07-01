use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::Response,
};
use std::{env, path::Path};
use tokio::fs;

// Serve static files from the frontend dist directory
pub async fn serve_static(uri: Uri) -> Result<Response<Body>, StatusCode> {
    let path = uri.path().trim_start_matches('/');
    
    // If requesting root or index.html, serve with environment injection
    if path.is_empty() || path == "index.html" {
        return serve_index_with_env().await;
    }
    
    // For all other files, serve them directly from the dist directory
    // Use absolute path resolution based on current working directory
    let current_dir = std::env::current_dir().unwrap();
    let file_path = if current_dir.ends_with("backend") {
        // Running from backend directory
        format!("../frontend/dist/{}", path)
    } else {
        // Running from root directory
        format!("frontend/dist/{}", path)
    };
    
    match fs::read(&file_path).await {
        Ok(content) => {
            let content_type = get_content_type(&file_path);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(content))
                .unwrap())
        }
        Err(_) => {
            // If file not found, serve index.html for SPA routing
            serve_index_with_env().await
        }
    }
}

// Serve index.html with injected environment variables
async fn serve_index_with_env() -> Result<Response<Body>, StatusCode> {
    // Use absolute path resolution based on current working directory
    let current_dir = std::env::current_dir().unwrap();
    let index_path = if current_dir.ends_with("backend") {
        // Running from backend directory
        "../frontend/dist/index.html"
    } else {
        // Running from root directory
        "frontend/dist/index.html"
    };
    
    match fs::read_to_string(index_path).await {
        Ok(mut html_content) => {
            // Create environment config from OS environment variables
            let backend_port = env::var("BACKEND_PORT")
                .or_else(|_| env::var("PORT"))
                .unwrap_or_else(|_| "5174".to_string());
            
            // Construct BACKEND_URL if not provided
            let backend_url = match env::var("BACKEND_URL") {
                Ok(url) if !url.is_empty() => url,
                _ => {
                    // Default to appropriate protocol and host with the current port
                    if backend_port == "80" {
                        "http://localhost".to_string()
                    } else if backend_port == "443" {
                        "https://localhost".to_string()
                    } else {
                        format!("http://localhost:{}", backend_port)
                    }
                }
            };
            
            // Construct API_BASE_URL intelligently
            let api_base_url = if backend_url.contains("://") {
                // If BACKEND_URL is a complete URL, check if it already has a port
                if backend_url.contains(&format!(":{}", backend_port)) {
                    // URL already has the correct port, use as-is
                    backend_url.clone()
                } else if (backend_url.starts_with("https://") && backend_port == "443") || 
                         (backend_url.starts_with("http://") && backend_port == "80") {
                    // Standard port for protocol, don't append port
                    backend_url.clone()
                } else {
                    // Non-standard port, append it
                    format!("{}:{}", backend_url, backend_port)
                }
            } else {
                // Not a complete URL, construct it
                format!("{}:{}", backend_url, backend_port)
            };
            
            let env_config = serde_json::json!({
                "BACKEND_PORT": backend_port,
                "BACKEND_URL": backend_url,
                "API_BASE_URL": api_base_url
            });
            
            // Inject the config into the HTML BEFORE any other scripts
            let env_script = format!(
                r#"<script>window.__ENV_CONFIG__ = {};</script>"#,
                env_config
            );
            
            // Insert before any script tags, not just before </head>
            if let Some(pos) = html_content.find("<script") {
                html_content.insert_str(pos, &env_script);
            } else {
                // Fallback: insert before </head> if no script tags found
                html_content = html_content.replace("</head>", &format!("{}</head>", env_script));
            }
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(Body::from(html_content))
                .unwrap())
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

// Get appropriate content type based on file extension
fn get_content_type(file_path: &str) -> &'static str {
    let path = Path::new(file_path);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("eot") => "application/vnd.ms-fontobject",
        _ => "application/octet-stream",
    }
}
