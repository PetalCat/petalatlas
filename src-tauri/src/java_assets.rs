use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::Manager;
use zip::ZipArchive;
use base64::{Engine as _, engine::general_purpose};
use image::GenericImageView;

// ============================================
// Core structures for Java capabilities
// ============================================

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JavaCapabilities {
    pub has_local_java_data: bool,
    pub has_local_java_assets: bool,
    pub java_version: Option<String>,
    pub jar_path: Option<String>,
    pub minecraft_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExtractedAsset {
    pub id: String,
    pub asset_type: String,
    pub data_base64: Option<String>,  // For binary data (textures)
    pub json: Option<serde_json::Value>,  // For JSON data
    pub cached_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AdvancementDef {
    pub id: String,
    pub parent: Option<String>,
    pub frame: Option<String>,  // "task", "goal", "challenge"
    pub criteria: Vec<String>,
    pub rewards: Option<serde_json::Value>,
    pub hidden: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegistryData {
    pub items: Vec<String>,
    pub blocks: Vec<String>,
    pub entities: Vec<String>,
    pub tags: HashMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VersionManifest {
    pub id: String,
    pub version_type: String,  // "release", "snapshot"
    pub jar_path: Option<String>,
}

// ============================================
// Model & Blockstate Structures
// ============================================

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockState {
    pub variants: Option<HashMap<String, Variant>>,
    pub multipart: Option<Vec<Multipart>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Variant {
    Single(VariantInfo),
    List(Vec<VariantInfo>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VariantInfo {
    pub model: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub uvlock: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Multipart {
    pub apply: Variant,
    pub when: Option<serde_json::Value>, // We might not need full condition parsing logic for basic colors
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockModel {
    pub parent: Option<String>,
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<ModelElement>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub faces: HashMap<String, ModelFace>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelFace {
    pub texture: String,
    pub cullface: Option<String>,
    pub tintindex: Option<i32>,
}

// ============================================
// Helper functions
// ============================================

/// Get the default .minecraft directory based on OS
fn get_default_minecraft_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let path = dirs::home_dir().map(|h| h.join("Library/Application Support/minecraft"));
        println!("[DEBUG] macOS Minecraft Path: {:?}", path);
        path
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| PathBuf::from(appdata).join(".minecraft"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|h| h.join(".minecraft"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        None
    }
}

/// Find available Minecraft versions in the .minecraft folder
fn find_minecraft_versions(minecraft_dir: &Path) -> Vec<VersionManifest> {
    let versions_dir = minecraft_dir.join("versions");
    println!("[DEBUG] Searching for versions in: {:?}", versions_dir);
    let mut versions = Vec::new();
    
    if let Ok(entries) = fs::read_dir(&versions_dir) {
        println!("[DEBUG] Versions dir exists.");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let version_id = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                // Look for the version json to determine type
                let json_path = path.join(format!("{}.json", &version_id));
                let jar_path = path.join(format!("{}.jar", &version_id));
                
                let version_type = if json_path.exists() {
                    if let Ok(content) = fs::read_to_string(&json_path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            json["type"].as_str().unwrap_or("unknown").to_string()
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };
                
                if jar_path.exists() {
                    versions.push(VersionManifest {
                        id: version_id,
                        version_type,
                        jar_path: Some(jar_path.to_string_lossy().to_string()),
                    });
                }
            }
        }
    } else {
        println!("[DEBUG] Failed to read versions directory");
    }
    
    println!("[DEBUG] Found {} versions: {:?}", versions.len(), versions.iter().map(|v| &v.id).collect::<Vec<_>>());
    
    // Sort by version id (newest first, roughly)
    versions.sort_by(|a, b| b.id.cmp(&a.id));
    versions
}

/// Get the cache directory for Java assets
pub fn get_cache_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let cache_dir = app_data_dir.join("java_assets");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache dir: {}", e))?;
    
    Ok(cache_dir)
}

// ============================================
// Tauri Commands
// ============================================

/// Detect Java installation and available versions
#[tauri::command]
pub fn detect_java_installation() -> Result<JavaCapabilities, String> {
    let minecraft_dir = get_default_minecraft_dir();
    
    match &minecraft_dir {
        Some(dir) if dir.exists() => {
            let versions = find_minecraft_versions(dir);
            let latest_jar = versions.first().and_then(|v| v.jar_path.clone());
            let latest_version = versions.first().map(|v| v.id.clone());
            
            Ok(JavaCapabilities {
                has_local_java_data: !versions.is_empty(),
                has_local_java_assets: !versions.is_empty(),
                java_version: latest_version,
                jar_path: latest_jar,
                minecraft_dir: Some(dir.to_string_lossy().to_string()),
            })
        }
        _ => Ok(JavaCapabilities {
            has_local_java_data: false,
            has_local_java_assets: false,
            java_version: None,
            jar_path: None,
            minecraft_dir: None,
        })
    }
}

/// Get list of available Minecraft versions
#[tauri::command]
pub fn get_minecraft_versions() -> Result<Vec<VersionManifest>, String> {
    let minecraft_dir = get_default_minecraft_dir()
        .ok_or("Could not determine .minecraft directory")?;
    
    if !minecraft_dir.exists() {
        return Ok(Vec::new());
    }
    
    Ok(find_minecraft_versions(&minecraft_dir))
}

/// User explicitly selects a JAR file
#[tauri::command]
pub fn select_java_jar(path: String) -> Result<JavaCapabilities, String> {
    let jar_path = Path::new(&path);
    
    if !jar_path.exists() {
        return Err("JAR file does not exist".to_string());
    }
    
    // Validate it's a Minecraft JAR by checking for expected content
    let file = fs::File::open(jar_path)
        .map_err(|e| format!("Failed to open JAR: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR as ZIP: {}", e))?;
    
    // Check for minecraft assets directory
    let has_assets = archive.file_names()
        .any(|name| name.starts_with("assets/minecraft/"));
    
    let has_data = archive.file_names()
        .any(|name| name.starts_with("data/minecraft/"));
    
    // Try to extract version from version.json in the JAR
    let version = archive.by_name("version.json")
        .ok()
        .and_then(|mut file| {
            let mut content = String::new();
            file.read_to_string(&mut content).ok()?;
            serde_json::from_str::<serde_json::Value>(&content).ok()
        })
        .and_then(|json| json["name"].as_str().map(|s| s.to_string()));
    
    Ok(JavaCapabilities {
        has_local_java_data: has_data,
        has_local_java_assets: has_assets,
        java_version: version,
        jar_path: Some(path),
        minecraft_dir: None,
    })
}

/// Search a directory for client JAR files (2-3 levels deep)
#[tauri::command]
pub fn search_for_client_jars(path: String) -> Result<Vec<VersionManifest>, String> {
    let search_dir = Path::new(&path);
    
    if !search_dir.exists() {
        return Err("Directory does not exist".to_string());
    }
    
    let mut found_jars = Vec::new();
    
    // Search up to 3 levels deep for files matching *client.jar or *.jar in versions folders
    fn search_recursive(dir: &Path, depth: usize, results: &mut Vec<VersionManifest>) {
        if depth > 3 {
            return;
        }
        
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        // Match *client.jar or just *.jar files
                        if name.ends_with("client.jar") || 
                           (name.ends_with(".jar") && !name.contains("server")) {
                            // Try to extract version from the JAR
                            let version = extract_version_from_jar(&path);
                            let version_type = if name.contains("client") { 
                                "client".to_string() 
                            } else { 
                                "jar".to_string() 
                            };
                            
                            results.push(VersionManifest {
                                id: version.unwrap_or_else(|| name.to_string()),
                                version_type,
                                jar_path: Some(path.to_string_lossy().to_string()),
                            });
                        }
                    }
                } else if path.is_dir() {
                    search_recursive(&path, depth + 1, results);
                }
            }
        }
    }
    
    search_recursive(search_dir, 0, &mut found_jars);
    
    // Sort by ID (version) descending
    found_jars.sort_by(|a, b| b.id.cmp(&a.id));
    
    Ok(found_jars)
}

/// Helper to extract version from a JAR's version.json
fn extract_version_from_jar(jar_path: &Path) -> Option<String> {
    let file = fs::File::open(jar_path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    
    let mut version_file = archive.by_name("version.json").ok()?;
    let mut content = String::new();
    version_file.read_to_string(&mut content).ok()?;
    
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    json["name"].as_str().map(|s| s.to_string())
}

/// Extract a texture from the JAR and return as base64 PNG
#[tauri::command]
pub fn extract_texture(
    app_handle: tauri::AppHandle,
    jar_path: String, 
    texture_id: String
) -> Result<String, String> {
    // Check cache first
    let cache_dir = get_cache_dir(&app_handle)?;
    let filename = get_texture_cache_filename(&texture_id);
    let cache_path = cache_dir.join("textures").join(&filename);
    
    // Create textures cache dir
    fs::create_dir_all(cache_dir.join("textures"))
        .map_err(|e| format!("Failed to create textures cache dir: {}", e))?;
    
    // Return cached if exists
    if cache_path.exists() {
        let data = fs::read(&cache_path)
            .map_err(|e| format!("Failed to read cached texture: {}", e))?;
        return Ok(format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(&data)));
    }
    
    // Extract from JAR
    let file = fs::File::open(&jar_path)
        .map_err(|e| format!("Failed to open JAR: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR: {}", e))?;
    
    // Convert texture_id to path (e.g., "minecraft:item/diamond" -> "assets/minecraft/textures/item/diamond.png")
    let texture_path = texture_id_to_path(&texture_id);
    
    let mut texture_file = archive.by_name(&texture_path)
        .map_err(|e| format!("Texture not found '{}': {}", texture_path, e))?;
    
    let mut buffer = Vec::new();
    texture_file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read texture: {}", e))?;
    
    // Cache it
    fs::write(&cache_path, &buffer)
        .map_err(|e| format!("Failed to cache texture: {}", e))?;
    
    Ok(format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(&buffer)))
}

/// Extract language file from JAR
#[tauri::command]
pub fn extract_language(
    app_handle: tauri::AppHandle,
    jar_path: String, 
    locale: String
) -> Result<HashMap<String, String>, String> {
    let cache_dir = get_cache_dir(&app_handle)?;
    let lang_cache_dir = cache_dir.join("lang");
    fs::create_dir_all(&lang_cache_dir)
        .map_err(|e| format!("Failed to create lang cache dir: {}", e))?;
    
    let cache_path = lang_cache_dir.join(format!("{}.json", &locale));
    
    // Return cached if exists
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read cached lang: {}", e))?;
        let lang_map: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse cached lang: {}", e))?;
        return Ok(lang_map);
    }
    
    // Extract from JAR
    let file = fs::File::open(&jar_path)
        .map_err(|e| format!("Failed to open JAR: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR: {}", e))?;
    
    let lang_path = format!("assets/minecraft/lang/{}.json", locale);
    
    let mut lang_file = archive.by_name(&lang_path)
        .map_err(|e| format!("Language file not found '{}': {}", lang_path, e))?;
    
    let mut content = String::new();
    lang_file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read language file: {}", e))?;
    
    let lang_map: HashMap<String, String> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse language file: {}", e))?;
    
    // Cache it
    fs::write(&cache_path, &content)
        .map_err(|e| format!("Failed to cache language file: {}", e))?;
    
    Ok(lang_map)
}

/// Extract all advancement definitions from JAR
#[tauri::command]
pub fn extract_advancements(
    app_handle: tauri::AppHandle,
    jar_path: String
) -> Result<Vec<AdvancementDef>, String> {
    let cache_dir = get_cache_dir(&app_handle)?;
    let cache_path = cache_dir.join("advancements.json");
    
    // Return cached if exists
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read cached advancements: {}", e))?;
        let advancements: Vec<AdvancementDef> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse cached advancements: {}", e))?;
        return Ok(advancements);
    }
    
    // Extract from JAR
    let file = fs::File::open(&jar_path)
        .map_err(|e| format!("Failed to open JAR: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR: {}", e))?;
    
    let mut advancements = Vec::new();
    
    // Collect advancement file names first (can't borrow archive twice)
    let advancement_files: Vec<String> = archive.file_names()
        .filter(|name| name.starts_with("data/minecraft/advancements/") && name.ends_with(".json"))
        .map(|s| s.to_string())
        .collect();
    
    for name in advancement_files {
        if let Ok(mut file) = archive.by_name(&name) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Convert path to advancement ID
                    // e.g., "data/minecraft/advancements/adventure/kill_a_mob.json" -> "minecraft:adventure/kill_a_mob"
                    let id = name
                        .strip_prefix("data/minecraft/advancements/")
                        .and_then(|s| s.strip_suffix(".json"))
                        .map(|s| format!("minecraft:{}", s))
                        .unwrap_or_else(|| name.clone());
                    
                    let parent = json["parent"].as_str().map(|s| s.to_string());
                    let frame = json["display"]["frame"].as_str().map(|s| s.to_string());
                    let hidden = json["display"]["hidden"].as_bool().unwrap_or(false);
                    
                    let criteria: Vec<String> = json["criteria"].as_object()
                        .map(|obj| obj.keys().cloned().collect())
                        .unwrap_or_default();
                    
                    let rewards = json.get("rewards").cloned();
                    
                    advancements.push(AdvancementDef {
                        id,
                        parent,
                        frame,
                        criteria,
                        rewards,
                        hidden,
                    });
                }
            }
        }
    }
    
    // Cache it
    let json = serde_json::to_string_pretty(&advancements)
        .map_err(|e| format!("Failed to serialize advancements: {}", e))?;
    fs::write(&cache_path, &json)
        .map_err(|e| format!("Failed to cache advancements: {}", e))?;
    
    println!("Extracted and cached {} advancements", advancements.len());
    Ok(advancements)
}

