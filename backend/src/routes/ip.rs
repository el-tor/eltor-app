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
    };    // Extract coordinates from the record or use defaults
    let (lat, lng) = if let (Some(latitude), Some(longitude)) = (location_record.latitude, location_record.longitude) {
        println!("✅ Using exact coordinates from IP database: ({}, {})", latitude, longitude);
        (latitude as f64, longitude as f64)
    } else {
        // Fallback to city/country coordinates
        let city_name = location_record.city.as_ref()
            .map(|c| c.as_ref())
            .unwrap_or("Unknown");

        println!("🔍 No coordinates in IP record, falling back to city lookup for: '{}'", city_name);
        println!("🔍 Raw city from database: {:?}", location_record.city);
        
        if !city_name.is_empty() && city_name != "-" && city_name != "Unknown" {
            // Try city coordinates first
            if let Some(coords) = city_to_coordinates(city_name) {
                println!("📍 City '{}' mapped to known coordinates: ({}, {})", city_name, coords.0, coords.1);
                coords
            } else {
                // Unknown city, try region fallback
                let region_name = location_record.region.as_ref()
                    .map(|r| r.as_ref())
                    .unwrap_or("Unknown");
                
                println!("🌍 Unknown city '{}', trying region '{}' for coordinates", city_name, region_name);
                
                // For US regions, try to map them to state center coordinates
                if let Some(coords) = us_region_to_coordinates(region_name) {
                    println!("📍 US region '{}' mapped to coordinates: ({}, {})", region_name, coords.0, coords.1);
                    coords
                } else {
                    // Final fallback: country coordinates, then hash if needed
                    let country_code = "US"; // Default for now
                    let coords = country_center_coordinates(country_code);
                    println!("🏳️ Falling back to country '{}' coordinates: ({}, {})", country_code, coords.0, coords.1);
                    coords
                }
            }
        } else {
            // Try to get region coordinates directly
            let region_name = location_record.region.as_ref()
                .map(|r| r.as_ref())
                .unwrap_or("Unknown");
            
            println!("🌍 No valid city, trying region '{}' for coordinates", region_name);
            
            // For US regions, try to map them to state center coordinates
            if let Some(coords) = us_region_to_coordinates(region_name) {
                println!("📍 US region '{}' mapped to coordinates: ({}, {})", region_name, coords.0, coords.1);
                coords
            } else {
                // Final fallback to country coordinates
                let country_code = "US"; // Default for now
                let coords = country_center_coordinates(country_code);
                println!("🏳️ Falling back to country '{}' coordinates: ({}, {})", country_code, coords.0, coords.1);
                coords
            }
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

    println!("📊 Final extracted data - City: '{}', Region: '{}', Coordinates: ({}, {})", 
             city, region, lat, lng);

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
            ipv6.is_loopback() || is_unicast_link_local_ipv6(ipv6)
        }
    }
}

