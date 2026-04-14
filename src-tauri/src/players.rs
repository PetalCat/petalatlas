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

// ============================================
// Extended types for full player data editing
// ============================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemEnchantment {
    pub id: String,
    pub level: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InventoryItem {
    pub id: String,
    pub count: i32,
    pub slot: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_damage: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enchantments: Option<Vec<ItemEnchantment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PotionEffect {
    pub id: String,
    pub amplifier: i32,
    pub duration: i32,
    #[serde(default)]
    pub show_particles: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInventory {
    pub main: Vec<InventoryItem>,      // Slots 9-35
    pub hotbar: Vec<InventoryItem>,    // Slots 0-8
    pub armor: Vec<InventoryItem>,     // Slots 100-103
    pub offhand: Option<InventoryItem>, // Slot -106
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FullPlayerData {
    // Basic stats (same as PlayerData)
    pub edition: String,
    pub primary_id: String,
    pub display_name: Option<String>,
    pub position: PlayerPosition,
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub saturation: f32,
    pub xp_level: i32,
    pub xp_progress: f32,  // 0.0 to 1.0
    pub xp_total: i32,
    pub game_mode: i32,
    pub dimension: String,
    
    // Extended data
    pub inventory: PlayerInventory,
    pub ender_items: Vec<InventoryItem>,
    pub active_effects: Vec<PotionEffect>,
    
    // Spawn point
    pub spawn_x: Option<i32>,
    pub spawn_y: Option<i32>,
    pub spawn_z: Option<i32>,
    pub spawn_dimension: Option<String>,
}

// ============================================
// Advancement types for reading player progress
// ============================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvancementCriterion {
    pub id: String,
    pub name: String,
    pub done: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvancementData {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub criteria: Vec<AdvancementCriterion>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AdvancementCategoryData {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub advancements: Vec<AdvancementData>,
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

// Get complete player data for editing
#[tauri::command]
pub fn get_player_full(world_path: String, uuid: String) -> Result<FullPlayerData, String> {
    let player_file = Path::new(&world_path)
        .join("playerdata")
        .join(format!("{}.dat", uuid));
    
    if !player_file.exists() {
        return Err("Player file not found".to_string());
    }

    let nbt = read_player_nbt(&player_file)?;
    let mut full_data = parse_full_player_data(&nbt)?;
    full_data.edition = "java".to_string();
    full_data.primary_id = uuid;
    
    Ok(full_data)
}

// Save modified player data back to file
#[tauri::command]
pub fn save_player_data(world_path: String, uuid: String, data: FullPlayerData) -> Result<(), String> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    
    let player_file = Path::new(&world_path)
        .join("playerdata")
        .join(format!("{}.dat", uuid));
    
    if !player_file.exists() {
        return Err("Player file not found".to_string());
    }
    
    // Create backup
    let backup_file = player_file.with_extension("dat.bak");
    fs::copy(&player_file, &backup_file)
        .map_err(|e| format!("Failed to create backup: {}", e))?;
    
    // Read existing NBT to preserve unedited fields
    let mut nbt = read_player_nbt(&player_file)?;
    let compound = nbt.as_object_mut().ok_or("Root is not a compound")?;
    
    // Update basic stats
    compound.insert("Health".to_string(), serde_json::json!(data.health));
    compound.insert("foodLevel".to_string(), serde_json::json!(data.food_level));
    compound.insert("foodSaturationLevel".to_string(), serde_json::json!(data.saturation));
    compound.insert("XpLevel".to_string(), serde_json::json!(data.xp_level));
    compound.insert("XpP".to_string(), serde_json::json!(data.xp_progress));
    compound.insert("XpTotal".to_string(), serde_json::json!(data.xp_total));
    compound.insert("playerGameType".to_string(), serde_json::json!(data.game_mode));
    
    // Update position
    compound.insert("Pos".to_string(), serde_json::json!([
        data.position.x,
        data.position.y,
        data.position.z
    ]));
    
    // Update dimension
    compound.insert("Dimension".to_string(), serde_json::json!(data.dimension));
    
    // Update inventory
    let mut inventory_items: Vec<serde_json::Value> = Vec::new();
    
    // Add hotbar items (slots 0-8)
    for item in &data.inventory.hotbar {
        inventory_items.push(item_to_nbt(item));
    }
    
    // Add main inventory (slots 9-35)
    for item in &data.inventory.main {
        inventory_items.push(item_to_nbt(item));
    }
    
    // Add armor (slots 100-103)
    for item in &data.inventory.armor {
        inventory_items.push(item_to_nbt(item));
    }
    
    // Add offhand (slot -106)
    if let Some(ref offhand) = data.inventory.offhand {
        inventory_items.push(item_to_nbt(offhand));
    }
    
    compound.insert("Inventory".to_string(), serde_json::json!(inventory_items));
    
    // Update ender chest
    let ender_items: Vec<serde_json::Value> = data.ender_items.iter()
        .map(|item| item_to_nbt(item))
        .collect();
    compound.insert("EnderItems".to_string(), serde_json::json!(ender_items));
    
    // Update effects
    let effects: Vec<serde_json::Value> = data.active_effects.iter()
        .map(|effect| serde_json::json!({
            "Id": effect.id.clone(),
            "Amplifier": effect.amplifier,
            "Duration": effect.duration,
            "ShowParticles": effect.show_particles
        }))
        .collect();
    compound.insert("ActiveEffects".to_string(), serde_json::json!(effects));
    
    // Update spawn point
    if let Some(x) = data.spawn_x {
        compound.insert("SpawnX".to_string(), serde_json::json!(x));
    }
    if let Some(y) = data.spawn_y {
        compound.insert("SpawnY".to_string(), serde_json::json!(y));
    }
    if let Some(z) = data.spawn_z {
        compound.insert("SpawnZ".to_string(), serde_json::json!(z));
    }
    if let Some(ref dim) = data.spawn_dimension {
        compound.insert("SpawnDimension".to_string(), serde_json::json!(dim));
    }
    
    // Convert back to NBT and write
    // Note: This is a simplified approach - for full fidelity we'd need to preserve NBT types
    let nbt_tag: quartz_nbt::NbtCompound = serde_json::from_value(nbt.clone())
        .map_err(|e| format!("Failed to convert to NBT: {}", e))?;
    
    let mut bytes = Vec::new();
    quartz_nbt::io::write_nbt(&mut bytes, None, &nbt_tag, quartz_nbt::io::Flavor::Uncompressed)
        .map_err(|e| format!("Failed to write NBT: {}", e))?;
    
    // Compress and write
    let file = fs::File::create(&player_file)
        .map_err(|e| format!("Failed to create player file: {}", e))?;
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&bytes)
        .map_err(|e| format!("Failed to write compressed data: {}", e))?;
    encoder.finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;
    
    Ok(())
}

fn item_to_nbt(item: &InventoryItem) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "id": item.id.clone(),
        "Count": item.count,
        "Slot": item.slot
    });
    
    if let Some(damage) = item.damage {
        obj["Damage"] = serde_json::json!(damage);
    }
    
    if let Some(ref name) = item.custom_name {
        obj["tag"] = serde_json::json!({
            "display": {
                "Name": format!("{{\"text\":\"{}\"}}", name)
            }
        });
    }
    
    if let Some(ref enchants) = item.enchantments {
        let ench_arr: Vec<serde_json::Value> = enchants.iter()
            .map(|e| serde_json::json!({
                "id": e.id.clone(),
                "lvl": e.level
            }))
            .collect();
        
        if obj.get("tag").is_none() {
            obj["tag"] = serde_json::json!({});
        }
        obj["tag"]["Enchantments"] = serde_json::json!(ench_arr);
    }
    
    obj
}