/// Extract registry data (items, blocks, entities, tags) from JAR
#[tauri::command]
pub fn extract_registries(
    app_handle: tauri::AppHandle,
    jar_path: String
) -> Result<RegistryData, String> {
    let cache_dir = get_cache_dir(&app_handle)?;
    let cache_path = cache_dir.join("registries.json");
    
    // Return cached if exists
    if cache_path.exists() {
        let content = fs::read_to_string(&cache_path)
            .map_err(|e| format!("Failed to read cached registries: {}", e))?;
        let registries: RegistryData = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse cached registries: {}", e))?;
        return Ok(registries);
    }
    
    // Extract from JAR
    let file = fs::File::open(&jar_path)
        .map_err(|e| format!("Failed to open JAR: {}", e))?;
    
    let mut archive = ZipArchive::new(file)
        .map_err(|e| format!("Failed to read JAR: {}", e))?;
    
    let mut items = Vec::new();
    let mut blocks = Vec::new();
    let mut entities = Vec::new();
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    
    // Collect tag file names
    let tag_files: Vec<String> = archive.file_names()
        .filter(|name| name.starts_with("data/minecraft/tags/") && name.ends_with(".json"))
        .map(|s| s.to_string())
        .collect();
    
    for name in tag_files {
        if let Ok(mut file) = archive.by_name(&name) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Parse tag values
                    if let Some(values) = json["values"].as_array() {
                        let tag_id = name
                            .strip_prefix("data/minecraft/tags/")
                            .and_then(|s| s.strip_suffix(".json"))
                            .map(|s| format!("minecraft:{}", s))
                            .unwrap_or_else(|| name.clone());
                        
                        let entries: Vec<String> = values.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        
                        // Categorize based on path
                        if name.contains("/items/") {
                            for entry in &entries {
                                if !items.contains(entry) {
                                    items.push(entry.clone());
                                }
                            }
                        } else if name.contains("/blocks/") {
                            for entry in &entries {
                                if !blocks.contains(entry) {
                                    blocks.push(entry.clone());
                                }
                            }
                        } else if name.contains("/entity_types/") {
                            for entry in &entries {
                                if !entities.contains(entry) {
                                    entities.push(entry.clone());
                                }
                            }
                        }
                        
                        tags.insert(tag_id, entries);
                    }
                }
            }
        }
    }
    
    // Sort for consistent output
    items.sort();
    blocks.sort();
    entities.sort();
    
    let registries = RegistryData {
        items,
        blocks,
        entities,
        tags,
    };
    
    // Cache it
    let json = serde_json::to_string_pretty(&registries)
        .map_err(|e| format!("Failed to serialize registries: {}", e))?;
    fs::write(&cache_path, &json)
        .map_err(|e| format!("Failed to cache registries: {}", e))?;
    
    println!("Extracted and cached registries");
    Ok(registries)
}