fn is_unicast_link_local_ipv6(addr: &std::net::Ipv6Addr) -> bool {
    // Link-local unicast addresses are in the range fe80::/10
    let octets = addr.octets();
    octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80
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
fn city_to_coordinates(city: &str) -> Option<(f64, f64)> {
    match city.to_lowercase().as_str() {
        "new york" | "new york city" | "nyc" => Some((40.7128, -74.0060)),
        "los angeles" | "la" => Some((34.0522, -118.2437)),
        "chicago" => Some((41.8781, -87.6298)),
        "houston" => Some((29.7604, -95.3698)),
        "phoenix" => Some((33.4484, -112.0740)),
        "philadelphia" | "philly" => Some((39.9526, -75.1652)),
        "san antonio" => Some((29.4241, -98.4936)),
        "san diego" => Some((32.7157, -117.1611)),
        "dallas" => Some((32.7767, -96.7970)),
        "austin" | "austin city" | "austin-round rock" | "austin-round rock-san marcos" | "austin-round rock-georgetown" | "austin metro" => Some((30.2672, -97.7431)),
        "san jose" => Some((37.3382, -121.8863)),
        "fort worth" => Some((32.7555, -97.3308)),
        "jacksonville" => Some((30.3322, -81.6557)),
        "columbus" => Some((39.9612, -82.9988)),
        "charlotte" => Some((35.2271, -80.8431)),
        "san francisco" | "sf" => Some((37.7749, -122.4194)),
        "indianapolis" => Some((39.7684, -86.1581)),
        "seattle" => Some((47.6062, -122.3321)),
        "denver" => Some((39.7392, -104.9903)),
        "washington" | "washington dc" | "dc" => Some((38.9072, -77.0369)),
        "boston" => Some((42.3601, -71.0589)),
        "el paso" => Some((31.7619, -106.4850)),
        "detroit" => Some((42.3314, -83.0458)),
        "nashville" => Some((36.1627, -86.7816)),
        "memphis" => Some((35.1495, -90.0490)),
        "portland" => Some((45.5152, -122.6784)), // Oregon Portland
        "oklahoma city" => Some((35.4676, -97.5164)),
        "las vegas" => Some((36.1699, -115.1398)),
        "baltimore" => Some((39.2904, -76.6122)),
        "milwaukee" => Some((43.0389, -87.9065)),
        "albuquerque" => Some((35.0844, -106.6504)),
        "tucson" => Some((32.2226, -110.9747)),
        "fresno" => Some((36.7378, -119.7871)),
        "sacramento" => Some((38.5816, -121.4944)),
        "mesa" => Some((33.4152, -111.8315)),
        "kansas city" => Some((39.0997, -94.5786)),
        "atlanta" => Some((33.7490, -84.3880)),
        "long beach" => Some((33.7701, -118.1937)),
        "colorado springs" => Some((38.8339, -104.8214)),
        "raleigh" => Some((35.7796, -78.6382)),
        "miami" => Some((25.7617, -80.1918)),
        "virginia beach" => Some((36.8529, -75.9780)),
        "omaha" => Some((41.2565, -95.9345)),
        "oakland" => Some((37.8044, -122.2712)),
        "minneapolis" => Some((44.9778, -93.2650)),
        "tulsa" => Some((36.1540, -95.9928)),
        "cleveland" => Some((41.4993, -81.6944)),
        "wichita" => Some((37.6872, -97.3301)),
        "arlington" => Some((32.7357, -97.1081)), // Texas Arlington
        "new orleans" => Some((29.9511, -90.0715)),
        "bakersfield" => Some((35.3733, -119.0187)),
        "tampa" => Some((27.9506, -82.4572)),
        "honolulu" => Some((21.3099, -157.8581)),
        "aurora" => Some((39.7294, -104.8319)), // Colorado Aurora
        "anaheim" => Some((33.8366, -117.9143)),
        "santa ana" => Some((33.7455, -117.8677)),
        "corpus christi" => Some((27.8006, -97.3964)),
        "riverside" => Some((33.9533, -117.3962)),
        "lexington" => Some((38.0406, -84.5037)),
        "stockton" => Some((37.9577, -121.2908)),
        "st. paul" => Some((44.9537, -93.0900)),
        "cincinnati" => Some((39.1031, -84.5120)),
        "anchorage" => Some((61.2181, -149.9003)),
        "henderson" => Some((36.0397, -114.9817)),
        "greensboro" => Some((36.0726, -79.7920)),
        "plano" => Some((33.0198, -96.6989)),
        "newark" => Some((40.7357, -74.1724)),
        "lincoln" => Some((40.8136, -96.7026)),
        "toledo" => Some((41.6528, -83.5379)),
        "orlando" => Some((28.5383, -81.3792)),
        "chula vista" => Some((32.6401, -117.0842)),
        "jersey city" => Some((40.7178, -74.0431)),
        "chandler" => Some((33.3062, -111.8413)),
        "laredo" => Some((27.5306, -99.4803)),
        "madison" => Some((43.0731, -89.4012)),
        "lubbock" => Some((33.5779, -101.8552)),
        "winston-salem" => Some((36.0999, -80.2442)),
        "garland" => Some((32.9126, -96.6389)),
        "glendale" => Some((33.5387, -112.1860)), // Arizona Glendale
        "hialeah" => Some((25.8576, -80.2781)),
        "reno" => Some((39.5296, -119.8138)),
        "baton rouge" => Some((30.4515, -91.1871)),
        "irvine" => Some((33.6846, -117.8265)),
        "chesapeake" => Some((36.7682, -76.2875)),
        "irving" => Some((32.8140, -96.9489)),
        "scottsdale" => Some((33.4942, -111.9261)),
        "north las vegas" => Some((36.1989, -115.1175)),
        "fremont" => Some((37.5485, -121.9886)),
        "gilbert" => Some((33.3528, -111.7890)),
        "san bernardino" => Some((34.1083, -117.2898)),
        "boise" => Some((43.6150, -116.2023)),
        "birmingham" => Some((33.5186, -86.8104)),
        // International cities
        "london" => Some((51.5074, -0.1278)),
        "paris" => Some((48.8566, 2.3522)),
        "berlin" => Some((52.5200, 13.4050)),
        "madrid" => Some((40.4168, -3.7038)),
        "rome" => Some((41.9028, 12.4964)),
        "amsterdam" => Some((52.3676, 4.9041)),
        "tokyo" => Some((35.6762, 139.6503)),
        "osaka" => Some((34.6937, 135.5023)),
        "sydney" => Some((-33.8688, 151.2093)),
        "melbourne" => Some((-37.8136, 144.9631)),
        "toronto" => Some((43.6532, -79.3832)),
        "vancouver" => Some((49.2827, -123.1207)),
        "montreal" => Some((45.5017, -73.5673)),
        "moscow" => Some((55.7558, 37.6176)),
        "beijing" => Some((39.9042, 116.4074)),
        "shanghai" => Some((31.2304, 121.4737)),
        "mumbai" => Some((19.0760, 72.8777)),
        "delhi" => Some((28.7041, 77.1025)),
        "seoul" => Some((37.5665, 126.9780)),
        "singapore" => Some((1.3521, 103.8198)),
        _ => {
            // Log unknown cities so we can add them to the database
            println!("❓ Unknown city '{}' (lowercase: '{}') - no coordinates found", city, city.to_lowercase());
            None
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

/// Map US state/region names to center coordinates
fn us_region_to_coordinates(region: &str) -> Option<(f64, f64)> {
    match region.to_lowercase().as_str() {
        "texas" | "tx" => Some((31.9686, -99.9018)),
        "california" | "ca" => Some((36.7783, -119.4179)),
        "florida" | "fl" => Some((27.6648, -81.5158)),
        "new york" | "ny" => Some((42.1657, -74.9481)),
        "pennsylvania" | "pa" => Some((41.2033, -77.1945)),
        "illinois" | "il" => Some((40.3363, -89.0022)),
        "ohio" | "oh" => Some((40.3888, -82.7649)),
        "georgia" | "ga" => Some((33.0406, -83.6431)),
        "north carolina" | "nc" => Some((35.5397, -79.8431)),
        "michigan" | "mi" => Some((43.3266, -84.5361)),
        "virginia" | "va" => Some((37.7693, -78.2057)),
        "washington" | "wa" => Some((47.7511, -120.7401)),
        "arizona" | "az" => Some((34.0489, -111.0937)),
        "massachusetts" | "ma" => Some((42.2081, -71.0275)),
        "tennessee" | "tn" => Some((35.7478, -86.7923)),
        "indiana" | "in" => Some((39.8494, -86.2583)),
        "missouri" | "mo" => Some((38.4561, -92.2884)),
        "maryland" | "md" => Some((39.0639, -76.8021)),
        "wisconsin" | "wi" => Some((44.2619, -89.6165)),
        "colorado" | "co" => Some((39.0598, -105.3111)),
        "minnesota" | "mn" => Some((45.7326, -93.9196)),
        "south carolina" | "sc" => Some((33.8191, -80.9066)),
        "alabama" | "al" => Some((32.3617, -86.7904)),
        "louisiana" | "la" => Some((31.1801, -91.8749)),
        "kentucky" | "ky" => Some((37.6681, -84.6701)),
        "oregon" | "or" => Some((44.5672, -122.1269)),
        "oklahoma" | "ok" => Some((35.5653, -96.9289)),
        "connecticut" | "ct" => Some((41.5978, -72.7554)),
        "utah" | "ut" => Some((40.1135, -111.8535)),
        "iowa" | "ia" => Some((42.0115, -93.2105)),
        "nevada" | "nv" => Some((38.3135, -117.0554)),
        "arkansas" | "ar" => Some((34.9513, -92.3809)),
        "mississippi" | "ms" => Some((32.7673, -89.6812)),
        "kansas" | "ks" => Some((38.5266, -96.7265)),
        "new mexico" | "nm" => Some((34.8405, -106.2485)),
        "nebraska" | "ne" => Some((41.1254, -98.2681)),
        "west virginia" | "wv" => Some((38.4912, -80.9540)),
        "idaho" | "id" => Some((44.2394, -114.5103)),
        "hawaii" | "hi" => Some((21.0943, -157.4983)),
        "new hampshire" | "nh" => Some((43.4525, -71.5639)),
        "maine" | "me" => Some((44.6939, -69.3819)),
        "montana" | "mt" => Some((47.0527, -110.2148)),
        "rhode island" | "ri" => Some((41.6809, -71.5118)),
        "delaware" | "de" => Some((39.3185, -75.5071)),
        "south dakota" | "sd" => Some((44.2998, -99.4388)),
        "north dakota" | "nd" => Some((47.5289, -99.7840)),
        "alaska" | "ak" => Some((61.2181, -149.9003)),
        "vermont" | "vt" => Some((44.0459, -72.7107)),
        "wyoming" | "wy" => Some((42.7560, -107.3025)),
        _ => None,
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