use std::fs;
use std::path::Path;

/// Get all values for a specific torrc configuration key
/// Returns an array of values found for the given key
/// 
/// Example: get_torrc_config(torrc_path, "SocksPort") -> vec!["18057", "127.0.0.1:18058"]
pub fn get_torrc_config<P: AsRef<Path>>(torrc_path: P, config_key: &str) -> Vec<String> {
    let mut values = Vec::new();
    
    if let Ok(content) = fs::read_to_string(torrc_path) {
        for line in content.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            
            // Check if line starts with the config key followed by space
            if let Some(config_line) = line.strip_prefix(&format!("{} ", config_key)) {
                values.push(config_line.trim().to_string());
            }
        }
    }
    
    values
}

/// Parse port number from a config value (handles formats like "18057" or "127.0.0.1:18057")
pub fn parse_port_from_config(config_value: &str) -> Option<u16> {
    if let Some(colon_pos) = config_value.rfind(':') {
        // Format: "127.0.0.1:18057"
        let port_str = &config_value[colon_pos + 1..];
        port_str.parse::<u16>().ok()
    } else {
        // Format: "18057"
        config_value.parse::<u16>().ok()
    }
}

/// Read the entire torrc file as text
/// Returns the raw file content as a string, or an error if the file cannot be read
pub fn get_torrc_txt<P: AsRef<Path>>(torrc_path: P) -> Result<String, String> {
    fs::read_to_string(torrc_path).map_err(|e| format!("Failed to read torrc file: {}", e))
}

/// Configuration structure for Lightning node from torrc
#[derive(Debug, Clone)]
pub struct LightningConfig {
    pub node_type: String,
    pub url: String,
    pub password: String, // Can be password, rune, or macaroon depending on node_type
    pub is_default: bool,
}

/// Operation type for modifying torrc
#[derive(Debug, Clone)]
pub enum Operation {
    Delete,
    Upsert,
}

/// Lightning node type
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Cln,
    Lnd,
    Phoenixd,
}

impl NodeType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "cln" => Ok(NodeType::Cln),
            "lnd" => Ok(NodeType::Lnd),
            "phoenixd" => Ok(NodeType::Phoenixd),
            _ => Err(format!("Unknown node type: {}", s)),
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            NodeType::Cln => "cln".to_string(),
            NodeType::Lnd => "lnd".to_string(),
            NodeType::Phoenixd => "phoenixd".to_string(),
        }
    }

    /// Get the password field name for this node type
    pub fn password_field(&self) -> &'static str {
        match self {
            NodeType::Cln => "rune",
            NodeType::Lnd => "macaroon",
            NodeType::Phoenixd => "password",
        }
    }
}