use crate::biome_types::ExtractedBiome;

#[tauri::command]
pub async fn extract_biome_data(
    app_handle: tauri::AppHandle,
    override_path: Option<String>
) -> Result<Vec<ExtractedBiome>, String> {
    println!("Starting biome extraction...");
    // 1. Get JAR
    let jar_path = if let Some(path) = override_path {
        path
    } else {
        detect_java_installation()?.jar_path.ok_or("No Minecraft JAR found")?
    };
    println!("Opening JAR for biomes: {}", jar_path);

    let cache_dir = get_cache_dir(&app_handle)?;
    let colormap_dir = cache_dir.join("colormaps");
    fs::create_dir_all(&colormap_dir).map_err(|e| e.to_string())?;

    // Perform heavy ZIP operations in a blocking thread
    let biomes = tauri::async_runtime::spawn_blocking(move || {
        let file = fs::File::open(&jar_path).map_err(|e| e.to_string())?;
        let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
        let mut biomes = Vec::new();

        // 2. Extract Colormaps
        let colormaps = ["grass.png", "foliage.png"];
        for name in colormaps {
             let jar_path = format!("assets/minecraft/textures/colormap/{}", name);
             if let Ok(mut file) = archive.by_name(&jar_path) {
                 let mut buf = Vec::new();
                 if file.read_to_end(&mut buf).is_ok() {
                     let _ = fs::write(colormap_dir.join(name), buf);
                     println!("Extracted colormap: {}", name);
                 }
             }
        }
        
        // 3. Extract Biome Definitions
        println!("Scanning for biome definitions...");
        // Scan for data/minecraft/worldgen/biome/*.json
        let file_names: Vec<String> = archive.file_names()
            .filter(|n| n.starts_with("data/minecraft/worldgen/biome/") && n.ends_with(".json"))
            .map(|n| n.to_string())
            .collect();

        println!("Found {} potential biome files", file_names.len());

        for path in file_names {
            if let Ok(mut file) = archive.by_name(&path) {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_ok() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        let id = Path::new(&path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        let temp = json["temperature"].as_f64().unwrap_or(0.5) as f32;
                        let downfall = json["downfall"].as_f64().unwrap_or(0.5) as f32;
                        let water_color = json["effects"]["water_color"].as_i64().map(|c| c as i32);
                        let grass_color = json["effects"]["grass_color"].as_i64().map(|c| c as i32);
                        let foliage_color = json["effects"]["foliage_color"].as_i64().map(|c| c as i32);
                        
                        biomes.push(ExtractedBiome {
                            id: format!("minecraft:{}", id),
                            temperature: temp, 
                            downfall,
                            water_color,
                            grass_color,
                            foliage_color, // Add overrides
                        });
                    }
                }
            }
        }
        Ok::<Vec<ExtractedBiome>, String>(biomes)
    }).await.map_err(|e| e.to_string())??;
    
    println!("Extracted {} biomes", biomes.len());
    
    // Save biomes.json cache
    let assets_dir = app_handle.path().app_data_dir().unwrap().join("data");
    let _ = fs::create_dir_all(&assets_dir);
    let _ = fs::write(
        assets_dir.join("biomes.json"), 
        serde_json::to_string_pretty(&biomes).unwrap_or_default()
    );

    Ok(biomes)
}


