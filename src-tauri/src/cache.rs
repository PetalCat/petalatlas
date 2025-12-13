use crate::scanner::WorldInfo;
use std::fs;
use std::path::Path;
use tauri::Manager;

#[tauri::command]
pub fn save_worlds_cache(app_handle: tauri::AppHandle, worlds: Vec<WorldInfo>) -> Result<(), String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    // Create directory if it doesn't exist
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data dir: {}", e))?;
    
    let cache_path = app_data_dir.join("worlds_cache.json");
    let json = serde_json::to_string_pretty(&worlds)
        .map_err(|e| format!("Failed to serialize worlds: {}", e))?;
    
    fs::write(cache_path, json)
        .map_err(|e| format!("Failed to write cache file: {}", e))?;
    
    println!("Saved {} worlds to cache", worlds.len());
    Ok(())
}

#[tauri::command]
pub fn load_worlds_cache(app_handle: tauri::AppHandle) -> Result<Vec<WorldInfo>, String> {
    let app_data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let cache_path = app_data_dir.join("worlds_cache.json");
    
    if !cache_path.exists() {
        println!("No cache file found");
        return Ok(Vec::new());
    }
    
    let json = fs::read_to_string(&cache_path)
        .map_err(|e| format!("Failed to read cache file: {}", e))?;
    
    let worlds: Vec<WorldInfo> = serde_json::from_str(&json)
        .map_err(|e| format!("Failed to deserialize worlds: {}", e))?;
    
    println!("Loaded {} worlds from cache", worlds.len());
    Ok(worlds)
}

#[tauri::command]
pub fn update_world_in_cache(app_handle: tauri::AppHandle, world_path: String) -> Result<WorldInfo, String> {
    // Load current cache
    let mut worlds = load_worlds_cache(app_handle.clone())?;
    
    // Rescan just this world
    let path = Path::new(&world_path);
    let updated_world = if path.join("level.dat").exists() && path.join("region").exists() {
        crate::scanner::parse_java_world(path)
            .ok_or("Failed to parse Java world".to_string())?
    } else if path.join("level.dat").exists() && path.join("db").exists() {
        crate::scanner::parse_bedrock_world(path)
            .ok_or("Failed to parse Bedrock world".to_string())?
    } else {
        return Err("Not a valid Minecraft world".to_string());
    };
    
    // Find and replace the world in cache
    if let Some(pos) = worlds.iter().position(|w| w.id == updated_world.id) {
        worlds[pos] = updated_world.clone();
    } else {
        // If not found, add it
        worlds.push(updated_world.clone());
    }
    
    // Save updated cache
    save_worlds_cache(app_handle, worlds)?;
    
    Ok(updated_world)
}