fn read_player_nbt(path: &Path) -> Result<serde_json::Value, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open player file: {}", e))?;
    
    let mut decoder = GzDecoder::new(file);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to decompress player file: {}", e))?;

    let mut cursor = std::io::Cursor::new(&bytes);
    match quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::Uncompressed) {
        Ok((tag, _)) => serde_json::to_value(tag)
            .map_err(|e| format!("Failed to convert NBT: {}", e)),
        Err(_) => Err("Failed to parse player NBT".to_string()),
    }
}

fn parse_full_player_data(nbt: &serde_json::Value) -> Result<FullPlayerData, String> {
    let compound = nbt.as_object().ok_or("Root is not a compound")?;

    // Position
    let position = if let Some(serde_json::Value::Array(pos)) = compound.get("Pos") {
        if pos.len() >= 3 {
            PlayerPosition {
                x: pos[0].as_f64().unwrap_or(0.0),
                y: pos[1].as_f64().unwrap_or(0.0),
                z: pos[2].as_f64().unwrap_or(0.0),
            }
        } else {
            PlayerPosition { x: 0.0, y: 0.0, z: 0.0 }
        }
    } else {
        PlayerPosition { x: 0.0, y: 0.0, z: 0.0 }
    };

    // Dimension
    let dimension = parse_dimension(compound.get("Dimension"));

    // Basic stats
    let health = compound.get("Health")
        .and_then(|v| v.as_f64())
        .unwrap_or(20.0) as f32;
    
    let max_health = compound.get("Attributes")
        .and_then(|attrs| attrs.as_array())
        .and_then(|arr| arr.iter().find(|a| {
            a.get("Name").and_then(|n| n.as_str()) == Some("minecraft:generic.max_health")
        }))
        .and_then(|attr| attr.get("Base"))
        .and_then(|v| v.as_f64())
        .unwrap_or(20.0) as f32;
    
    let food_level = compound.get("foodLevel")
        .and_then(|v| v.as_i64())
        .unwrap_or(20) as i32;
    
    let saturation = compound.get("foodSaturationLevel")
        .and_then(|v| v.as_f64())
        .unwrap_or(5.0) as f32;
    
    let xp_level = compound.get("XpLevel")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    
    let xp_progress = compound.get("XpP")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    
    let xp_total = compound.get("XpTotal")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    
    let game_mode = compound.get("playerGameType")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    // Parse inventory
    let inventory = parse_inventory(compound.get("Inventory"));
    
    // Parse ender chest
    let ender_items = parse_items_list(compound.get("EnderItems"));
    
    // Parse active effects
    let active_effects = parse_effects(compound.get("ActiveEffects"));
    
    // Spawn point
    let spawn_x = compound.get("SpawnX").and_then(|v| v.as_i64()).map(|v| v as i32);
    let spawn_y = compound.get("SpawnY").and_then(|v| v.as_i64()).map(|v| v as i32);
    let spawn_z = compound.get("SpawnZ").and_then(|v| v.as_i64()).map(|v| v as i32);
    let spawn_dimension = compound.get("SpawnDimension")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(FullPlayerData {
        edition: String::new(),
        primary_id: String::new(),
        display_name: None,
        position,
        health,
        max_health,
        food_level,
        saturation,
        xp_level,
        xp_progress,
        xp_total,
        game_mode,
        dimension,
        inventory,
        ender_items,
        active_effects,
        spawn_x,
        spawn_y,
        spawn_z,
        spawn_dimension,
    })
}

fn parse_dimension(dim: Option<&serde_json::Value>) -> String {
    match dim {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => {
            match n.as_i64().unwrap_or(0) {
                -1 => "minecraft:the_nether".to_string(),
                0 => "minecraft:overworld".to_string(),
                1 => "minecraft:the_end".to_string(),
                _ => format!("dimension_{}", n),
            }
        },
        _ => "minecraft:overworld".to_string(),
    }
}