/// Extract all block textures from JAR and calculate average colors
// ============================================
// Model-Driven Extraction Logic
// ============================================

// Assumes global imports are sufficient


// Assuming these structs are defined elsewhere in the project, e.g.:
// #[derive(Debug, serde::Deserialize)]
// struct BlockModel {
//     parent: Option<String>,
//     textures: Option<HashMap<String, String>>,
// }
//
// #[derive(Debug, serde::Deserialize)]
// #[serde(untagged)]
// enum Variant {
//     Single(ModelInfo),
//     List(Vec<ModelInfo>),
// }
//
// #[derive(Debug, serde::Deserialize)]
// struct ModelInfo {
//     model: String,
//     #[serde(default)]
//     x: u32,
//     #[serde(default)]
//     y: u32,
//     #[serde(default)]
//     uvlock: bool,
//     #[serde(default)]
//     weight: u32,
// }
//
// #[derive(Debug, serde::Deserialize)]
// struct BlockState {
//     variants: Option<HashMap<String, Variant>>,
//     multipart: Option<Vec<MultipartEntry>>,
// }
//
// #[derive(Debug, serde::Deserialize)]
// struct MultipartEntry {
//     #[serde(default)]
//     when: Option<serde_json::Value>, // Complex conditions, we'll ignore for simplicity
//     apply: Variant,
// }


