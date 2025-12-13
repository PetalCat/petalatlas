use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use flate2::read::GzDecoder;
use std::io::Read;
use base64::Engine;
use std::collections::HashMap;
use std::sync::Mutex;

// Simple in-memory cache
lazy_static::lazy_static! {
    static ref PLAYER_PROFILE_CACHE: Mutex<HashMap<String, (serde_json::Value, std::time::SystemTime)>> = Mutex::new(HashMap::new());
    static ref IMAGE_CACHE: Mutex<HashMap<String, (String, std::time::SystemTime)>> = Mutex::new(HashMap::new());
}

const CACHE_DURATION_SECS: u64 = 3600; // 1 hour

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerData {
    pub edition: String,           // "java" or "bedrock"
    pub primary_id: String,        // UUID for Java, XUID for Bedrock
    pub display_name: Option<String>, // Cached name (not authoritative)
    pub position: PlayerPosition,
    pub health: f32,
    pub food_level: i32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub game_mode: i32,
    pub dimension: String,
    pub last_seen: Option<i64>,    // Timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[tauri::command]
pub fn get_players(world_path: String) -> Result<Vec<PlayerData>, String> {
    let playerdata_path = Path::new(&world_path).join("playerdata");
    
    if !playerdata_path.exists() {
        return Ok(Vec::new());
    }

    let mut players = Vec::new();

    // Just list UUIDs quickly - don't parse NBT yet
    if let Ok(entries) = fs::read_dir(&playerdata_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            // Only process .dat files
            if path.extension().and_then(|s| s.to_str()) != Some("dat") {
                continue;
            }

            // Extract UUID from filename
            let uuid = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            // Create minimal player data with just UUID
            // NBT parsing can happen later if needed
            players.push(PlayerData {
                edition: "java".to_string(),
                primary_id: uuid,
                display_name: None,
                position: PlayerPosition { x: 0.0, y: 0.0, z: 0.0 },
                health: 0.0,
                food_level: 0,
                xp_level: 0,
                xp_total: 0,
                game_mode: 0,
                dimension: "unknown".to_string(),
                last_seen: None,
            });
        }
    }

    Ok(players)
}

// Separate command to get detailed player data
#[tauri::command]
pub fn get_player_details(world_path: String, uuid: String) -> Result<PlayerData, String> {
    let player_file = Path::new(&world_path)
        .join("playerdata")
        .join(format!("{}.dat", uuid));
    
    if !player_file.exists() {
        return Err("Player file not found".to_string());
    }

    let mut player_data = parse_player_file(&player_file)?;
    player_data.edition = "java".to_string();
    player_data.primary_id = uuid;
    
    Ok(player_data)
}

// Fetch player profile from Mojang API (bypasses proxy)
#[tauri::command]
pub async fn fetch_player_profile(uuid: String) -> Result<serde_json::Value, String> {
    // Check cache first
    {
        let cache = PLAYER_PROFILE_CACHE.lock().unwrap();
        if let Some((data, timestamp)) = cache.get(&uuid) {
            if let Ok(elapsed) = timestamp.elapsed() {
                if elapsed.as_secs() < CACHE_DURATION_SECS {
                    return Ok(data.clone());
                }
            }
        }
    }

    let uuid_clean = uuid.replace("-", "");
    let url = format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", uuid_clean);
    
    // Create HTTP client with no proxy
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch player profile: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}: Failed to fetch player profile", response.status()));
    }
    
    let data = response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse player profile: {}", e))?;
    
    // Cache the result
    {
        let mut cache = PLAYER_PROFILE_CACHE.lock().unwrap();
        cache.insert(uuid, (data.clone(), std::time::SystemTime::now()));
    }
    
    Ok(data)
}

// Fetch player avatar as base64 (bypasses proxy)
#[tauri::command]
pub async fn fetch_player_avatar(uuid: String, avatar_type: String) -> Result<String, String> {
    let uuid_clean = uuid.replace("-", "");
    
    let url = match avatar_type.as_str() {
        "head" => format!("https://crafatar.com/avatars/{}?size=64&overlay", uuid_clean),
        "body" => format!("https://crafatar.com/renders/body/{}?scale=6&overlay", uuid_clean),
        _ => return Err("Invalid avatar type".to_string()),
    };
    
    // Create HTTP client with no proxy
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch avatar: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}: Failed to fetch avatar", response.status()));
    }
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read avatar bytes: {}", e))?;
    
    // Return as base64 data URL
    let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{}", base64))
}