fn parse_inventory(inv: Option<&serde_json::Value>) -> PlayerInventory {
    let items = parse_items_list(inv);
    
    let mut main = Vec::new();
    let mut hotbar = Vec::new();
    let mut armor = Vec::new();
    let mut offhand = None;
    
    for item in items {
        match item.slot {
            0..=8 => hotbar.push(item),
            9..=35 => main.push(item),
            100..=103 => armor.push(item),
            -106 => offhand = Some(item),
            _ => {} // Unknown slot
        }
    }
    
    // Sort armor by slot (boots=100, leggings=101, chestplate=102, helmet=103)
    armor.sort_by_key(|i| i.slot);
    
    PlayerInventory { main, hotbar, armor, offhand }
}

fn parse_items_list(items: Option<&serde_json::Value>) -> Vec<InventoryItem> {
    let Some(serde_json::Value::Array(arr)) = items else {
        return Vec::new();
    };
    
    arr.iter().filter_map(|item| parse_single_item(item)).collect()
}

fn parse_single_item(item: &serde_json::Value) -> Option<InventoryItem> {
    let obj = item.as_object()?;
    
    let id = obj.get("id")?.as_str()?.to_string();
    let count = obj.get("Count")
        .and_then(|v| v.as_i64())
        .unwrap_or(1) as i32;
    let slot = obj.get("Slot")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    
    let damage = obj.get("Damage")
        .or_else(|| obj.get("tag").and_then(|t| t.get("Damage")))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);
    
    // Try to get max damage from item registry (simplified)
    let max_damage = get_item_max_damage(&id);
    
    // Custom name from tag.display.Name
    let custom_name = obj.get("tag")
        .and_then(|t| t.get("display"))
        .and_then(|d| d.get("Name"))
        .and_then(|n| n.as_str())
        .and_then(|s| {
            // Try to parse JSON text component
            if s.starts_with('{') {
                serde_json::from_str::<serde_json::Value>(s)
                    .ok()
                    .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
            } else {
                Some(s.to_string())
            }
        });
    
    // Enchantments
    let enchantments = obj.get("tag")
        .and_then(|t| t.get("Enchantments").or_else(|| t.get("ench")))
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter().filter_map(|ench| {
                let ench_obj = ench.as_object()?;
                let id = ench_obj.get("id")?.as_str()?.to_string();
                let level = ench_obj.get("lvl")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(1) as i32;
                Some(ItemEnchantment { id, level })
            }).collect()
        });
    
    // Raw tag data
    let tag = obj.get("tag").cloned();
    
    Some(InventoryItem {
        id,
        count,
        slot,
        damage,
        max_damage,
        custom_name,
        enchantments,
        tag,
    })
}

fn get_item_max_damage(id: &str) -> Option<i32> {
    // Simplified max durability lookup
    match id {
        "minecraft:diamond_sword" => Some(1561),
        "minecraft:diamond_pickaxe" => Some(1561),
        "minecraft:diamond_axe" => Some(1561),
        "minecraft:diamond_shovel" => Some(1561),
        "minecraft:diamond_hoe" => Some(1561),
        "minecraft:diamond_helmet" => Some(363),
        "minecraft:diamond_chestplate" => Some(528),
        "minecraft:diamond_leggings" => Some(495),
        "minecraft:diamond_boots" => Some(429),
        "minecraft:netherite_sword" => Some(2031),
        "minecraft:netherite_pickaxe" => Some(2031),
        "minecraft:netherite_axe" => Some(2031),
        "minecraft:netherite_shovel" => Some(2031),
        "minecraft:netherite_hoe" => Some(2031),
        "minecraft:netherite_helmet" => Some(407),
        "minecraft:netherite_chestplate" => Some(592),
        "minecraft:netherite_leggings" => Some(555),
        "minecraft:netherite_boots" => Some(481),
        "minecraft:iron_sword" => Some(250),
        "minecraft:iron_pickaxe" => Some(250),
        "minecraft:iron_axe" => Some(250),
        "minecraft:iron_shovel" => Some(250),
        "minecraft:iron_hoe" => Some(250),
        "minecraft:iron_helmet" => Some(165),
        "minecraft:iron_chestplate" => Some(240),
        "minecraft:iron_leggings" => Some(225),
        "minecraft:iron_boots" => Some(195),
        "minecraft:bow" => Some(384),
        "minecraft:crossbow" => Some(465),
        "minecraft:trident" => Some(250),
        "minecraft:elytra" => Some(432),
        "minecraft:shield" => Some(336),
        "minecraft:fishing_rod" => Some(64),
        "minecraft:flint_and_steel" => Some(64),
        "minecraft:shears" => Some(238),
        _ => None,
    }
}

fn parse_effects(effects: Option<&serde_json::Value>) -> Vec<PotionEffect> {
    let Some(serde_json::Value::Array(arr)) = effects else {
        return Vec::new();
    };
    
    arr.iter().filter_map(|effect| {
        let obj = effect.as_object()?;
        
        // Effect ID can be numeric or string depending on version
        let id = if let Some(id_val) = obj.get("Id") {
            match id_val {
                serde_json::Value::Number(n) => {
                    // Convert numeric ID to string name
                    effect_id_to_string(n.as_i64().unwrap_or(0) as i32)
                },
                serde_json::Value::String(s) => s.clone(),
                _ => return None,
            }
        } else {
            return None;
        };
        
        let amplifier = obj.get("Amplifier")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        
        let duration = obj.get("Duration")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        
        let show_particles = obj.get("ShowParticles")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        Some(PotionEffect { id, amplifier, duration, show_particles })
    }).collect()
}

