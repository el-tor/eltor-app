use serde::Serialize;
use std::env;
use crate::paths::PathConfig;
use crate::torrc_parser::{get_torrc_config, parse_port_from_config, get_torrc_txt};
use crate::socks::SocksRouterConfig;

#[derive(Debug, Clone, Serialize)]
pub struct DebugInfo {
    pub torrc_path: String,
    pub torrc_relay_path: String,
    pub bin_dir: String,
    pub data_dir: String,
    pub app_data_dir: Option<String>,
    pub backend_port: u16,
    pub frontend_port: u16,
    pub torrc_socks_port: Option<u16>,
    pub torrc_relay_socks_port: Option<u16>,
    pub environment: String,
    pub current_dir: String,
    pub executable_path: Option<String>,
    pub platform: String,
    pub architecture: String,
    pub torrc_file: String,
    pub torrc_relay_file: String,
    pub socks_router_port: Option<String>,
    pub arti_socks_port: Option<String>,
    pub local_ip: Option<String>,
    pub payment_rate_msats: Option<u64>,
}

impl DebugInfo {
    /// Parse SocksPort from a torrc file using the generic torrc parser
    async fn parse_socks_port_from_file(torrc_path: &std::path::Path) -> Option<u16> {
        let socks_ports = get_torrc_config(torrc_path, "SocksPort").await;
        socks_ports.first().and_then(|port| parse_port_from_config(port))
    }

    /// Create debug info using PathConfig
    pub async fn new(path_config: &PathConfig) -> Result<Self, String> {
        // Ensure torrc files exist
        path_config.ensure_torrc_files()?;
        
        let torrc_path = path_config.get_torrc_path(None);
        let torrc_relay_path = path_config.get_torrc_relay_path();
        
        // Get ports from environment or use defaults
        let backend_port = env::var("BACKEND_PORT")
            .unwrap_or_else(|_| "3001".to_string())
            .parse::<u16>()
            .unwrap_or(3001);
            
        let frontend_port = env::var("FRONTEND_PORT")
            .or_else(|_| env::var("PORT"))
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);
        
        // Determine environment
        let environment = if env::var("TAURI_ENV").is_ok() {
            "tauri".to_string()
        } else if env::var("NODE_ENV").unwrap_or_default() == "production" {
            "web-production".to_string()
        } else if env::var("NODE_ENV").unwrap_or_default() == "development" {
            "web-development".to_string()
        } else {
            "unknown".to_string()
        };
        
        // Get current directory
        let current_dir = env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        
        // Get executable path
        let executable_path = env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .ok();
        
        // Platform and architecture
        let platform = env::consts::OS.to_string();
        let architecture = env::consts::ARCH.to_string();
        
        // Parse SocksPort from torrc files
        let torrc_socks_port = Self::parse_socks_port_from_file(&torrc_path).await;
        let torrc_relay_socks_port = Self::parse_socks_port_from_file(&torrc_relay_path).await;

        let torrc_file = get_torrc_txt(&torrc_path).await
            .unwrap_or_else(|_| "Failed to read torrc file".to_string());
        
        let torrc_relay_file = get_torrc_txt(&torrc_relay_path).await
            .unwrap_or_else(|_| "Failed to read torrc relay file".to_string());
        
        // Get SOCKS router configuration (same way the actual router reads it)
        let socks_config = SocksRouterConfig::from_env();
        let socks_router_port = if let Some(ip) = socks_config.listen_addr {
            Some(format!("{}:{}", ip, socks_config.listen_port))
        } else {
            Some(format!("127.0.0.1:{}", socks_config.listen_port))
        };
        let arti_socks_port = Some(socks_config.arti_socks_port.to_string());
        
        // Get local IP address
        let local_ip = match local_ip_address::local_ip() {
            Ok(ip) => Some(ip.to_string()),
            Err(_) => None,
        };
        
        // Get payment rate from torrc.relay
        let payment_rate_msats = get_torrc_config(&torrc_relay_path, "PaymentRateMsats")
            .await
            .first()
            .and_then(|rate| rate.parse::<u64>().ok());
        
        Ok(DebugInfo {
            torrc_path: torrc_path.to_string_lossy().to_string(),
            torrc_relay_path: torrc_relay_path.to_string_lossy().to_string(),
            bin_dir: path_config.bin_dir.to_string_lossy().to_string(),
            data_dir: path_config.data_dir.to_string_lossy().to_string(),
            app_data_dir: path_config.app_data_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
            backend_port,
            frontend_port,
            torrc_socks_port,
            torrc_relay_socks_port,
            environment,
            current_dir,
            executable_path,
            platform,
            architecture,
            torrc_file,
            torrc_relay_file,
            socks_router_port,
            arti_socks_port,
            local_ip,
            payment_rate_msats,
        })
    }
    
    /// Create debug info with custom path config (for Tauri with resource dir)
    pub async fn with_path_config(path_config: PathConfig) -> Result<Self, String> {
        Self::new(&path_config).await
    }
    
    /// Create debug info using default path detection
    pub async fn create_default() -> Result<Self, String> {
        let path_config = PathConfig::new()?;
        Self::new(&path_config).await
    }
    
    /// Format debug info as a human-readable string
    pub fn format_for_display(&self) -> String {
        format!(
            "Debug Information:\n\
            Environment: {}\n\
            Platform: {} ({})\n\
            Current Directory: {}\n\
            Executable: {}\n\
            Local IP: {}\n\
            \n\
            Paths:\n\
            - Torrc File: {}\n\
            - Torrc Relay File: {}\n\
            - Binary Directory: {}\n\
            - Data Directory: {}\n\
            - App Data Directory: {}\n\
            \n\
            Ports:\n\
            - Backend Port: {}\n\
            - Frontend Port: {}\n\
            - Torrc SocksPort: {}\n\
            - Torrc Relay SocksPort: {}\n\
            - SOCKS Router Port: {}\n\
            - Arti SOCKS Port: {}\n\
            - Torrc File Content: {}\n\
            - Torrc Relay File Content: {}",

            self.environment,
            self.platform,
            self.architecture,
            self.current_dir,
            self.executable_path.as_deref().unwrap_or("unknown"),
            self.local_ip.as_deref().unwrap_or("unavailable"),
            self.torrc_path,
            self.torrc_relay_path,
            self.bin_dir,
            self.data_dir,
            self.app_data_dir.as_deref().unwrap_or("none"),
            self.backend_port,
            self.frontend_port,
            self.torrc_socks_port.map_or("not found".to_string(), |p| p.to_string()),
            self.torrc_relay_socks_port.map_or("not found".to_string(), |p| p.to_string()),
            self.socks_router_port.as_ref().map_or("not set".to_string(), |p| p.to_string()),
            self.arti_socks_port.as_ref().map_or("not set".to_string(), |p| p.to_string()),
            self.torrc_file.clone(),
            self.torrc_relay_file.clone()
        )
    }
    
    /// Get just the torrc path (for backwards compatibility)
    pub fn get_torrc_path(&self) -> &str {
        &self.torrc_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    
    #[tokio::test]
    async fn test_debug_info_creation() {
        match DebugInfo::create_default().await {
            Ok(debug_info) => {
                info!("{}", debug_info.format_for_display());
                assert!(!debug_info.torrc_path.is_empty());
                assert!(!debug_info.bin_dir.is_empty());
            }
            Err(e) => {
                info!("Debug info creation failed (expected in some environments): {}", e);
            }
        }
    }
}