/// Upsert or delete PaymentLightningNodeConfig in torrc file
pub fn modify_payment_lightning_config<P: AsRef<Path>>(
    torrc_path: P,
    operation: Operation,
    node_type: NodeType,
    url: Option<String>,
    password: Option<String>,
    set_as_default: bool,
) -> Result<(), String> {
    let content =
        fs::read_to_string(&torrc_path).map_err(|e| format!("Failed to read torrc file: {}", e))?;

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut found_target = false;
    let mut found_default = false;
    let mut target_line_index: Option<usize> = None;
    let mut default_line_index: Option<usize> = None;

    // First pass: find existing configurations
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip commented lines
        if trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("PaymentLightningNodeConfig") {
            let config_part = trimmed
                .trim_start_matches("PaymentLightningNodeConfig")
                .trim();

            if let Ok(existing_type) = get_config_value(config_part, "type")
                .ok_or("No type found")
                .and_then(|t| NodeType::from_str(&t).map_err(|_| "Invalid node type"))
            {
                let existing_url = get_config_value(config_part, "url");

                // Check if this matches our target type and URL
                if existing_type == node_type {
                    match operation {
                        Operation::Upsert => {
                            // For upsert, match by node_type only (one config per type)
                            if !found_target {
                                found_target = true;
                                target_line_index = Some(i);
                            }
                        }
                        Operation::Delete => {
                            // For delete, use the provided url if specified
                            if let Some(target_url) = url.as_ref() {
                                if existing_url.as_ref() == Some(target_url) {
                                    found_target = true;
                                    target_line_index = Some(i);
                                }
                            } else {
                                // For delete operations without URL, take the first match
                                if !found_target {
                                    found_target = true;
                                    target_line_index = Some(i);
                                }
                            }
                        }
                    }
                }

                // Check if this line has default=true
                if get_config_value(config_part, "default") == Some("true".to_string()) {
                    found_default = true;
                    default_line_index = Some(i);
                }
            }
        }
    }

    match operation {
        Operation::Delete => {
            if let Some(index) = target_line_index {
                lines.remove(index);
            }
        }
        Operation::Upsert => {
            let url = url.ok_or("URL is required for upsert operation")?;
            let password = password.ok_or("Password is required for upsert operation")?;

            // Build the new config line
            let password_field = node_type.password_field();
            let default_str = if set_as_default { " default=true" } else { "" };
            let new_config = format!(
                "PaymentLightningNodeConfig type={} url={} {}={}{}",
                node_type.to_string(),
                url,
                password_field,
                password,
                default_str
            );

            dbg!(&new_config);

            // If we're setting this as default, remove default=true from other lines
            if set_as_default && found_default {
                if let Some(default_index) = default_line_index {
                    if Some(default_index) != target_line_index {
                        // Remove default=true from the existing default line
                        let line = &lines[default_index];
                        let updated_line = line
                            .replace(" default=true", "")
                            .replace("default=true ", "")
                            .replace("default=true", "");
                        lines[default_index] = updated_line;
                    }
                }
            }

            // Update existing line or add new one
            if let Some(index) = target_line_index {
                lines[index] = new_config;
            } else {
                lines.push(new_config);
            }
        }
    }

    // Write the updated content back to file
    let updated_content = lines.join("\n");
    fs::write(&torrc_path, updated_content)
        .map_err(|e| format!("Failed to write torrc file: {}", e))?;

    Ok(())
}

