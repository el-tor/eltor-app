use std::fs;
use std::path::Path;

/// Configuration structure for Lightning node from torrc
#[derive(Debug, Clone)]
pub struct LightningConfig {
    pub node_type: String,
    pub url: String,
    pub password: String, // Can be password, rune, or macaroon depending on node_type
}

/// Parse lightning configuration from torrc file
pub fn parse_lightning_config_from_torrc<P: AsRef<Path>>(
    torrc_path: P,
) -> Result<Option<LightningConfig>, String> {
    let content = fs::read_to_string(&torrc_path)
        .map_err(|e| format!("Failed to read torrc file: {}", e))?;

    for line in content.lines() {
        let trimmed = line.trim();
        
        // Look for PaymentLightningNodeConfig lines
        if trimmed.starts_with("#PaymentLightningNodeConfig") || 
           trimmed.starts_with("PaymentLightningNodeConfig") {
            
            // Remove the prefix to get the config part
            let config_part = trimmed
                .trim_start_matches("#PaymentLightningNodeConfig")
                .trim_start_matches("PaymentLightningNodeConfig")
                .trim();
            
            return parse_lightning_config_string(config_part).map(Some);
        }
    }
    
    Ok(None)
}

/// Parse lightning configuration from a config string
/// Format: "type=phoenixd url=http://127.0.0.1:9740 password=abc123 default=true"
pub fn parse_lightning_config_string(config_str: &str) -> Result<LightningConfig, String> {
    let node_type = get_config_value(config_str, "type")
        .ok_or("Node type not found in config")?;
    
    let url = get_config_value(config_str, "url")
        .ok_or("URL not found in config")?;
    
    // Try to find authentication method in order: password, rune, macaroon
    let password = get_config_value(config_str, "password")
        .or_else(|| get_config_value(config_str, "rune"))
        .or_else(|| get_config_value(config_str, "macaroon"))
        .ok_or("No authentication method (password/rune/macaroon) found in config")?;
    
    Ok(LightningConfig {
        node_type,
        url,
        password,
    })
}

/// Extract a specific key-value pair from config string
/// Example: get_config_value("type=phoenixd url=http://localhost", "type") -> Some("phoenixd")
pub fn get_config_value(config_str: &str, key: &str) -> Option<String> {
    config_str
        .split_whitespace()
        .find(|pair| pair.starts_with(&format!("{}=", key)))
        .and_then(|pair| {
            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() == 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
}

/// Get default value from config string (used by the pattern you showed)
pub fn get_default_value(config_str: String, key: String) -> Option<String> {
    get_config_value(&config_str, &key)
}

/// Parse torrc file and extract all relevant configuration
pub fn parse_torrc<P: AsRef<Path>>(torrc_path: P) -> Result<TorrcConfig, String> {
    let content = fs::read_to_string(&torrc_path)
        .map_err(|e| format!("Failed to read torrc file: {}", e))?;

    let mut config = TorrcConfig::default();

    for line in content.lines() {
        let trimmed = line.trim();
        
        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse key-value pairs
        if let Some((key, value)) = parse_torrc_line(trimmed) {
            match key.as_str() {
                "SocksPort" => config.socks_port = Some(value),
                "ControlPort" => config.control_port = Some(value),
                "Address" => config.address = Some(value),
                "ORPort" => config.or_port = Some(value),
                "Nickname" => config.nickname = Some(value),
                "ContactInfo" => config.contact_info = Some(value),
                "DataDirectory" => config.data_directory = Some(value),
                "HashedControlPassword" => config.hashed_control_password = Some(value),
                _ => {
                    // Store other configurations
                    config.other_configs.insert(key, value);
                }
            }
        }
    }

    Ok(config)
}

/// Parse a single torrc line into key-value pair
fn parse_torrc_line(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, ' ').collect();
    if parts.len() == 2 {
        Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
    } else {
        None
    }
}

/// Torrc configuration structure
#[derive(Debug, Clone, Default)]
pub struct TorrcConfig {
    pub socks_port: Option<String>,
    pub control_port: Option<String>,
    pub address: Option<String>,
    pub or_port: Option<String>,
    pub nickname: Option<String>,
    pub contact_info: Option<String>,
    pub data_directory: Option<String>,
    pub hashed_control_password: Option<String>,
    pub other_configs: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_value() {
        let config = "type=phoenixd url=http://127.0.0.1:9740 password=abc123 default=true";
        
        assert_eq!(get_config_value(config, "type"), Some("phoenixd".to_string()));
        assert_eq!(get_config_value(config, "url"), Some("http://127.0.0.1:9740".to_string()));
        assert_eq!(get_config_value(config, "password"), Some("abc123".to_string()));
        assert_eq!(get_config_value(config, "default"), Some("true".to_string()));
        assert_eq!(get_config_value(config, "nonexistent"), None);
    }

    #[test]
    fn test_parse_lightning_config_string() {
        let config = "type=phoenixd url=http://127.0.0.1:9740 password=abc123 default=true";
        let result = parse_lightning_config_string(config).unwrap();
        
        assert_eq!(result.node_type, "phoenixd");
        assert_eq!(result.url, "http://127.0.0.1:9740");
        assert_eq!(result.password, "abc123");
    }

    #[test]
    fn test_parse_cln_config() {
        let config = "type=cln url=https://cln.example.com rune=abc123rune default=true";
        let result = parse_lightning_config_string(config).unwrap();
        
        assert_eq!(result.node_type, "cln");
        assert_eq!(result.url, "https://cln.example.com");
        assert_eq!(result.password, "abc123rune"); // Should pick up rune as password
    }

    #[test]
    fn test_parse_lnd_config() {
        let config = "type=lnd url=https://lnd.example.com macaroon=abc123macaroon default=true";
        let result = parse_lightning_config_string(config).unwrap();
        
        assert_eq!(result.node_type, "lnd");
        assert_eq!(result.url, "https://lnd.example.com");
        assert_eq!(result.password, "abc123macaroon"); // Should pick up macaroon as password
    }
}
