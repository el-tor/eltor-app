use log::{debug, error, info, warn};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Debug)]
enum SocksCommand {
    Connect = 0x01,
}

#[derive(Clone, Debug)]
enum AuthMethod {
    NoAuth = 0x00,
    NoAcceptable = 0xFF,
}

#[derive(Clone, Debug)]
enum ReplyCode {
    Success = 0x00,
    GeneralFailure = 0x01,
    ConnectionNotAllowed = 0x02,
    NetworkUnreachable = 0x03,
    HostUnreachable = 0x04,
    ConnectionRefused = 0x05,
    TtlExpired = 0x06,
    CommandNotSupported = 0x07,
    AddressTypeNotSupported = 0x08,
}

#[derive(Clone, Debug)]
enum AddressType {
    IPv4 = 0x01,
    DomainName = 0x03,
    IPv6 = 0x04,
}

/// Configuration for the SOCKS router
#[derive(Clone, Debug)]
pub struct SocksRouterConfig {
    pub listen_port: u16,
    pub arti_socks_port: u16,
    pub eltord_socks_port: u16,
}

impl Default for SocksRouterConfig {
    fn default() -> Self {
        Self {
            listen_port: 18049,
            arti_socks_port: 18050,
            eltord_socks_port: 18058, // Updated to match eltord's actual SOCKS port
        }
    }
}

/// Target address from SOCKS5 request
#[derive(Debug, Clone)]
pub enum TargetAddress {
    IPv4(Ipv4Addr, u16),
    Domain(String, u16),
    IPv6(std::net::Ipv6Addr, u16),
}

impl TargetAddress {
    pub fn is_onion(&self) -> bool {
        match self {
            TargetAddress::Domain(domain, _) => domain.ends_with(".onion"),
            _ => false,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            TargetAddress::IPv4(ip, port) => format!("{}:{}", ip, port),
            TargetAddress::Domain(domain, port) => format!("{}:{}", domain, port),
            TargetAddress::IPv6(ip, port) => format!("[{}]:{}", ip, port),
        }
    }
}

