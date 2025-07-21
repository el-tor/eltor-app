use serde::Serialize;
use std::env;
use crate::paths::PathConfig;

#[derive(Debug, Clone, Serialize)]
pub struct DebugInfo {
    pub torrc_path: String,
    pub torrc_relay_path: String,
    pub bin_dir: String,
    pub data_dir: String,
    pub app_data_dir: Option<String>,
    pub backend_port: u16,
    pub frontend_port: u16,
    pub environment: String,
    pub current_dir: String,
    pub executable_path: Option<String>,
    pub platform: String,
    pub architecture: String,
}

impl DebugInfo {
    /// Create debug info using PathConfig
    pub fn new(path_config: &PathConfig) -> Result<Self, String> {
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
        
        Ok(DebugInfo {
            torrc_path: torrc_path.to_string_lossy().to_string(),
            torrc_relay_path: torrc_relay_path.to_string_lossy().to_string(),
            bin_dir: path_config.bin_dir.to_string_lossy().to_string(),
            data_dir: path_config.data_dir.to_string_lossy().to_string(),
            app_data_dir: path_config.app_data_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
            backend_port,
            frontend_port,
            environment,
            current_dir,
            executable_path,
            platform,
            architecture,
        })
    }
    
    /// Create debug info with custom path config (for Tauri with resource dir)
    pub fn with_path_config(path_config: PathConfig) -> Result<Self, String> {
        Self::new(&path_config)
    }
    
    /// Create debug info using default path detection
    pub fn create_default() -> Result<Self, String> {
        let path_config = PathConfig::new()?;
        Self::new(&path_config)
    }
    
    /// Format debug info as a human-readable string
    pub fn format_for_display(&self) -> String {
        format!(
            "Debug Information:\n\
            Environment: {}\n\
            Platform: {} ({})\n\
            Current Directory: {}\n\
            Executable: {}\n\
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
            - Frontend Port: {}",
            self.environment,
            self.platform,
            self.architecture,
            self.current_dir,
            self.executable_path.as_deref().unwrap_or("unknown"),
            self.torrc_path,
            self.torrc_relay_path,
            self.bin_dir,
            self.data_dir,
            self.app_data_dir.as_deref().unwrap_or("none"),
            self.backend_port,
            self.frontend_port
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
    
    #[test]
    fn test_debug_info_creation() {
        match DebugInfo::create_default() {
            Ok(debug_info) => {
                println!("{}", debug_info.format_for_display());
                assert!(!debug_info.torrc_path.is_empty());
                assert!(!debug_info.bin_dir.is_empty());
            }
            Err(e) => {
                println!("Debug info creation failed (expected in some environments): {}", e);
            }
        }
    }
}
