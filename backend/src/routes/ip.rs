use axum::{extract::Path, http::StatusCode, response::Json};
use ip2location::DB;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpLocationResponse {
    pub ip: String,
    pub latitude: f64,
    pub longitude: f64,
    pub city: Option<String>,
    pub region: Option<String>,
    pub country: String,
    pub country_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpLocationError {
    pub error: String,
}

// Global database instance - wrapped in Arc<Mutex<>> for thread safety
static IP_DB: OnceLock<Option<Arc<Mutex<DB>>>> = OnceLock::new();

/// Initialize the IP2Location database
pub fn init_ip_database(db_path: PathBuf) -> Result<(), String> {
    let db = DB::from_file(&db_path)
        .map_err(|e| format!("Failed to load IP2Location database: {}", e))?;
    
    IP_DB.set(Some(Arc::new(Mutex::new(db))))
        .map_err(|_| "Database already initialized".to_string())?;
    
    println!("✅ IP2Location database loaded from: {}", db_path.display());
    Ok(())
}

/// Core IP lookup function that can be used by both web and Tauri
pub fn lookup_ip_location(ip: &str) -> Result<IpLocationResponse, String> {
    let ip_addr: IpAddr = ip.parse()
        .map_err(|e| format!("Invalid IP address '{}': {}", ip, e))?;

    // Handle local/private IPs
    if is_private_ip(&ip_addr) {
        return Ok(get_local_ip_location(ip));
    }

    // Get database instance
    let db_arc = IP_DB.get()
        .ok_or("IP database not initialized")?
        .as_ref()
        .ok_or("IP database not loaded")?;

    let db = db_arc.lock()
        .map_err(|_| "Failed to acquire database lock".to_string())?;
    
    let record = db.ip_lookup(ip_addr)
        .map_err(|e| format!("IP lookup failed for '{}': {}", ip, e))?;

    // Pattern match on Record enum to get LocationRecord
    let location_record = match record {
        ip2location::Record::LocationDb(location_record) => location_record,
        ip2location::Record::ProxyDb(_) => {
            return Err("Received proxy record instead of location record".to_string());
        }
    };

    // Extract coordinates from the record or use defaults
    let (lat, lng) = if let (Some(latitude), Some(longitude)) = (location_record.latitude, location_record.longitude) {
        (latitude as f64, longitude as f64)
    } else {
        // Fallback to city/country coordinates
        let city_name = location_record.city.as_ref()
            .map(|c| c.as_ref())
            .unwrap_or("Unknown");
        
        if !city_name.is_empty() && city_name != "-" && city_name != "Unknown" {
            city_to_coordinates(city_name)
        } else {
            // Try to get country code for fallback coordinates
            let country_code = location_record.country.as_ref()
                .map(|_| "US") // For now, default to US. We'll need to figure out how to extract country code
                .unwrap_or("US");
            country_center_coordinates(country_code)
        }
    };

    // Extract city, handling Option<Cow<str>>
    let city = location_record.city.as_ref()
        .map(|c| c.as_ref())
        .filter(|&c| !c.is_empty() && c != "-")
        .unwrap_or("Unknown")
        .to_string();

    // Extract region, handling Option<Cow<str>>
    let region = location_record.region.as_ref()
        .map(|r| r.as_ref())
        .filter(|&r| !r.is_empty() && r != "-")
        .unwrap_or("Unknown")
        .to_string();

    // For now, we'll return placeholder values for country info until we understand the Country type
    let country = "Unknown".to_string();
    let country_code = "XX".to_string();

    Ok(IpLocationResponse {
        ip: ip.to_string(),
        latitude: lat,
        longitude: lng,
        city: Some(city),
        region: Some(region),
        country,
        country_code,
    })
}

/// Check if IP is private/local
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local()
        }
        IpAddr::V6(ipv6) => {
            ipv6.is_loopback() || ipv6.is_unicast_link_local()
        }
    }
}

/// Generate location for local/private IPs
fn get_local_ip_location(ip: &str) -> IpLocationResponse {
    // Use a hash of the IP to generate consistent coordinates
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    ip.hash(&mut hasher);
    let hash = hasher.finish();
    
    // Generate coordinates in a reasonable range
    let lat = ((hash % 180) as f64) - 90.0; // -90 to 90
    let lng = (((hash >> 32) % 360) as f64) - 180.0; // -180 to 180
    
    IpLocationResponse {
        ip: ip.to_string(),
        latitude: lat,
        longitude: lng,
        city: Some("Local Network".to_string()),
        region: Some("Private".to_string()),
        country: "Local".to_string(),
        country_code: "LO".to_string(),
    }
}