impl std::fmt::Display for TargetAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Environment variable configuration support
impl SocksRouterConfig {
    pub fn from_env() -> Self {
        Self {
            listen_port: std::env::var("APP_SOCKS_ROUTER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(18049),
            arti_socks_port: std::env::var("APP_ARTI_SOCKS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(18050),
            eltord_socks_port: std::env::var("APP_ELTORD_SOCKS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(18058),
        }
    }
}

/// SOCKS router that routes .onion domains to Arti and other traffic to eltord
pub struct SocksRouter {
    config: SocksRouterConfig,
    listener: Option<TcpListener>,
}

impl SocksRouter {
    pub fn new(config: SocksRouterConfig) -> Self {
        Self {
            config,
            listener: None,
        }
    }
    
    /// Start the SOCKS router server
    pub async fn start(&mut self) -> Result<(), String> {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), self.config.listen_port);
        
        match TcpListener::bind(bind_addr).await {
            Ok(listener) => {
                info!("🔀 SOCKS Router started on {}", bind_addr);
                info!("   .onion domains -> Arti SOCKS (port {})", self.config.arti_socks_port);
                info!("   Other domains -> eltord SOCKS (port {})", self.config.eltord_socks_port);
                
                self.listener = Some(listener);
                Ok(())
            }
            Err(e) => {
                Err(format!("Failed to bind SOCKS router to {}: {}", bind_addr, e))
            }
        }
    }
    
    /// Run the SOCKS router server
    pub async fn run(&mut self) -> Result<(), String> {
        let listener = self.listener.take().ok_or("SOCKS router not started")?;
        
        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    let config = self.config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_socks_connection(client_stream, client_addr, config).await {
                            warn!("⚠️ SOCKS connection error from {}: {}", client_addr, e);
                        }
                    });
                }
                Err(e) => {
                    error!("❌ Failed to accept SOCKS connection: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}

/// Handle a single SOCKS5 connection using the cleaner approach
async fn handle_socks_connection(
    mut client_stream: TcpStream,
    client_addr: SocketAddr,
    config: SocksRouterConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    debug!("🔌 New SOCKS5 connection from {}", client_addr);
    
    // Step 1: SOCKS5 greeting
    let mut buffer = vec![0u8; 1024];
    let n = client_stream.read(&mut buffer).await?;
    
    debug!("📥 Received {} bytes for greeting from {}", n, client_addr);
    
    if n < 2 || buffer[0] != 0x05 {
        warn!("❌ Invalid SOCKS5 greeting from {}: {:?}", client_addr, &buffer[..n.min(10)]);
        return Err("Invalid SOCKS5 greeting".into());
    }

    debug!("✅ Valid SOCKS5 greeting from {}, sending auth method", client_addr);
    // Send method selection (no auth)
    client_stream.write_all(&[0x05, 0x00]).await?;

    // Step 2: Read SOCKS5 request
    let n = client_stream.read(&mut buffer).await?;
    
    debug!("📥 Received {} bytes for request from {}", n, client_addr);
    
    if n < 10 || buffer[0] != 0x05 {
        warn!("❌ Invalid SOCKS5 request from {}: {:?}", client_addr, &buffer[..n.min(20)]);
        return Err("Invalid SOCKS5 request".into());
    }

    if buffer[1] != SocksCommand::Connect as u8 {
        warn!("❌ Unsupported SOCKS command from {}: {}", client_addr, buffer[1]);
        // Send command not supported
        let response = vec![0x05, 0x07, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        client_stream.write_all(&response).await?;
        return Err("Only CONNECT command is supported".into());
    }
    
    // Parse target address
    let target = parse_target_address(&buffer[3..n])?;
    debug!("🎯 SOCKS target from {}: {}", client_addr, target.to_string());
    
    // Step 3: Determine which proxy to use
    if target.is_onion() {
        debug!("🧅 Routing .onion domain to Arti (port {}) for {}", config.arti_socks_port, client_addr);
        handle_via_proxy(client_stream, &buffer[..n], config.arti_socks_port).await
    } else {
        debug!("🌐 Routing regular domain to eltord (port {}) for {}", config.eltord_socks_port, client_addr);
        handle_via_proxy(client_stream, &buffer[..n], config.eltord_socks_port).await
    }
}

/// Forward connection through a SOCKS5 proxy
async fn handle_via_proxy(
    mut client_stream: TcpStream,
    request_data: &[u8],
    proxy_port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let proxy_addr = format!("127.0.0.1:{}", proxy_port);
    debug!("🔌 Connecting to proxy at {}", proxy_addr);
    
    // Connect to upstream SOCKS5 proxy
    let mut upstream_stream = match TcpStream::connect(&proxy_addr).await {
        Ok(stream) => {
            debug!("✅ Connected to proxy at {}", proxy_addr);
            stream
        },
        Err(e) => {
            warn!("⚠️ Failed to connect to proxy {}: {}", proxy_addr, e);
            // Send connection refused to client
            let response = vec![0x05, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            client_stream.write_all(&response).await?;
            return Err(format!("Failed to connect to proxy: {}", e).into());
        }
    };

    // Forward SOCKS5 handshake to upstream
    debug!("🔐 Sending auth handshake to proxy");
    upstream_stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut auth_response = [0u8; 2];
    upstream_stream.read_exact(&mut auth_response).await?;
    debug!("🔐 Proxy auth response: {:?}", auth_response);

    if auth_response[1] != 0x00 {
        warn!("⚠️ Proxy authentication failed: {:?}", auth_response);
        let response = vec![0x05, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        client_stream.write_all(&response).await?;
        return Err("Proxy authentication failed".into());
    }

    // Forward the original SOCKS5 request to upstream
    debug!("📤 Forwarding SOCKS request to proxy: {} bytes", request_data.len());
    upstream_stream.write_all(request_data).await?;

    // Read upstream response
    debug!("📥 Reading proxy response");
    let mut buffer = vec![0u8; 1024];
    let resp_n = upstream_stream.read(&mut buffer).await?;
    debug!("📥 Proxy response: {} bytes, status: {}", resp_n, if resp_n > 1 { buffer[1] } else { 255 });
    
    // Forward response to client
    debug!("📤 Forwarding proxy response to client");
    client_stream.write_all(&buffer[..resp_n]).await?;

    // Check if the proxy connection was successful
    if resp_n >= 2 && buffer[1] != 0x00 {
        warn!("⚠️ Proxy connection failed with status: {}", buffer[1]);
        return Err(format!("Proxy connection failed with status: {}", buffer[1]).into());
    }

    debug!("✅ SOCKS tunnel established via proxy port {}", proxy_port);

    // Bidirectional copy
    let (mut client_read, mut client_write) = client_stream.into_split();
    let (mut upstream_read, mut upstream_write) = upstream_stream.into_split();

    let client_to_upstream = tokio::spawn(async move {
        tokio::io::copy(&mut client_read, &mut upstream_write).await
    });

    let upstream_to_client = tokio::spawn(async move {
        tokio::io::copy(&mut upstream_read, &mut client_write).await
    });

    let _ = tokio::try_join!(client_to_upstream, upstream_to_client);
    debug!("🔌 SOCKS connection closed");
    Ok(())
}

/// Parse target address from SOCKS5 request
fn parse_target_address(data: &[u8]) -> Result<TargetAddress, Box<dyn std::error::Error + Send + Sync>> {
    if data.is_empty() {
        return Err("Empty address data".into());
    }
    
    match data[0] {
        0x01 => { // IPv4
            if data.len() < 7 {
                return Err("Invalid IPv4 address length".into());
            }
            let ip = Ipv4Addr::new(data[1], data[2], data[3], data[4]);
            let port = u16::from_be_bytes([data[5], data[6]]);
            Ok(TargetAddress::IPv4(ip, port))
        }
        0x03 => { // Domain name
            if data.len() < 2 {
                return Err("Invalid domain name length".into());
            }
            let domain_len = data[1] as usize;
            if data.len() < 4 + domain_len {
                return Err("Incomplete domain name data".into());
            }
            let domain = String::from_utf8(data[2..2 + domain_len].to_vec())?;
            let port = u16::from_be_bytes([data[2 + domain_len], data[3 + domain_len]]);
            Ok(TargetAddress::Domain(domain, port))
        }
        0x04 => { // IPv6
            if data.len() < 19 {
                return Err("Invalid IPv6 address length".into());
            }
            let mut ipv6_bytes = [0u8; 16];
            ipv6_bytes.copy_from_slice(&data[1..17]);
            let ip = std::net::Ipv6Addr::from(ipv6_bytes);
            let port = u16::from_be_bytes([data[17], data[18]]);
            Ok(TargetAddress::IPv6(ip, port))
        }
        _ => Err(format!("Unsupported address type: {}", data[0]).into()),
    }
}

/// Create and start a SOCKS router
pub async fn create_socks_router() -> Result<SocksRouter, String> {
    let config = SocksRouterConfig::from_env();
    info!("🔧 Creating SOCKS router with config: {:?}", config);
    
    let mut router = SocksRouter::new(config);
    router.start().await?;
    Ok(router)
}

/// Start the SOCKS router server
pub async fn start_socks_router() -> Result<(), String> {
    let mut router = create_socks_router().await?;
    router.run().await
}

/// Stop the SOCKS router (placeholder for compatibility)
pub async fn stop_socks_router() -> Result<(), String> {
    // Since we're using a simple approach, there's no global state to clean up
    // The router stops when the task is cancelled
    Ok(())
}

/// Check if SOCKS router is running (placeholder for compatibility)
pub fn is_socks_router_running() -> bool {
    // For now, we'll assume it's not running since we don't track global state
    // This could be enhanced with proper state tracking if needed
    false
}