fn load_json<T: serde::de::DeserializeOwned>(archive: &mut ZipArchive<fs::File>, path: &str) -> Option<T> {
    if let Ok(mut file) = archive.by_name(path) {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            // Remove comments if any (standard JSON spec doesn't allow specific comments but Minecraft often uses standard JSON)
            // But we can just try parsing.
            return serde_json::from_str(&content).ok();
        }
    }
    None
}

/// Recursively resolves the texture map for a model by following its parent chain.
fn resolve_model_textures(
    archive: &mut ZipArchive<fs::File>, 
    model_name: &str, 
    visited: &mut HashSet<String> // Prevention for infinite loops
) -> HashMap<String, String> {
    if visited.contains(model_name) {
        return HashMap::new();
    }
    visited.insert(model_name.to_string());

    // Path correction: "minecraft:block/stone" -> "assets/minecraft/models/block/stone.json"
    // Usually references are like "block/stone" or "minecraft:block/stone"
    let clean_name = model_name.strip_prefix("minecraft:").unwrap_or(model_name);
    let path = format!("assets/minecraft/models/{}.json", clean_name);

    let mut textures = HashMap::new();

    if let Some(model) = load_json::<BlockModel>(archive, &path) {
        // 1. Load Parent's textures first (so children override them)
        if let Some(parent) = model.parent {
            let parent_textures = resolve_model_textures(archive, &parent, visited);
            textures.extend(parent_textures);
        }

        // 2. Apply this model's textures
        if let Some(local_textures) = model.textures {
            textures.extend(local_textures);
        }
    }

    textures
}

/// Resolves a texture reference like "#side" into a concrete "block/stone"
fn resolve_var(var: &str, textures: &HashMap<String, String>) -> Option<String> {
    if var.starts_with('#') {
        let key = &var[1..];
        if let Some(val) = textures.get(key) {
            return resolve_var(val, textures); // Recursive resolution
        }
        return None;
    }
    Some(var.to_string())
}

fn get_model_from_variant(v: &Variant) -> String {
    match v {
        Variant::Single(info) => info.model.clone(),
        Variant::List(list) => list.first().map(|i| i.model.clone()).unwrap_or_default(),
    }
}