fn effect_id_to_string(id: i32) -> String {
    (match id {
        1 => "minecraft:speed",
        2 => "minecraft:slowness",
        3 => "minecraft:haste",
        4 => "minecraft:mining_fatigue",
        5 => "minecraft:strength",
        6 => "minecraft:instant_health",
        7 => "minecraft:instant_damage",
        8 => "minecraft:jump_boost",
        9 => "minecraft:nausea",
        10 => "minecraft:regeneration",
        11 => "minecraft:resistance",
        12 => "minecraft:fire_resistance",
        13 => "minecraft:water_breathing",
        14 => "minecraft:invisibility",
        15 => "minecraft:blindness",
        16 => "minecraft:night_vision",
        17 => "minecraft:hunger",
        18 => "minecraft:weakness",
        19 => "minecraft:poison",
        20 => "minecraft:wither",
        21 => "minecraft:health_boost",
        22 => "minecraft:absorption",
        23 => "minecraft:saturation",
        24 => "minecraft:glowing",
        25 => "minecraft:levitation",
        26 => "minecraft:luck",
        27 => "minecraft:unluck",
        28 => "minecraft:slow_falling",
        29 => "minecraft:conduit_power",
        30 => "minecraft:dolphins_grace",
        31 => "minecraft:bad_omen",
        32 => "minecraft:hero_of_the_village",
        _ => return format!("minecraft:effect_{}", id),
    }).to_string()
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

// Get player advancements from world's advancements folder
#[tauri::command]
pub fn get_player_advancements(world_path: String, uuid: String) -> Result<Vec<AdvancementCategoryData>, String> {
    let advancements_file = Path::new(&world_path)
        .join("advancements")
        .join(format!("{}.json", uuid));
    
    if !advancements_file.exists() {
        // Return empty categories if no advancements file exists
        return Ok(get_empty_advancement_categories());
    }
    
    // Read and parse the JSON file
    let content = fs::read_to_string(&advancements_file)
        .map_err(|e| format!("Failed to read advancements file: {}", e))?;
    
    let raw: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse advancements JSON: {}", e))?;
    
    let obj = raw.as_object().ok_or("Advancements file is not a JSON object")?;
    
    // Group advancements by category
    let mut story_advs = Vec::new();
    let mut adventure_advs = Vec::new();
    let mut husbandry_advs = Vec::new();
    let mut nether_advs = Vec::new();
    let mut end_advs = Vec::new();
    
    // Iterate over ALL known advancements, not just the ones in the player's file
    for adv_id in get_all_advancement_ids() {
        if let Some(adv_data) = get_advancement_metadata(adv_id) {
            // Check if the player has this advancement
            let (criteria, completed) = if let Some(progress) = obj.get(adv_id) {
                let criteria = parse_advancement_criteria(adv_id, progress);
                let completed = progress.get("done").and_then(|d| d.as_bool()).unwrap_or(false)
                    || criteria.iter().all(|c| c.done);
                (criteria, completed)
            } else {
                // Player doesn't have this advancement - show as incomplete
                let criteria = get_advancement_criteria(adv_id)
                    .map(|ids| ids.iter().map(|id| AdvancementCriterion {
                        id: id.to_string(),
                        name: get_criterion_display_name(adv_id, id),
                        done: false,
                    }).collect())
                    .unwrap_or_else(|| vec![AdvancementCriterion {
                        id: adv_id.to_string(),
                        name: adv_data.0.to_string(),
                        done: false,
                    }]);
                (criteria, false)
            };
            
            let advancement = AdvancementData {
                id: adv_id.to_string(),
                title: adv_data.0.to_string(),
                description: adv_data.1.to_string(),
                completed,
                criteria,
            };
            
            // Categorize based on ID prefix
            if adv_id.starts_with("minecraft:story/") {
                story_advs.push(advancement);
            } else if adv_id.starts_with("minecraft:adventure/") {
                adventure_advs.push(advancement);
            } else if adv_id.starts_with("minecraft:husbandry/") {
                husbandry_advs.push(advancement);
            } else if adv_id.starts_with("minecraft:nether/") {
                nether_advs.push(advancement);
            } else if adv_id.starts_with("minecraft:end/") {
                end_advs.push(advancement);
            }
        }
    }
    
    let mut categories = Vec::new();
    
    // Always add all categories (even if no progress yet)
    categories.push(AdvancementCategoryData {
        id: "minecraft:story".to_string(),
        name: "Minecraft".to_string(),
        icon: "🌍".to_string(),
        advancements: story_advs,
    });
    
    categories.push(AdvancementCategoryData {
        id: "minecraft:adventure".to_string(),
        name: "Adventure".to_string(),
        icon: "🗺️".to_string(),
        advancements: adventure_advs,
    });
    
    categories.push(AdvancementCategoryData {
        id: "minecraft:husbandry".to_string(),
        name: "Husbandry".to_string(),
        icon: "🌾".to_string(),
        advancements: husbandry_advs,
    });
    
    categories.push(AdvancementCategoryData {
        id: "minecraft:nether".to_string(),
        name: "Nether".to_string(),
        icon: "🔥".to_string(),
        advancements: nether_advs,
    });
    
    categories.push(AdvancementCategoryData {
        id: "minecraft:end".to_string(),
        name: "The End".to_string(),
        icon: "🐉".to_string(),
        advancements: end_advs,
    });
    
    Ok(categories)
}

fn get_empty_advancement_categories() -> Vec<AdvancementCategoryData> {
    Vec::new()
}

fn parse_advancement_criteria(adv_id: &str, progress: &serde_json::Value) -> Vec<AdvancementCriterion> {
    let mut criteria = Vec::new();
    
    if let Some(criteria_obj) = progress.get("criteria").and_then(|c| c.as_object()) {
        for (criterion_id, _timestamp) in criteria_obj.iter() {
            let name = get_criterion_display_name(adv_id, criterion_id);
            criteria.push(AdvancementCriterion {
                id: criterion_id.clone(),
                name,
                done: true, // If it's in the criteria object, it's completed
            });
        }
    }
    
    // Add missing criteria from known advancement definitions
    if let Some(all_criteria) = get_advancement_criteria(adv_id) {
        for criterion_id in all_criteria {
            if !criteria.iter().any(|c| c.id == criterion_id) {
                let name = get_criterion_display_name(adv_id, criterion_id);
                criteria.push(AdvancementCriterion {
                    id: criterion_id.to_string(),
                    name,
                    done: false,
                });
            }
        }
    }
    
    // Sort by ID for consistent display
    criteria.sort_by(|a, b| a.id.cmp(&b.id));
    
    criteria
}

fn get_criterion_display_name(_adv_id: &str, criterion_id: &str) -> String {
    // For most advancements, the criterion ID is descriptive enough
    // Clean up the ID to be more readable
    let cleaned = criterion_id
        .replace("minecraft:", "")
        .replace("_", " ");
    
    // Capitalize first letter of each word
    cleaned
        .split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// Returns (title, description) for known advancements
fn get_advancement_metadata(adv_id: &str) -> Option<(&'static str, &'static str)> {
    match adv_id {
        // Story (Minecraft) advancements
        "minecraft:story/root" => Some(("Minecraft", "The heart and story of the game")),
        "minecraft:story/mine_stone" => Some(("Stone Age", "Mine stone with your new pickaxe")),
        "minecraft:story/upgrade_tools" => Some(("Getting an Upgrade", "Construct a better pickaxe")),
        "minecraft:story/smelt_iron" => Some(("Acquire Hardware", "Smelt an iron ingot")),
        "minecraft:story/obtain_armor" => Some(("Suit Up", "Protect yourself with a piece of iron armor")),
        "minecraft:story/lava_bucket" => Some(("Hot Stuff", "Fill a bucket with lava")),
        "minecraft:story/iron_tools" => Some(("Isn't It Iron Pick", "Upgrade your pickaxe")),
        "minecraft:story/deflect_arrow" => Some(("Not Today, Thank You", "Deflect a projectile with a shield")),
        "minecraft:story/form_obsidian" => Some(("Ice Bucket Challenge", "Obtain a block of obsidian")),
        "minecraft:story/mine_diamond" => Some(("Diamonds!", "Acquire diamonds")),
        "minecraft:story/enter_the_nether" => Some(("We Need to Go Deeper", "Build, light and enter a Nether Portal")),
        "minecraft:story/shiny_gear" => Some(("Cover Me with Diamonds", "Diamond armor saves lives")),
        "minecraft:story/enchant_item" => Some(("Enchanter", "Enchant an item at an Enchanting Table")),
        "minecraft:story/cure_zombie_villager" => Some(("Zombie Doctor", "Weaken and then cure a Zombie Villager")),
        "minecraft:story/follow_ender_eye" => Some(("Eye Spy", "Follow an Eye of Ender")),
        "minecraft:story/enter_the_end" => Some(("The End?", "Enter the End Portal")),
        
        // Nether advancements
        "minecraft:nether/root" => Some(("Nether", "Bring summer clothes")),
        "minecraft:nether/return_to_sender" => Some(("Return to Sender", "Destroy a Ghast with a fireball")),
        "minecraft:nether/find_bastion" => Some(("Those Were the Days", "Enter a Bastion Remnant")),
        "minecraft:nether/obtain_ancient_debris" => Some(("Hidden in the Depths", "Obtain Ancient Debris")),
        "minecraft:nether/fast_travel" => Some(("Subspace Bubble", "Use the Nether to travel 7km in the Overworld")),
        "minecraft:nether/find_fortress" => Some(("A Terrible Fortress", "Break your way into a Nether Fortress")),
        "minecraft:nether/obtain_crying_obsidian" => Some(("Who is Cutting Onions?", "Obtain Crying Obsidian")),
        "minecraft:nether/distract_piglin" => Some(("Oh Shiny", "Distract Piglins with gold")),
        "minecraft:nether/ride_strider" => Some(("This Boat Has Legs", "Ride a Strider with a Warped Fungus on a Stick")),
        "minecraft:nether/uneasy_alliance" => Some(("Uneasy Alliance", "Rescue a Ghast from the Nether, bring it safely home to the Overworld... and then kill it")),
        "minecraft:nether/loot_bastion" => Some(("War Pigs", "Loot a chest in a Bastion Remnant")),
        "minecraft:nether/use_lodestone" => Some(("Country Lode, Take Me Home", "Use a Compass on a Lodestone")),
        "minecraft:nether/netherite_armor" => Some(("Cover Me in Debris", "Get a full suit of Netherite armor")),
        "minecraft:nether/get_wither_skull" => Some(("Spooky Scary Skeleton", "Obtain a Wither Skeleton's skull")),
        "minecraft:nether/obtain_blaze_rod" => Some(("Into Fire", "Relieve a Blaze of its rod")),
        "minecraft:nether/charge_respawn_anchor" => Some(("Not Quite 'Nine' Lives", "Charge a Respawn Anchor to full power")),
        "minecraft:nether/ride_strider_in_overworld_lava" => Some(("Feels Like Home", "Take a Strider for a loooong ride on a lava lake in the Overworld")),
        "minecraft:nether/explore_nether" => Some(("Hot Tourist Destinations", "Explore all Nether biomes")),
        "minecraft:nether/summon_wither" => Some(("Withering Heights", "Summon the Wither")),
        "minecraft:nether/brew_potion" => Some(("Local Brewery", "Brew a potion")),
        "minecraft:nether/create_beacon" => Some(("Bring Home the Beacon", "Construct and place a beacon")),
        "minecraft:nether/all_potions" => Some(("A Furious Cocktail", "Have every potion effect applied at the same time")),
        "minecraft:nether/create_full_beacon" => Some(("Beaconator", "Bring a beacon to full power")),
        "minecraft:nether/all_effects" => Some(("How Did We Get Here?", "Have every effect applied at the same time")),
        
        // End advancements
        "minecraft:end/root" => Some(("The End", "Or the beginning?")),
        "minecraft:end/kill_dragon" => Some(("Free the End", "Good luck")),
        "minecraft:end/dragon_egg" => Some(("The Next Generation", "Hold the Dragon Egg")),
        "minecraft:end/enter_end_gateway" => Some(("Remote Getaway", "Escape the island")),
        "minecraft:end/respawn_dragon" => Some(("The End... Again...", "Respawn the Ender Dragon")),
        "minecraft:end/dragon_breath" => Some(("You Need a Mint", "Collect dragon's breath in a glass bottle")),
        "minecraft:end/find_end_city" => Some(("The City at the End of the Game", "Go on in, what could happen?")),
        "minecraft:end/elytra" => Some(("Sky's the Limit", "Find elytra")),
        "minecraft:end/levitate" => Some(("Great View From Up Here", "Levitate up 50 blocks from the attacks of a Shulker")),
        
        // Adventure advancements
        "minecraft:adventure/root" => Some(("Adventure", "Adventure, exploration, and combat")),
        "minecraft:adventure/voluntary_exile" => Some(("Voluntary Exile", "Kill a raid captain. Maybe consider staying away from villages for the time being...")),
        "minecraft:adventure/spyglass_at_parrot" => Some(("Is It a Bird?", "Look at a parrot through a spyglass")),
        "minecraft:adventure/kill_a_mob" => Some(("Monster Hunter", "Kill any hostile monster")),
        "minecraft:adventure/trade" => Some(("What a Deal!", "Successfully trade with a Villager")),
        "minecraft:adventure/honey_block_slide" => Some(("Sticky Situation", "Jump into a Honey Block to break your fall")),
        "minecraft:adventure/ol_betsy" => Some(("Ol' Betsy", "Shoot a crossbow")),
        "minecraft:adventure/sleep_in_bed" => Some(("Sweet Dreams", "Sleep in a bed to change your respawn point")),
        "minecraft:adventure/hero_of_the_village" => Some(("Hero of the Village", "Successfully defend a village from a raid")),
        "minecraft:adventure/spyglass_at_ghast" => Some(("Is It a Balloon?", "Look at a ghast through a spyglass")),
        "minecraft:adventure/throw_trident" => Some(("A Throwaway Joke", "Throw a trident at something. Note: Throwing away your only weapon is not a good idea.")),
        "minecraft:adventure/kill_all_mobs" => Some(("Monsters Hunted", "Kill one of every hostile monster")),
        "minecraft:adventure/totem_of_undying" => Some(("Postmortal", "Use a Totem of Undying to cheat death")),
        "minecraft:adventure/summon_iron_golem" => Some(("Hired Help", "Summon an Iron Golem to help defend a village")),
        "minecraft:adventure/trade_at_world_height" => Some(("Star Trader", "Trade with a Villager at the build height limit")),
        "minecraft:adventure/two_birds_one_arrow" => Some(("Two Birds, One Arrow", "Kill two Phantoms with a piercing arrow")),
        "minecraft:adventure/whos_the_pillager_now" => Some(("Who's the Pillager Now?", "Give a Pillager a taste of their own medicine")),
        "minecraft:adventure/arbalistic" => Some(("Arbalistic", "Kill five unique mobs with one crossbow shot")),
        "minecraft:adventure/adventuring_time" => Some(("Adventuring Time", "Discover every biome")),
        "minecraft:adventure/play_jukebox_in_meadows" => Some(("Sound of Music", "Make the Meadows come alive with the sound of music from a jukebox")),
        "minecraft:adventure/walk_on_powder_snow_with_leather_boots" => Some(("Light as a Rabbit", "Walk on powder snow... without sinking in it")),
        "minecraft:adventure/lightning_rod_with_villager_no_fire" => Some(("Surge Protector", "Protect a villager from an undesired shock without starting a fire")),
        "minecraft:adventure/fall_from_world_height" => Some(("Caves & Cliffs", "Free fall from the top of the world to the bottom and survive")),
        "minecraft:adventure/salvage_sherd" => Some(("Respecting the Remnants", "Brush a Suspicious block to obtain a Pottery Sherd")),
        "minecraft:adventure/avoid_vibration" => Some(("Sneak 100", "Sneak near a Sculk Sensor or Warden to prevent it from detecting you")),
        "minecraft:adventure/spyglass_at_dragon" => Some(("Is It a Plane?", "Look at the Ender Dragon through a spyglass")),
        "minecraft:adventure/very_very_frightening" => Some(("Very Very Frightening", "Strike a Villager with lightning")),
        "minecraft:adventure/sniper_duel" => Some(("Sniper Duel", "Kill a Skeleton from at least 50 meters away")),
        "minecraft:adventure/bullseye" => Some(("Bullseye", "Hit the bullseye of a Target block from at least 30 meters away")),
        
        // Husbandry advancements
        "minecraft:husbandry/root" => Some(("Husbandry", "The world is full of friends and food")),
        "minecraft:husbandry/safely_harvest_honey" => Some(("Bee Our Guest", "Use a Campfire to collect Honey from a Beehive using a Bottle without aggravating the bees")),
        "minecraft:husbandry/breed_an_animal" => Some(("The Parrots and the Bats", "Breed two animals together")),
        "minecraft:husbandry/tame_an_animal" => Some(("Best Friends Forever", "Tame an animal")),
        "minecraft:husbandry/fishy_business" => Some(("Fishy Business", "Catch a fish")),
        "minecraft:husbandry/silk_touch_nest" => Some(("Total Beelocation", "Move a Bee Nest, with 3 bees inside, using Silk Touch")),
        "minecraft:husbandry/plant_seed" => Some(("A Seedy Place", "Plant a seed and watch it grow")),
        "minecraft:husbandry/wax_on" => Some(("Wax On", "Apply Honeycomb to a Copper block!")),
        "minecraft:husbandry/bred_all_animals" => Some(("Two by Two", "Breed all the animals!")),
        "minecraft:husbandry/complete_catalogue" => Some(("A Complete Catalogue", "Tame all cat variants!")),
        "minecraft:husbandry/tactical_fishing" => Some(("Tactical Fishing", "Catch a fish... without a fishing rod!")),
        "minecraft:husbandry/balanced_diet" => Some(("A Balanced Diet", "Eat everything that is edible, even if it's not good for you")),
        "minecraft:husbandry/obtain_netherite_hoe" => Some(("Serious Dedication", "Use a Netherite Ingot to upgrade a hoe, and then reevaluate your life choices")),
        "minecraft:husbandry/wax_off" => Some(("Wax Off", "Scrape Wax off of a Copper block!")),
        "minecraft:husbandry/axolotl_in_a_bucket" => Some(("The Cutest Predator", "Catch an axolotl in a bucket")),
        "minecraft:husbandry/kill_axolotl_target" => Some(("The Healing Power of Friendship!", "Team up with an axolotl and win a fight")),
        "minecraft:husbandry/make_a_sign_glow" => Some(("Glow and Behold!", "Make the text of any kind of sign glow")),
        "minecraft:husbandry/ride_a_boat_with_a_goat" => Some(("Whatever Floats Your Goat!", "Get in a boat and float with a goat")),
        "minecraft:husbandry/leash_all_frog_variants" => Some(("When the Squad Hops into Town", "Get each Frog variant on a Lead")),
        "minecraft:husbandry/froglights" => Some(("With Our Powers Combined!", "Have all Froglights in your inventory")),
        "minecraft:husbandry/allay_deliver_item_to_player" => Some(("You've Got a Friend in Me", "Have an Allay deliver items to you")),
        "minecraft:husbandry/allay_deliver_cake_to_note_block" => Some(("Birthday Song", "Have an Allay drop a Cake at a Note Block")),
        "minecraft:husbandry/tadpole_in_a_bucket" => Some(("Bukkit Bukkit", "Catch a Tadpole in a Bucket")),
        "minecraft:husbandry/obtain_sniffer_egg" => Some(("Smells Interesting", "Obtain a Sniffer Egg")),
        "minecraft:husbandry/feed_snifflet" => Some(("Little Sniffs", "Feed a Snifflet")),
        "minecraft:husbandry/plant_any_sniffer_seed" => Some(("Planting the Past", "Plant any Sniffer seed")),
        
        _ => None, // Unknown advancement
    }
}

// Returns all known vanilla advancement IDs
fn get_all_advancement_ids() -> Vec<&'static str> {
    vec![
        // Story advancements
        "minecraft:story/root",
        "minecraft:story/mine_stone",
        "minecraft:story/upgrade_tools",
        "minecraft:story/smelt_iron",
        "minecraft:story/obtain_armor",
        "minecraft:story/lava_bucket",
        "minecraft:story/iron_tools",
        "minecraft:story/deflect_arrow",
        "minecraft:story/form_obsidian",
        "minecraft:story/mine_diamond",
        "minecraft:story/enter_the_nether",
        "minecraft:story/shiny_gear",
        "minecraft:story/enchant_item",
        "minecraft:story/cure_zombie_villager",
        "minecraft:story/follow_ender_eye",
        "minecraft:story/enter_the_end",
        
        // Nether advancements
        "minecraft:nether/root",
        "minecraft:nether/return_to_sender",
        "minecraft:nether/find_bastion",
        "minecraft:nether/obtain_ancient_debris",
        "minecraft:nether/fast_travel",
        "minecraft:nether/find_fortress",
        "minecraft:nether/obtain_crying_obsidian",
        "minecraft:nether/distract_piglin",
        "minecraft:nether/ride_strider",
        "minecraft:nether/uneasy_alliance",
        "minecraft:nether/loot_bastion",
        "minecraft:nether/use_lodestone",
        "minecraft:nether/netherite_armor",
        "minecraft:nether/get_wither_skull",
        "minecraft:nether/obtain_blaze_rod",
        "minecraft:nether/charge_respawn_anchor",
        "minecraft:nether/ride_strider_in_overworld_lava",
        "minecraft:nether/explore_nether",
        "minecraft:nether/summon_wither",
        "minecraft:nether/brew_potion",
        "minecraft:nether/create_beacon",
        "minecraft:nether/all_potions",
        "minecraft:nether/create_full_beacon",
        "minecraft:nether/all_effects",
        
        // End advancements
        "minecraft:end/root",
        "minecraft:end/kill_dragon",
        "minecraft:end/dragon_egg",
        "minecraft:end/enter_end_gateway",
        "minecraft:end/respawn_dragon",
        "minecraft:end/dragon_breath",
        "minecraft:end/find_end_city",
        "minecraft:end/elytra",
        "minecraft:end/levitate",
        
        // Adventure advancements
        "minecraft:adventure/root",
        "minecraft:adventure/voluntary_exile",
        "minecraft:adventure/spyglass_at_parrot",
        "minecraft:adventure/kill_a_mob",
        "minecraft:adventure/trade",
        "minecraft:adventure/honey_block_slide",
        "minecraft:adventure/ol_betsy",
        "minecraft:adventure/sleep_in_bed",
        "minecraft:adventure/hero_of_the_village",
        "minecraft:adventure/spyglass_at_ghast",
        "minecraft:adventure/throw_trident",
        "minecraft:adventure/kill_all_mobs",
        "minecraft:adventure/totem_of_undying",
        "minecraft:adventure/summon_iron_golem",
        "minecraft:adventure/trade_at_world_height",
        "minecraft:adventure/two_birds_one_arrow",
        "minecraft:adventure/whos_the_pillager_now",
        "minecraft:adventure/arbalistic",
        "minecraft:adventure/adventuring_time",
        "minecraft:adventure/play_jukebox_in_meadows",
        "minecraft:adventure/walk_on_powder_snow_with_leather_boots",
        "minecraft:adventure/lightning_rod_with_villager_no_fire",
        "minecraft:adventure/fall_from_world_height",
        "minecraft:adventure/salvage_sherd",
        "minecraft:adventure/avoid_vibration",
        "minecraft:adventure/spyglass_at_dragon",
        "minecraft:adventure/very_very_frightening",
        "minecraft:adventure/sniper_duel",
        "minecraft:adventure/bullseye",
        
        // Husbandry advancements
        "minecraft:husbandry/root",
        "minecraft:husbandry/safely_harvest_honey",
        "minecraft:husbandry/breed_an_animal",
        "minecraft:husbandry/tame_an_animal",
        "minecraft:husbandry/fishy_business",
        "minecraft:husbandry/silk_touch_nest",
        "minecraft:husbandry/plant_seed",
        "minecraft:husbandry/wax_on",
        "minecraft:husbandry/bred_all_animals",
        "minecraft:husbandry/complete_catalogue",
        "minecraft:husbandry/tactical_fishing",
        "minecraft:husbandry/balanced_diet",
        "minecraft:husbandry/obtain_netherite_hoe",
        "minecraft:husbandry/wax_off",
        "minecraft:husbandry/axolotl_in_a_bucket",
        "minecraft:husbandry/kill_axolotl_target",
        "minecraft:husbandry/make_a_sign_glow",
        "minecraft:husbandry/ride_a_boat_with_a_goat",
        "minecraft:husbandry/leash_all_frog_variants",
        "minecraft:husbandry/froglights",
        "minecraft:husbandry/allay_deliver_item_to_player",
        "minecraft:husbandry/allay_deliver_cake_to_note_block",
        "minecraft:husbandry/tadpole_in_a_bucket",
        "minecraft:husbandry/obtain_sniffer_egg",
        "minecraft:husbandry/feed_snifflet",
        "minecraft:husbandry/plant_any_sniffer_seed",
    ]
}

// Returns known criteria IDs for multi-criteria advancements
fn get_advancement_criteria(adv_id: &str) -> Option<Vec<&'static str>> {
    match adv_id {
        // Most advancements have a single criterion with the same name as the advancement
        // Here we only list multi-criteria advancements
        "minecraft:adventure/kill_all_mobs" => Some(vec![
            "blaze", "cave_spider", "creeper", "drowned", "elder_guardian", "ender_dragon",
            "enderman", "endermite", "evoker", "ghast", "guardian", "hoglin", "husk",
            "magma_cube", "phantom", "piglin", "piglin_brute", "pillager", "ravager",
            "shulker", "silverfish", "skeleton", "slime", "spider", "stray", "vex",
            "vindicator", "witch", "wither", "wither_skeleton", "zoglin", "zombie",
            "zombie_villager", "zombified_piglin"
        ]),
        "minecraft:husbandry/bred_all_animals" => Some(vec![
            "axolotl", "bee", "cat", "chicken", "cow", "donkey", "fox", "goat", "hoglin",
            "horse", "llama", "mooshroom", "mule", "ocelot", "panda", "pig", "rabbit",
            "sheep", "strider", "turtle", "wolf"
        ]),
        "minecraft:nether/explore_nether" => Some(vec![
            "basalt_deltas", "crimson_forest", "nether_wastes", "soul_sand_valley", "warped_forest"
        ]),
        _ => None, // Single-criterion or unknown
    }
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
    let mut cursor = std::io::Cursor::new(&bytes);
    let val: serde_json::Value = match quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::GzCompressed) {
        Ok((tag, _)) => serde_json::to_value(tag).unwrap_or(serde_json::Value::Null),
        Err(_) => {
            // Try uncompressed
            let mut cursor = std::io::Cursor::new(&bytes);
            match quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::Uncompressed) {
                 Ok((tag, _)) => serde_json::to_value(tag).unwrap_or(serde_json::Value::Null),
                 Err(_) => return Err("Failed to parse player NBT".to_string()),
            }
        }
    };

    parse_player_data(&val)
}