// Fetch any image URL as base64 (bypasses proxy)
#[tauri::command]
pub async fn fetch_image(url: String) -> Result<String, String> {
    // Check cache first
    {
        let cache = IMAGE_CACHE.lock().unwrap();
        if let Some((data, timestamp)) = cache.get(&url) {
            if let Ok(elapsed) = timestamp.elapsed() {
                if elapsed.as_secs() < CACHE_DURATION_SECS {
                    return Ok(data.clone());
                }
            }
        }
    }

    // Create HTTP client with no proxy
    let client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch image: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}: Failed to fetch image", response.status()));
    }
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;
    
    // Return as base64 data URL
    let base64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:image/png;base64,{}", base64);
    
    // Cache the result
    {
        let mut cache = IMAGE_CACHE.lock().unwrap();
        cache.insert(url, (data_url.clone(), std::time::SystemTime::now()));
    }
    
    Ok(data_url)
}

// Clear all player-related caches
#[tauri::command]
pub fn clear_player_caches() -> Result<(), String> {
    {
        let mut cache = PLAYER_PROFILE_CACHE.lock().unwrap();
        cache.clear();
    }
    {
        let mut cache = IMAGE_CACHE.lock().unwrap();
        cache.clear();
    }
    Ok(())
}

fn parse_player_file(path: &Path) -> Result<PlayerData, String> {
    // Read and decompress the player .dat file
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open player file: {}", e))?;
    
    let mut decoder = GzDecoder::new(file);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to decompress player file: {}", e))?;

    // Parse NBT
    let nbt_value: fastnbt::Value = fastnbt::from_bytes(&bytes)
        .map_err(|e| format!("Failed to parse player NBT: {}", e))?;

    // Extract player data from NBT
    extract_player_data(&nbt_value)
}

fn extract_player_data(nbt: &fastnbt::Value) -> Result<PlayerData, String> {
    let compound = match nbt {
        fastnbt::Value::Compound(c) => c,
        _ => return Err("Invalid NBT structure".to_string()),
    };

    // Extract position
    let position = if let Some(fastnbt::Value::List(pos)) = compound.get("Pos") {
        if pos.len() >= 3 {
            PlayerPosition {
                x: match &pos[0] {
                    fastnbt::Value::Double(d) => *d,
                    fastnbt::Value::Float(f) => *f as f64,
                    _ => 0.0,
                },
                y: match &pos[1] {
                    fastnbt::Value::Double(d) => *d,
                    fastnbt::Value::Float(f) => *f as f64,
                    _ => 0.0,
                },
                z: match &pos[2] {
                    fastnbt::Value::Double(d) => *d,
                    fastnbt::Value::Float(f) => *f as f64,
                    _ => 0.0,
                },
            }
        } else {
            PlayerPosition { x: 0.0, y: 0.0, z: 0.0 }
        }
    } else {
        PlayerPosition { x: 0.0, y: 0.0, z: 0.0 }
    };

    // Extract health
    let health = match compound.get("Health") {
        Some(fastnbt::Value::Float(h)) => *h,
        Some(fastnbt::Value::Double(h)) => *h as f32,
        _ => 20.0,
    };

    // Extract food level
    let food_level = match compound.get("foodLevel") {
        Some(fastnbt::Value::Int(f)) => *f,
        Some(fastnbt::Value::Short(f)) => *f as i32,
        _ => 20,
    };

    // Extract XP
    let xp_level = match compound.get("XpLevel") {
        Some(fastnbt::Value::Int(x)) => *x,
        _ => 0,
    };

    let xp_total = match compound.get("XpTotal") {
        Some(fastnbt::Value::Int(x)) => *x,
        _ => 0,
    };

    // Extract game mode
    let game_mode = match compound.get("playerGameType") {
        Some(fastnbt::Value::Int(g)) => *g,
        _ => 0,
    };

    // Extract dimension
    let dimension = match compound.get("Dimension") {
        Some(fastnbt::Value::String(d)) => d.clone(),
        Some(fastnbt::Value::Int(d)) => match d {
            -1 => "minecraft:the_nether".to_string(),
            0 => "minecraft:overworld".to_string(),
            1 => "minecraft:the_end".to_string(),
            _ => format!("dimension_{}", d),
        },
        _ => "minecraft:overworld".to_string(),
    };

    Ok(PlayerData {
        edition: String::new(),     // Will be set by caller
        primary_id: String::new(),  // Will be set from filename
        display_name: None,         // TODO: Resolve via Mojang API
        position,
        health,
        food_level,
        xp_level,
        xp_total,
        game_mode,
        dimension,
        last_seen: None,            // TODO: Extract from player file
    })
}
