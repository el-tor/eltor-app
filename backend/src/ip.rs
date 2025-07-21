use std::process::Command;

/// Get the public IP address of the current machine
/// Falls back to 127.0.0.1 if unable to determine public IP
pub fn get_public_ip() -> String {
    // Try multiple methods to get public IP
    
    // Method 1: Try using curl with ipify.org
    if let Ok(output) = Command::new("curl")
        .args(["-s", "--max-time", "5", "https://api.ipify.org"])
        .output()
    {
        if output.status.success() {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if is_valid_ip(&ip) {
                return ip;
            }
        }
    }
    
    // Method 2: Try using curl with ifconfig.me
    if let Ok(output) = Command::new("curl")
        .args(["-s", "--max-time", "5", "https://ifconfig.me/ip"])
        .output()
    {
        if output.status.success() {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if is_valid_ip(&ip) {
                return ip;
            }
        }
    }
    
    // Method 3: Try using curl with icanhazip.com
    if let Ok(output) = Command::new("curl")
        .args(["-s", "--max-time", "5", "https://icanhazip.com"])
        .output()
    {
        if output.status.success() {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if is_valid_ip(&ip) {
                return ip;
            }
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

    #[test]
    fn test_get_public_ip_returns_something() {
        let ip = get_public_ip();
        assert!(!ip.is_empty());
        assert!(is_valid_ip(&ip));
    }
}