/// Recursively resolves elements from model hierarchy
fn resolve_model_elements(
    archive: &mut ZipArchive<fs::File>, 
    model_name: &str
) -> Option<Vec<ModelElement>> {
    let clean_name = model_name.strip_prefix("minecraft:").unwrap_or(model_name);
    let path = format!("assets/minecraft/models/{}.json", clean_name);
    
    if let Some(model) = load_json::<BlockModel>(archive, &path) {
        if let Some(elements) = model.elements {
            return Some(elements);
        } else if let Some(parent) = model.parent {
            return resolve_model_elements(archive, &parent);
        }
    }
    None
}

/// Core extraction logic, independent of Tauri
pub fn extract_colors_from_jar(
    jar_path: &str,
    write_cache_dir: Option<&PathBuf>
) -> Result<(HashMap<String, [u8; 3]>, HashMap<String, String>), String> {
    println!("Opening JAR at: {}", jar_path);
    let file = fs::File::open(jar_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    // 1. Extract ALL textures first into a cache: Name -> AvgColor AND Name -> Buffer
    // We only care about block textures: assets/minecraft/textures/block/*.png
    println!("Step 1: Extracting raw textures...");
    let mut raw_texture_colors: HashMap<String, [u8; 3]> = HashMap::new();
    let mut raw_texture_buffers: HashMap<String, Vec<u8>> = HashMap::new();
    
    // We iterate by index to extract valid block textures
    for i in 0..archive.len() {
        let file_res = archive.by_index(i);
        if let Ok(mut file) = file_res {
            let path = file.name().to_string();
            if path.starts_with("assets/minecraft/textures/block/") && path.ends_with(".png") {
                // "assets/minecraft/textures/block/grass_block_top.png" -> "minecraft:block/grass_block_top"
                let bare_name = Path::new(&path).file_stem().unwrap().to_str().unwrap();
                let texture_key = format!("minecraft:block/{}", bare_name);

                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    if let Ok(img) = image::load_from_memory(&buf) {
                        // Store Buffer
                        raw_texture_buffers.insert(texture_key.clone(), buf.clone());

                        let (width, height) = img.dimensions();
                        let mut r: u64 = 0;
                        let mut g: u64 = 0;
                        let mut b: u64 = 0;
                        let mut count: u64 = 0;
                        for x in 0..width {
                            for y in 0..height {
                                let p = img.get_pixel(x, y);
                                if p[3] > 10 { // Skip transparent
                                    r += p[0] as u64;
                                    g += p[1] as u64;
                                    b += p[2] as u64;
                                    count += 1;
                                }
                            }
                        }
                        if count > 0 {
                            raw_texture_colors.insert(texture_key, [(r/count) as u8, (g/count) as u8, (b/count) as u8]);
                        }
                    }
                }
            }
        }
    }
    println!("Cached {} textures.", raw_texture_colors.len());

    // 2. Iterate Blockstates to find blocks and their models
    println!("Step 2: Processing blockstates...");
    let mut block_colors: HashMap<String, [u8; 3]> = HashMap::new();
    let mut block_textures: HashMap<String, String> = HashMap::new();

    // Setup textures cache dir if needed
    if let Some(dir) = write_cache_dir {
        let tex_dir = dir.join("textures");
        let _ = fs::create_dir_all(&tex_dir);
    }

    // List all blockstates
    let mut blockstate_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
             let name = file.name().to_string();
             if name.starts_with("assets/minecraft/blockstates/") && name.ends_with(".json") {
                 blockstate_names.push(name);
             }
        }
    }

    println!("Found {} blockstates", blockstate_names.len());

    for bs_path in blockstate_names {
        // "assets/minecraft/blockstates/grass_block.json" -> "minecraft:grass_block"
        let bare_name = Path::new(&bs_path).file_stem().unwrap().to_str().unwrap();
        let block_id = format!("minecraft:{}", bare_name);

        if let Some(bs) = load_json::<BlockState>(&mut archive, &bs_path) {
            // Pick best variant
            let mut best_model: Option<String> = None;

            if let Some(variants) = bs.variants {
                // Priority: "" -> "normal" -> "axis=y" -> keys start with "facing=north" -> first
                if let Some(v) = variants.get("") {
                     best_model = Some(get_model_from_variant(v));
                } else if let Some(v) = variants.get("normal") {
                     best_model = Some(get_model_from_variant(v));
                } else if let Some(v) = variants.get("axis=y") {
                     best_model = Some(get_model_from_variant(v));
                } else {
                    // Search keys
                    for (k, v) in &variants {
                        if k.contains("axis=y") || k.contains("facing=north") {
                            best_model = Some(get_model_from_variant(v));
                            break;
                        }
                    }
                    // Fallback: first
                    if best_model.is_none() {
                         if let Some(v) = variants.values().next() {
                             best_model = Some(get_model_from_variant(v));
                         }
                    }
                }
            } else if let Some(multiparts) = bs.multipart {
                 if let Some(part) = multiparts.first() {
                     best_model = Some(get_model_from_variant(&part.apply));
                 }
            }

            if let Some(model_name) = best_model {
                // Resolve Texture
                let mut visited = HashSet::new();
                let texture_map = resolve_model_textures(&mut archive, &model_name, &mut visited);

                // Now find the "top" face
                let mut resolved_texture: Option<String> = None;
                
                // Try semantic variables first (fast path)
                let priorities = vec!["top", "up", "plant", "cross", "texture", "all", "layer0", "particle"];
                for p in &priorities {
                     if let Some(val) = texture_map.get(*p) {
                         resolved_texture = resolve_var(val, &texture_map);
                         if resolved_texture.is_some() { break; }
                     }
                }

                // If semantic variables failed, we must look at elements (faces)
                if resolved_texture.is_none() {
                     let elements = resolve_model_elements(&mut archive, &model_name);
                     if let Some(elems) = elements {
                         for elem in elems {
                             if let Some(face) = elem.faces.get("up") {
                                 let raw_tex = &face.texture; 
                                 resolved_texture = resolve_var(raw_tex, &texture_map);
                                 if resolved_texture.is_some() { break; }
                             }
                             if resolved_texture.is_none() {
                                 if let Some(face) = elem.faces.get("down") {
                                     let raw_tex = &face.texture; 
                                     resolved_texture = resolve_var(raw_tex, &texture_map);
                                     if resolved_texture.is_some() { break; }
                                 }
                             }
                             if resolved_texture.is_none() {
                                 if let Some(face) = elem.faces.values().next() {
                                      let raw_tex = &face.texture; 
                                      resolved_texture = resolve_var(raw_tex, &texture_map);
                                      if resolved_texture.is_some() { break; }
                                 }
                             }
                         }
                     }
                }

                if let Some(final_tex_id) = resolved_texture {
                     let clean_tex_id = if !final_tex_id.contains(':') {
                         format!("minecraft:{}", final_tex_id)
                     } else {
                         final_tex_id.clone()
                     };
                     
                     // Store Mapping
                     block_textures.insert(block_id.clone(), clean_tex_id.clone());

                     // Store Average Color if known
                     if let Some(c) = raw_texture_colors.get(&clean_tex_id) {
                         // Tints
                         let mut final_color = *c;
                         let bare_id = block_id.strip_prefix("minecraft:").unwrap_or(&block_id);
                         let _apply_grass_tint = bare_id.contains("grass") || bare_id.contains("fern") || bare_id == "sugar_cane" || bare_id.contains("leaves") || bare_id == "vine";
                         if bare_id == "grass_block" || bare_id == "grass" || bare_id == "fern" || bare_id == "short_grass" || bare_id == "tall_grass" {
                             final_color[0] = (final_color[0] as u16 * 145 / 255) as u8;
                             final_color[1] = (final_color[1] as u16 * 189 / 255) as u8;
                             final_color[2] = (final_color[2] as u16 * 89 / 255) as u8;
                         } else if bare_id.contains("spruce") || bare_id.contains("pine") {
                              final_color[0] = (final_color[0] as u16 * 97 / 255) as u8;
                              final_color[1] = (final_color[1] as u16 * 153 / 255) as u8;
                              final_color[2] = (final_color[2] as u16 * 97 / 255) as u8;
                         } else if bare_id.contains("birch") {
                              final_color[0] = (final_color[0] as u16 * 128 / 255) as u8;
                              final_color[1] = (final_color[1] as u16 * 167 / 255) as u8;
                              final_color[2] = (final_color[2] as u16 * 85 / 255) as u8;
                         } else if bare_id == "water" || bare_id == "water_flow" {
                              final_color[0] = (final_color[0] as u16 * 63 / 255) as u8;
                              final_color[1] = (final_color[1] as u16 * 118 / 255) as u8;
                              final_color[2] = (final_color[2] as u16 * 228 / 255) as u8;
                         } else if bare_id.contains("leaves") {
                              final_color[0] = (final_color[0] as u16 * 120 / 255) as u8;
                              final_color[1] = (final_color[1] as u16 * 200 / 255) as u8;
                              final_color[2] = (final_color[2] as u16 * 100 / 255) as u8;
                         }

                         block_colors.insert(block_id.clone(), final_color);
                     } else {
                         // Texture extraction might have failed or it's missing (e.g. lava sometimes)
                         // println!("Texture not found in raw dump: {}", clean_tex_id);
                     }
                     
                     // Write Texture to Cache
                     if let Some(dir) = write_cache_dir {
                        if let Some(buf) = raw_texture_buffers.get(&clean_tex_id) {
                            let filename = get_texture_cache_filename(&clean_tex_id);
                            let path = dir.join("textures").join(&filename);
                            if !path.exists() {
                                let _ = fs::write(path, buf);
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("Extracted colors for {} blocks", block_colors.len());
    Ok((block_colors, block_textures))
}

#[tauri::command]
pub fn extract_all_block_colors(app_handle: tauri::AppHandle, override_path: Option<String>) -> Result<HashMap<String, [u8; 3]>, String> {
    // 1. Determine JAR path
    let jar_path = if let Some(path) = override_path {
        println!("[DEBUG] Using override JAR path: {}", path);
        path
    } else {
        // Fallback to auto-detection
        println!("[DEBUG] Auto-detecting JAR path...");
        let caps = detect_java_installation()?;
        caps.jar_path.ok_or("No Minecraft JAR found")?
    };
    
    // We want to write textures to the cache
    let cache_dir = get_cache_dir(&app_handle).ok(); // Option<PathBuf>
    
    let (block_colors, block_textures) = extract_colors_from_jar(&jar_path, cache_dir.as_ref())?;
    
    // Save to disk
    let data_dir = app_handle.path().app_data_dir().map_err(|e| e.to_string())?;
    let assets_dir = data_dir.join("data");
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    
    // Save Colors
    let json_path = assets_dir.join("block_colors.json");
    let file = fs::File::create(json_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, &block_colors).map_err(|e| e.to_string())?;

    // Save Textures Map for Renderer
    let tex_json_path = assets_dir.join("block_textures.json");
    let tex_file = fs::File::create(&tex_json_path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(tex_file, &block_textures).map_err(|e| e.to_string())?;

    Ok(block_colors)
}

/// Clear all cached Java assets
#[tauri::command]
pub fn clear_asset_cache(app_handle: tauri::AppHandle) -> Result<(), String> {
    let cache_dir = get_cache_dir(&app_handle)?;
    
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to clear cache: {}", e))?;
    }
    
    println!("Cleared Java asset cache");
    Ok(())
}

/// Get cached asset if available
#[tauri::command]
pub fn get_cached_asset(
    app_handle: tauri::AppHandle,
    asset_id: String, 
    asset_type: String
) -> Result<Option<ExtractedAsset>, String> {
    let cache_dir = get_cache_dir(&app_handle)?;
    
    let cache_path = match asset_type.as_str() {
        "texture" => {
            let filename = get_texture_cache_filename(&asset_id);
            cache_dir.join("textures").join(&filename)
        }
        "lang" => cache_dir.join("lang").join(format!("{}.json", asset_id)),
        "advancements" => cache_dir.join("advancements.json"),
        "registries" => cache_dir.join("registries.json"),
        _ => return Err(format!("Unknown asset type: {}", asset_type)),
    };
    
    if !cache_path.exists() {
        return Ok(None);
    }
    
    let metadata = fs::metadata(&cache_path)
        .map_err(|e| format!("Failed to get cache metadata: {}", e))?;
    
    let cached_at = metadata.modified()
        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
        .unwrap_or(0);
    
    match asset_type.as_str() {
        "texture" => {
            let data = fs::read(&cache_path)
                .map_err(|e| format!("Failed to read cached texture: {}", e))?;
            Ok(Some(ExtractedAsset {
                id: asset_id,
                asset_type,
                data_base64: Some(general_purpose::STANDARD.encode(&data)),
                json: None,
                cached_at,
            }))
        }
        _ => {
            let content = fs::read_to_string(&cache_path)
                .map_err(|e| format!("Failed to read cached asset: {}", e))?;
            let json: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse cached asset: {}", e))?;
            Ok(Some(ExtractedAsset {
                id: asset_id,
                asset_type,
                data_base64: None,
                json: Some(json),
                cached_at,
            }))
        }
    }
}

// ============================================
// Utility functions
// ============================================

/// Convert a texture ID to a JAR path
pub fn texture_id_to_path(texture_id: &str) -> String {
    // Handle various formats:
    // "minecraft:item/diamond" -> "assets/minecraft/textures/item/diamond.png"
    // "minecraft:textures/item/diamond" -> "assets/minecraft/textures/item/diamond.png"
    // "item/diamond" -> "assets/minecraft/textures/item/diamond.png"
    
    let id = texture_id.strip_prefix("minecraft:").unwrap_or(texture_id);
    let path = if id.starts_with("textures/") {
        id.to_string()
    } else {
        format!("textures/{}", id)
    };
    
    let path = if path.ends_with(".png") {
        path
    } else {
        format!("{}.png", path)
    };
    
    format!("assets/minecraft/{}", path)
}



/// Sanitizes a texture ID to be a valid filename (deterministic)
pub fn get_texture_cache_filename(id: &str) -> String {
    let sanitized = id.replace(':', "_").replace('/', "_").replace('\\', "_");
    format!("{}.png", sanitized)
}
