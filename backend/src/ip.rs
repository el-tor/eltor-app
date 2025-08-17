use std::time::Duration;

/// Get the public IP address of the current machine (async version)
/// Falls back to 127.0.0.1 if unable to determine public IP
pub async fn get_public_ip() -> String {
    // Create a reqwest client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    
    // Try multiple methods to get public IP
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip", 
        "https://icanhazip.com",
    ];
    
    for service in &services {
        match client.get(*service).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(ip_text) => {
                            let ip = ip_text.trim().to_string();
                            if is_valid_ip(&ip) {
                                return ip;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    // Fallback: return localhost
    "127.0.0.1".to_string()
}

/// Get the public IP address of the current machine (blocking version)
/// Falls back to 127.0.0.1 if unable to determine public IP
pub fn get_public_ip_blocking() -> String {
    // Create a blocking reqwest client with timeout
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
    
    // Try multiple methods to get public IP
    let services = [
        "https://api.ipify.org",
        "https://ifconfig.me/ip", 
        "https://icanhazip.com",
    ];
    
    for service in &services {
        match client.get(*service).send() {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text() {
                        Ok(ip_text) => {
                            let ip = ip_text.trim().to_string();
                            if is_valid_ip(&ip) {
                                return ip;
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    // Fallback: return localhost
    "127.0.0.1".to_string()
}

/// Basic IP address validation
fn is_valid_ip(ip: &str) -> bool {
    use std::net::IpAddr;
    ip.parse::<IpAddr>().is_ok() && !ip.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_ip() {
        assert!(is_valid_ip("192.168.1.1"));
        assert!(is_valid_ip("8.8.8.8"));
        assert!(is_valid_ip("::1"));
        assert!(!is_valid_ip(""));
        assert!(!is_valid_ip("not.an.ip"));
        assert!(!is_valid_ip("999.999.999.999"));
    }

    #[tokio::test]
    async fn test_get_public_ip_returns_something() {
        let ip = get_public_ip().await;
        assert!(!ip.is_empty());
        assert!(is_valid_ip(&ip));
    }

    #[test]
    fn test_get_public_ip_blocking_returns_something() {
        let ip = get_public_ip_blocking();
        assert!(!ip.is_empty());
        assert!(is_valid_ip(&ip));
    }
}
