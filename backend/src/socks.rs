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
    pub listen_addr: Option<IpAddr>,
    pub arti_socks_port: u16,
    pub eltord_client_socks_port: u16,
    pub eltord_relay_socks_port: u16,
}

impl Default for SocksRouterConfig {
    fn default() -> Self {
        Self {
            listen_port: 18048,
            listen_addr: None, // Defaults to 127.0.0.1 if None
            arti_socks_port: 18050,
            eltord_client_socks_port: 18058, // Client mode SOCKS port
            eltord_relay_socks_port: 18057, // Relay mode SOCKS port
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
        // Helper function to parse address and port from "IP:PORT" or just "PORT" format
        fn parse_addr_port(env_var: &str) -> (Option<IpAddr>, Option<u16>) {
            if let Ok(s) = std::env::var(env_var) {
                if let Some((ip_str, port_str)) = s.split_once(':') {
                    // Parse both IP and port
                    let ip = ip_str.parse::<IpAddr>().ok();
                    let port = port_str.parse::<u16>().ok();
                    (ip, port)
                } else {
                    // Just a port number
                    (None, s.parse::<u16>().ok())
                }
            } else {
                (None, None)
            }
        }
        
        let (router_ip, router_port) = parse_addr_port("APP_ELTOR_SOCKS_ROUTER_PORT");
        let (_, arti_port) = parse_addr_port("APP_ARTI_SOCKS_PORT");
        let (_, eltord_client_port) = parse_addr_port("APP_ELTOR_TOR_SOCKS_PORT");
        let (_, eltord_relay_port) = parse_addr_port("APP_ELTOR_TOR_RELAY_SOCKS_PORT");
        
        Self {
            listen_port: router_port.unwrap_or(18048),
            listen_addr: router_ip,
            arti_socks_port: arti_port.unwrap_or(18050),
            eltord_client_socks_port: eltord_client_port.unwrap_or(18058),
            eltord_relay_socks_port: eltord_relay_port.unwrap_or(18057),
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
        // Use provided IP address or default to 127.0.0.1 (localhost)
        let ip = self.config.listen_addr.unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let bind_addr = SocketAddr::new(ip, self.config.listen_port);
        
        match TcpListener::bind(bind_addr).await {
            Ok(listener) => {
                info!("🔀 SOCKS Router started on {}", bind_addr);
                info!("   .onion domains -> Arti SOCKS (port {})", self.config.arti_socks_port);
                info!("   Other domains -> eltord client SOCKS (port {}) or relay SOCKS (port {})", 
                    self.config.eltord_client_socks_port, self.config.eltord_relay_socks_port);
                
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
    
    // TCP reads are not message boundaries: read the complete greeting.
    let mut greeting = [0u8; 2];
    client_stream.read_exact(&mut greeting).await?;
    if greeting[0] != 0x05 || greeting[1] == 0 {
        return Err("Invalid SOCKS5 greeting".into());
    }
    let mut methods = vec![0; greeting[1] as usize];
    client_stream.read_exact(&mut methods).await?;
    if !methods.contains(&0x00) {
        client_stream.write_all(&[0x05, 0xff]).await?;
        return Err("No supported SOCKS5 authentication method".into());
    }
    client_stream.write_all(&[0x05, 0x00]).await?;

    let buffer = read_socks_message(&mut client_stream).await?;
    let n = buffer.len();

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
        // Check which eltord port is available (client or relay)
        // Try to connect to client port first
        let eltord_port = if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", config.eltord_client_socks_port)).await.is_ok() {
            debug!("🌐 Using eltord client port {} for {}", config.eltord_client_socks_port, client_addr);
            config.eltord_client_socks_port
        } else {
            debug!("🌐 Using eltord relay port {} for {}", config.eltord_relay_socks_port, client_addr);
            config.eltord_relay_socks_port
        };
        
        handle_via_proxy(client_stream, &buffer[..n], eltord_port).await
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

    // Read exactly one complete reply, preserving any following tunnel data.
    let buffer = read_socks_message(&mut upstream_stream).await?;
    let resp_n = buffer.len();
    client_stream.write_all(&buffer).await?;

    // Check if the proxy connection was successful
    if resp_n >= 2 && buffer[1] != 0x00 {
        warn!("⚠️ Proxy connection failed with status: {}", buffer[1]);
        return Err(format!("Proxy connection failed with status: {}", buffer[1]).into());
    }

    debug!("✅ SOCKS tunnel established via proxy port {}", proxy_port);

    // Propagate half-closes and surface transfer errors without detached tasks.
    tokio::io::copy_bidirectional(&mut client_stream, &mut upstream_stream).await?;
    debug!("🔌 SOCKS connection closed");
    Ok(())
}

/// Read a SOCKS5 request/reply without assuming a single TCP read contains it.
async fn read_socks_message(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 5 || header[2] != 0 {
        return Err("Invalid SOCKS5 header".into());
    }
    let mut message = header.to_vec();
    let address_len = match header[3] {
        1 => 4,
        4 => 16,
        3 => {
            let len = stream.read_u8().await?;
            if len == 0 { return Err("Empty SOCKS5 hostname".into()); }
            message.push(len);
            len as usize
        }
        _ => return Err("Unsupported SOCKS5 address type".into()),
    };
    let start = message.len();
    message.resize(start + address_len + 2, 0);
    stream.read_exact(&mut message[start..]).await?;
    Ok(message)
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).await.unwrap();
        let (server, _) = listener.accept().await.unwrap();
        (client, server)
    }

    #[tokio::test]
    async fn reads_fragmented_ipv4_and_preserves_following_data() {
        let (mut client, mut server) = pair().await;
        let writer = tokio::spawn(async move {
            for byte in [5, 1, 0, 1, 127, 0, 0, 1, 0, 80] {
                client.write_all(&[byte]).await.unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(2)).await;
            }
            client.write_all(b"GET").await.unwrap();
        });
        assert_eq!(read_socks_message(&mut server).await.unwrap(), [5,1,0,1,127,0,0,1,0,80]);
        let mut data = [0; 3];
        server.read_exact(&mut data).await.unwrap();
        assert_eq!(&data, b"GET");
        writer.await.unwrap();
    }

    #[tokio::test]
    async fn accepts_short_domain_and_ipv6_frames() {
        for frame in [vec![5,1,0,3,1,b'x',0,80],
                      [vec![5,1,0,4], vec![0;16], vec![0,80]].concat()] {
            let (mut client, mut server) = pair().await;
            client.write_all(&frame).await.unwrap();
            assert_eq!(read_socks_message(&mut server).await.unwrap(), frame);
        }
    }

    #[tokio::test]
    async fn rejects_truncated_frames() {
        let (mut client, mut server) = pair().await;
        client.write_all(&[5,1,0,3,10,b'x']).await.unwrap();
        client.shutdown().await.unwrap();
        assert!(read_socks_message(&mut server).await.is_err());
    }

    #[tokio::test]
    async fn fragmented_greeting_consumes_methods_before_request() {
        let (mut client, server) = pair().await;
        let task = tokio::spawn(handle_socks_connection(server,
            "127.0.0.1:1".parse().unwrap(), SocksRouterConfig::default()));
        client.write_all(&[5]).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        client.write_all(&[1]).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        client.write_all(&[0]).await.unwrap();
        let mut reply = [0;2];
        client.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [5,0]);
        drop(client);
        assert!(task.await.unwrap().is_err());
    }

    #[tokio::test]
    async fn rejects_unoffered_no_auth_method() {
        let (mut client, server) = pair().await;
        let task = tokio::spawn(handle_socks_connection(server,
            "127.0.0.1:1".parse().unwrap(), SocksRouterConfig::default()));
        client.write_all(&[5,1,2]).await.unwrap();
        let mut reply = [0;2];
        client.read_exact(&mut reply).await.unwrap();
        assert_eq!(reply, [5,255]);
        assert!(task.await.unwrap().is_err());
    }
}