fn parse_player_data(nbt: &serde_json::Value) -> Result<PlayerData, String> {
    // If nbt is likely just the root compound (which it is from quartz),
    // Player data is usually at the top level for player.dat files (offline).
    // Or sometimes inside "Data" for older level.dat player structs?
    // Typical player.dat is root compound.
    
    let compound = nbt.as_object().ok_or("Root is not a compound")?;

    // Position "Pos" [x, y, z]
    let position = if let Some(serde_json::Value::Array(pos)) = compound.get("Pos") {
        if pos.len() >= 3 {
             let x = pos[0].as_f64().unwrap_or(0.0);
             let y = pos[1].as_f64().unwrap_or(0.0);
             let z = pos[2].as_f64().unwrap_or(0.0);
             PlayerPosition { x, y, z }
        } else { PlayerPosition { x: 0.0, y: 0.0, z: 0.0 } }
    } else { PlayerPosition { x: 0.0, y: 0.0, z: 0.0 } };

    // Dimension
    let dimension = if let Some(dim) = compound.get("Dimension") {
         match dim {
             serde_json::Value::String(s) => s.clone(), // modern "minecraft:overworld"
             serde_json::Value::Number(n) => {
                 match n.as_i64().unwrap_or(0) {
                     -1 => "minecraft:the_nether".to_string(),
                     0 => "minecraft:overworld".to_string(),
                     1 => "minecraft:the_end".to_string(),
                     _ => format!("dimension_{}", n),
                 }
             },
             _ => "unknown".to_string(),
         }
    } else { "unknown".to_string() };

    // Game Mode "playerGameType"
    let game_mode = if let Some(serde_json::Value::Number(gm)) = compound.get("playerGameType") {
         gm.as_i64().unwrap_or(0) as i32
    } else { 0 };
    
    // Health "Health"
    let health = if let Some(serde_json::Value::Number(h)) = compound.get("Health") {
        h.as_f64().unwrap_or(20.0) as f32
    } else { 20.0 };

    // Food Level "foodLevel"
    let food_level = if let Some(serde_json::Value::Number(f)) = compound.get("foodLevel") {
        f.as_i64().unwrap_or(20) as i32
    } else { 20 };

    // XP
    let xp_level = if let Some(serde_json::Value::Number(l)) = compound.get("XpLevel") {
        l.as_i64().unwrap_or(0) as i32
    } else { 0 };

    let xp_total = if let Some(serde_json::Value::Number(t)) = compound.get("XpTotal") {
        t.as_i64().unwrap_or(0) as i32
    } else { 0 };

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