/// Convert city name to approximate coordinates
fn city_to_coordinates(city: &str) -> (f64, f64) {
    match city.to_lowercase().as_str() {
        "new york" | "new york city" => (40.7128, -74.0060),
        "los angeles" => (34.0522, -118.2437),
        "chicago" => (41.8781, -87.6298),
        "houston" => (29.7604, -95.3698),
        "phoenix" => (33.4484, -112.0740),
        "philadelphia" => (39.9526, -75.1652),
        "san antonio" => (29.4241, -98.4936),
        "san diego" => (32.7157, -117.1611),
        "dallas" => (32.7767, -96.7970),
        "san jose" => (37.3382, -121.8863),
        "london" => (51.5074, -0.1278),
        "paris" => (48.8566, 2.3522),
        "berlin" => (52.5200, 13.4050),
        "madrid" => (40.4168, -3.7038),
        "rome" => (41.9028, 12.4964),
        "amsterdam" => (52.3676, 4.9041),
        "tokyo" => (35.6762, 139.6503),
        "osaka" => (34.6937, 135.5023),
        "sydney" => (-33.8688, 151.2093),
        "melbourne" => (-37.8136, 144.9631),
        "toronto" => (43.6532, -79.3832),
        "vancouver" => (49.2827, -123.1207),
        "moscow" => (55.7558, 37.6176),
        "beijing" => (39.9042, 116.4074),
        "shanghai" => (31.2304, 121.4737),
        "mumbai" => (19.0760, 72.8777),
        "delhi" => (28.7041, 77.1025),
        "seoul" => (37.5665, 126.9780),
        "singapore" => (1.3521, 103.8198),
        _ => {
            // Generate consistent coordinates for unknown cities
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            city.hash(&mut hasher);
            let hash = hasher.finish();
            let lat = ((hash % 160) as f64) - 80.0; // -80 to 80 (avoid poles)
            let lng = (((hash >> 32) % 360) as f64) - 180.0; // -180 to 180
            (lat, lng)
        }
    }
}

/// Get country center coordinates
fn country_center_coordinates(country_code: &str) -> (f64, f64) {
    match country_code {
        "US" => (39.8283, -98.5795),
        "CA" => (56.1304, -106.3468),
        "GB" => (55.3781, -3.4360),
        "DE" => (51.1657, 10.4515),
        "FR" => (46.2276, 2.2137),
        "IT" => (41.8719, 12.5674),
        "ES" => (40.4637, -3.7492),
        "NL" => (52.1326, 5.2913),
        "JP" => (36.2048, 138.2529),
        "AU" => (-25.2744, 133.7751),
        "CN" => (35.8617, 104.1954),
        "RU" => (61.5240, 105.3188),
        "IN" => (20.5937, 78.9629),
        "KR" => (35.9078, 127.7669),
        "SG" => (1.3521, 103.8198),
        "BR" => (-14.2350, -51.9253),
        "MX" => (23.6345, -102.5528),
        _ => (0.0, 0.0) // Default to equator
    }
}

// === WEB API ROUTES ===

/// GET /api/ip/{ip} - Lookup IP location
pub async fn get_ip_location(Path(ip): Path<String>) -> Result<Json<IpLocationResponse>, (StatusCode, Json<IpLocationError>)> {
    match lookup_ip_location(&ip) {
        Ok(location) => Ok(Json(location)),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(IpLocationError { error }),
        )),
    }
}

/// POST /api/ip/bulk - Lookup multiple IPs
#[derive(Deserialize)]
pub struct BulkIpRequest {
    pub ips: Vec<String>,
}

#[derive(Serialize)]
pub struct BulkIpResponse {
    pub results: Vec<Result<IpLocationResponse, String>>,
}

pub async fn get_bulk_ip_locations(Json(request): Json<BulkIpRequest>) -> Json<BulkIpResponse> {
    let results = request.ips
        .iter()
        .map(|ip| lookup_ip_location(ip))
        .collect();

    Json(BulkIpResponse { results })
}