/// Parse lightning configuration from torrc file
pub fn parse_lightning_config_from_torrc<P: AsRef<Path>>(
    torrc_path: P,
) -> Result<Option<LightningConfig>, String> {
    let content =
        fs::read_to_string(&torrc_path).map_err(|e| format!("Failed to read torrc file: {}", e))?;

    for line in content.lines() {
        let trimmed = line.trim();

        // Look for PaymentLightningNodeConfig lines
        if trimmed.starts_with("#PaymentLightningNodeConfig")
            || trimmed.starts_with("PaymentLightningNodeConfig")
        {
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
    let node_type = get_config_value(config_str, "type").ok_or("Node type not found in config")?;

    let url = get_config_value(config_str, "url").ok_or("URL not found in config")?;

    // Try to find authentication method in order: password, rune, macaroon
    let password = get_config_value(config_str, "password")
        .or_else(|| get_config_value(config_str, "rune"))
        .or_else(|| get_config_value(config_str, "macaroon"))
        .ok_or("No authentication method (password/rune/macaroon) found in config")?;

    // Check if this config is marked as default
    let is_default = get_config_value(config_str, "default")
        .map(|val| val.to_lowercase() == "true")
        .unwrap_or(false);

    Ok(LightningConfig {
        node_type,
        url,
        password,
        is_default,
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
    let content =
        fs::read_to_string(&torrc_path).map_err(|e| format!("Failed to read torrc file: {}", e))?;

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

/// Get all PaymentLightningNodeConfig entries from torrc
pub fn get_all_payment_lightning_configs<P: AsRef<Path>>(
    torrc_path: P,
) -> Result<Vec<LightningConfig>, String> {
    let content =
        fs::read_to_string(&torrc_path).map_err(|e| format!("Failed to read torrc file: {}", e))?;

    let mut configs = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip commented lines and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("PaymentLightningNodeConfig") {
            let config_part = trimmed
                .trim_start_matches("PaymentLightningNodeConfig")
                .trim();

            if let Ok(config) = parse_lightning_config_string(config_part) {
                configs.push(config);
            }
        }
    }

    Ok(configs)
}

/// Update or add a single-value configuration line in a torrc file
/// If the config_key exists, it will be updated. If not, it will be added.
/// 
/// Example: update_torrc_config_line(torrc_relay_path, "PaymentBolt12Offer", "lno1...")
pub fn update_torrc_config_line<P: AsRef<Path>>(
    torrc_path: P,
    config_key: &str,
    new_value: &str,
) -> Result<(), String> {
    let torrc_path = torrc_path.as_ref();
    
    // Read existing content
    let content = fs::read_to_string(torrc_path)
        .map_err(|e| format!("Failed to read torrc file: {}", e))?;
    
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut found = false;
    let config_line = format!("{} {}", config_key, new_value);
    
    // Try to find and update existing line
    for line in &mut lines {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }
        
        // Check if this line starts with our config key
        if trimmed.starts_with(&format!("{} ", config_key)) || trimmed == config_key {
            *line = config_line.clone();
            found = true;
            break;
        }
    }
    
    // If not found, add it to the end
    if !found {
        lines.push(config_line);
    }
    
    // Write back to file
    let updated_content = lines.join("\n");
    fs::write(torrc_path, updated_content)
        .map_err(|e| format!("Failed to write torrc file: {}", e))?;
    
    Ok(())
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
    use std::fs;
    use std::path::PathBuf;

    fn create_test_torrc(content: &str) -> PathBuf {
        let mut temp_file = std::env::temp_dir();
        temp_file.push(format!("test_torrc_{}", std::process::id()));
        fs::write(&temp_file, content).unwrap();
        temp_file
    }

    #[test]
    fn test_get_torrc_config() {
        let content = r#"
# Test torrc file
SocksPort 18057
SocksPort 127.0.0.1:18058
ControlPort 9992
Log notice stdout
# Comment about logs
Log info file /tmp/test.log
"#;
        
        let test_file = create_test_torrc(content);
        
        // Test SocksPort - should return multiple values
        let socks_ports = get_torrc_config(&test_file, "SocksPort");
        assert_eq!(socks_ports, vec!["18057", "127.0.0.1:18058"]);
        
        // Test ControlPort - should return single value
        let control_ports = get_torrc_config(&test_file, "ControlPort");
        assert_eq!(control_ports, vec!["9992"]);
        
        // Test Log - should return multiple values
        let log_configs = get_torrc_config(&test_file, "Log");
        assert_eq!(log_configs, vec!["notice stdout", "info file /tmp/test.log"]);
        
        // Test non-existent config
        let missing = get_torrc_config(&test_file, "NonExistent");
        assert_eq!(missing, Vec::<String>::new());
        
        // Clean up
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_torrc_txt() {
        let content = r#"# Test torrc file
SocksPort 18057
SocksPort 127.0.0.1:18058
ControlPort 9992
Log notice stdout
# Comment about logs
Log info file /tmp/test.log
"#;
        
        let test_file = create_test_torrc(content);
        
        // Test reading entire file content
        let file_content = get_torrc_txt(&test_file).unwrap();
        assert_eq!(file_content, content);
        
        // Clean up
        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_get_torrc_txt_nonexistent_file() {
        let nonexistent_path = "/tmp/nonexistent_torrc_file_12345";
        
        // Test error handling for nonexistent file
        let result = get_torrc_txt(nonexistent_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read torrc file"));
    }
    
    #[test]
    fn test_parse_port_from_config() {
        // Test simple port
        assert_eq!(parse_port_from_config("18057"), Some(18057));
        
        // Test IP:port format
        assert_eq!(parse_port_from_config("127.0.0.1:18058"), Some(18058));
        
        // Test invalid formats
        assert_eq!(parse_port_from_config("invalid"), None);
        assert_eq!(parse_port_from_config("127.0.0.1:invalid"), None);
    }

    #[test]
    fn test_get_config_value() {
        let config = "type=phoenixd url=http://127.0.0.1:9740 password=abc123 default=true";

        assert_eq!(
            get_config_value(config, "type"),
            Some("phoenixd".to_string())
        );
        assert_eq!(
            get_config_value(config, "url"),
            Some("http://127.0.0.1:9740".to_string())
        );
        assert_eq!(
            get_config_value(config, "password"),
            Some("abc123".to_string())
        );
        assert_eq!(
            get_config_value(config, "default"),
            Some("true".to_string())
        );
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

    #[test]
    fn test_modify_payment_lightning_config_upsert_new() {
        let content = r#"SocksPort 18058
ControlPort 9992
DataDirectory /var/lib/tor
"#;
        let torrc_path = create_test_torrc(content);

        // Upsert new phoenixd config
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Phoenixd,
            Some("http://127.0.0.1:9740".to_string()),
            Some("secret123".to_string()),
            true,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://127.0.0.1:9740 password=secret123 default=true"));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_upsert_existing() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://old.url password=oldpass default=true
PaymentLightningNodeConfig type=phoenixd url=http://old2.url password=old2pass
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Update existing phoenixd config by URL
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Phoenixd,
            Some("http://old2.url".to_string()),
            Some("newpass".to_string()),
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://old2.url password=newpass"
        ));
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://old.url password=oldpass default=true"));
        assert!(!updated_content.contains("old2pass"));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_upsert_multiple_same_type() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url password=pass1 default=true
PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=pass2
PaymentLightningNodeConfig type=phoenixd url=http://phoenix3.url password=pass3
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Update middle phoenixd config by URL
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Phoenixd,
            Some("http://phoenix2.url".to_string()),
            Some("updated_pass2".to_string()),
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // Should update only the matching URL
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url password=pass1 default=true"));
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=updated_pass2"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://phoenix3.url password=pass3"
        ));
        // Check that the original line with "password=pass2" (not "updated_pass2") is gone
        assert!(!updated_content.contains("password=pass2 "));
        assert!(!updated_content.contains("password=pass2\n"));
        assert!(!updated_content.contains(" password=pass2"));

        // More precise check: the original line should be completely gone
        assert!(!updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=pass2"
        ));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_delete_by_url() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url password=pass1 default=true
PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=pass2
PaymentLightningNodeConfig type=phoenixd url=http://phoenix3.url password=pass3
PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Delete specific phoenixd config by URL
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Delete,
            NodeType::Phoenixd,
            Some("http://phoenix2.url".to_string()),
            None,
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // Should delete only the matching URL
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url password=pass1 default=true"));
        assert!(!updated_content
            .contains("PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://phoenix3.url password=pass3"
        ));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune"
        ));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_delete_without_url() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url password=pass1 default=true
PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=pass2
PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Delete phoenixd config without specifying URL (should delete first match)
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Delete,
            NodeType::Phoenixd,
            None,
            None,
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // Should delete the first phoenixd config found
        assert!(!updated_content
            .contains("PaymentLightningNodeConfig type=phoenixd url=http://phoenix1.url"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://phoenix2.url password=pass2"
        ));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune"
        ));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_upsert_new_when_url_not_found() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://existing.url password=existing_pass default=true
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Try to upsert with a new URL (should add new config)
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Phoenixd,
            Some("http://new.url".to_string()),
            Some("new_pass".to_string()),
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // Should have both configs
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://existing.url password=existing_pass default=true"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://new.url password=new_pass"
        ));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_delete() {
        let content = r#"SocksPort 18058
PaymentLightningNodeConfig type=phoenixd url=http://127.0.0.1:9740 password=secret123 default=true
PaymentLightningNodeConfig type=phoenixd url=http://127.0.0.2:9740 password=secret2
PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune
ControlPort 9992
"#;
        let torrc_path = create_test_torrc(content);

        // Delete specific phoenixd config by URL
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Delete,
            NodeType::Phoenixd,
            Some("http://127.0.0.1:9740".to_string()),
            None,
            false,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();
        assert!(!updated_content
            .contains("PaymentLightningNodeConfig type=phoenixd url=http://127.0.0.1:9740"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=phoenixd url=http://127.0.0.2:9740 password=secret2"
        ));
        assert!(updated_content.contains("type=cln")); // CLN config should remain

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_modify_payment_lightning_config_change_default() {
        let content = r#"PaymentLightningNodeConfig type=phoenixd url=http://phoenix.url password=phoenix_pass default=true
PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune
"#;
        let torrc_path = create_test_torrc(content);

        // Set CLN as default (should remove default from phoenixd)
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Cln,
            Some("https://new-cln.example.com".to_string()),
            Some("new_cln_rune".to_string()),
            true,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // CLN should be default now
        assert!(updated_content.contains("PaymentLightningNodeConfig type=cln url=https://new-cln.example.com rune=new_cln_rune default=true"));

        // Phoenixd should not have default=true anymore
        let phoenixd_line = updated_content
            .lines()
            .find(|line| line.contains("type=phoenixd"))
            .unwrap();
        assert!(!phoenixd_line.contains("default=true"));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_ignore_commented_lines() {
        let content = r#"SocksPort 18058
#PaymentLightningNodeConfig type=phoenixd url=http://commented.url password=commented_pass default=true
# PaymentLightningNodeConfig type=cln url=https://also.commented rune=also_commented
PaymentLightningNodeConfig type=lnd url=https://active.lnd macaroon=active_macaroon
"#;
        let torrc_path = create_test_torrc(content);

        // Upsert phoenixd config (should add new, not modify commented one)
        modify_payment_lightning_config(
            &torrc_path,
            Operation::Upsert,
            NodeType::Phoenixd,
            Some("http://new.phoenix".to_string()),
            Some("new_phoenix_pass".to_string()),
            true,
        )
        .unwrap();

        let updated_content = fs::read_to_string(&torrc_path).unwrap();

        // Should have both commented lines unchanged and new active line
        assert!(updated_content
            .contains("#PaymentLightningNodeConfig type=phoenixd url=http://commented.url"));
        assert!(updated_content.contains("# PaymentLightningNodeConfig type=cln"));
        assert!(updated_content.contains("PaymentLightningNodeConfig type=phoenixd url=http://new.phoenix password=new_phoenix_pass default=true"));
        assert!(updated_content.contains(
            "PaymentLightningNodeConfig type=lnd url=https://active.lnd macaroon=active_macaroon"
        ));

        fs::remove_file(torrc_path).unwrap();
    }

    #[test]
    fn test_get_all_payment_lightning_configs() {
        let content = r#"SocksPort 18058
#PaymentLightningNodeConfig type=phoenixd url=http://commented.url password=commented_pass
PaymentLightningNodeConfig type=phoenixd url=http://phoenix.url password=phoenix_pass default=true
PaymentLightningNodeConfig type=cln url=https://cln.example.com rune=cln_rune
PaymentLightningNodeConfig type=lnd url=https://lnd.example.com macaroon=lnd_macaroon
"#;
        let torrc_path = create_test_torrc(content);

        let configs = get_all_payment_lightning_configs(&torrc_path).unwrap();

        assert_eq!(configs.len(), 3); // Should ignore commented line

        let phoenixd_config = configs.iter().find(|c| c.node_type == "phoenixd").unwrap();
        assert_eq!(phoenixd_config.url, "http://phoenix.url");
        assert_eq!(phoenixd_config.password, "phoenix_pass");

        let cln_config = configs.iter().find(|c| c.node_type == "cln").unwrap();
        assert_eq!(cln_config.url, "https://cln.example.com");
        assert_eq!(cln_config.password, "cln_rune");

        let lnd_config = configs.iter().find(|c| c.node_type == "lnd").unwrap();
        assert_eq!(lnd_config.url, "https://lnd.example.com");
        assert_eq!(lnd_config.password, "lnd_macaroon");

        fs::remove_file(torrc_path).unwrap();
    }
}
