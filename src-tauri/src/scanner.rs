use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Cursor};
use std::path::Path;
use walkdir::WalkDir;
use flate2::read::GzDecoder;
use base64::{Engine as _, engine::general_purpose};
use zip::ZipArchive;
use rayon::prelude::*;
use std::sync::Mutex;
use tauri::Emitter;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WorldInfo {
    pub id: String,
    pub name: String,
    pub folder_name: String,
    pub path: String,
    pub version_id: String,
    pub last_played: i64,
    pub size: u64,
    pub icon: Option<String>,
    pub seed: Option<i64>,
    pub game_mode: String,
    pub platform: String,
    pub is_zip: bool,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct LevelDat {
    Data: LevelData,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct LevelData {
    LevelName: String,
    LastPlayed: i64,
    RandomSeed: Option<i64>,
    GameType: i32, // 0=Survival, 1=Creative, 2=Adventure, 3=Spectator
    Version: Option<VersionInfo>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct VersionInfo {
    Name: String,
}

#[derive(Clone, Serialize)]
struct ScanProgress {
    current_path: String,
    worlds_found: usize,
    items_processed: usize,
    total_items: usize,
    elapsed_seconds: f64,
}

// Generate UUIDv8 for a world
fn generate_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    
    // Simple UUIDv8-like format: timestamp + random
    format!("{:016x}{:016x}", timestamp, rand::random::<u64>())
}

// Read or create UUID for a world folder
fn get_or_create_world_id(world_path: &Path, is_zip: bool) -> String {
    if is_zip {
        // For zips, use fingerprint
        fingerprint_world(world_path)
    } else {
        // For regular folders, read or create .petalatlas-UUIDv8
        let uuid_file = world_path.join(".petalatlas-UUIDv8");
        
        if let Ok(existing_id) = fs::read_to_string(&uuid_file) {
            existing_id.trim().to_string()
        } else {
            // Generate new UUID and save it
            let new_id = generate_uuid();
            let _ = fs::write(&uuid_file, &new_id);
            new_id
        }
    }
}

// Create fingerprint for zip worlds
fn fingerprint_world(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let path_str = path.to_string_lossy();
    let mut hasher = DefaultHasher::new();
    path_str.hash(&mut hasher);
    
    let metadata = fs::metadata(path).ok();
    if let Some(meta) = metadata {
        meta.len().hash(&mut hasher);
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
    }
    
    format!("zip-{:x}", hasher.finish())
}

pub fn scan_for_worlds(
    root: &str, 
    max_depth: usize,
    scan_zips: bool,
    window: tauri::Window, 
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    skip_zip_flag: std::sync::Arc<std::sync::atomic::AtomicBool>
) -> Vec<WorldInfo> {
    let worlds = Mutex::new(Vec::new());
    
    // Expand ~ to home directory
    let expanded_root = if root.starts_with("~") {
        if let Ok(home) = std::env::var("HOME") {
            root.replacen("~", &home, 1)
        } else {
            root.to_string()
        }
    } else {
        root.to_string()
    };
    
    let checked_count = Mutex::new(0usize);
    
    // Configure WalkDir to skip hidden paths and known system directories
    let walker = WalkDir::new(&expanded_root)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            
            // Skip hidden files/folders (starting with dot) - includes .git, .vscode, etc.
            if e.file_name().to_string_lossy().starts_with('.') {
                return false;
            }
            
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                
                // Skip common build/dependency directories (these can be HUGE)
                let build_dirs = [
                    "node_modules", "target", "build", "dist", "out",
                    "__pycache__", "venv", "env", ".venv", 
                    "vendor", "pkg", "bin", "obj",
                ];
                if build_dirs.contains(&name_str.as_ref()) {
                    return false;
                }
                
                // Skip common cache/temp directories
                let cache_dirs = [
                    "cache", "Cache", "Caches", "tmp", "temp",
                    "Temp", "logs", "log",
                ];
                if cache_dirs.contains(&name_str.as_ref()) {
                    return false;
                }
                
                // Skip macOS app containers (can contain millions of files)
                if name_str.ends_with(".app") || name_str == "Containers" {
                    return false;
                }
                
                // Skip common system directories at root level
                if name_str == "System" || name_str == "dev" || name_str == "proc" 
                    || name_str == "sys" || name_str == "private" {
                    if path.parent().map(|p| p == Path::new("/")).unwrap_or(false) {
                        return false;
                    }
                }
                
                // Skip /Library at root, but allow ~/Library (where Minecraft worlds might be)
                if name_str == "Library" {
                    if path.parent().map(|p| p == Path::new("/")).unwrap_or(false) {
                        return false;
                    }
                }
            }
            true
        });
    
    // Collect all paths first with progress feedback
    let _ = window.emit("scan-phase", "Preparing scan...");
    println!("Collecting entries...");
    
    let mut entries = Vec::new();
    let mut collect_count = 0;
    
    for entry in walker.filter_map(|e| e.ok()) {
        let path_string = entry.path().to_string_lossy().to_string();
        entries.push(entry);
        collect_count += 1;
        
        // Emit progress during collection every 1000 items (optimized for millions of files)
        if collect_count % 1000 == 0 {
            let _ = window.emit("scan-progress", ScanProgress {
                current_path: format!("📂 {}", path_string),
                worlds_found: 0,
                items_processed: 0,
                total_items: collect_count,
                elapsed_seconds: 0.0,
            });
        }
    }
    
    let total_items = entries.len();
    println!("Found {} total items to scan", total_items);
    
    // Emit total count
    let _ = window.emit("scan-total", total_items);
    
    let start_time = std::time::Instant::now();
    
    // OPTIMIZED: Partition entries into dirs and zips, then process
    println!("Partitioning entries...");
    let _ = window.emit("scan-phase", "Scanning for worlds...");
    
    // Partition all entries into directories and zip files
    let mut dir_entries = Vec::new();
    let mut zip_entries = Vec::new();
    
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            dir_entries.push(entry);
        } else if scan_zips && path.is_file() && path.extension().map(|ext| ext == "zip").unwrap_or(false) {
            zip_entries.push(entry);
        }
    }
    
    let total_dirs = dir_entries.len();
    println!("Processing {} directories...", total_dirs);
    
    dir_entries.par_iter().for_each(|entry| {
        // Check if scan was cancelled
        if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        
        let path = entry.path();
        
        let mut count = checked_count.lock().unwrap();
        *count += 1;
        let current_count = *count;
        drop(count);
        
        // Emit progress every 500 items (optimized for performance)
        if current_count % 500 == 0 {
            let current_worlds = worlds.lock().unwrap().len();
            let elapsed = start_time.elapsed().as_secs_f64();
            let _ = window.emit("scan-progress", ScanProgress {
                current_path: path.to_string_lossy().to_string(),
                worlds_found: current_worlds,
                items_processed: current_count,
                total_items: total_dirs,
                elapsed_seconds: elapsed,
            });
        }
        
        // Check if it's a Java world
        if is_java_world(&path) {
            println!("Found Java world: {:?}", path);
            if let Some(world) = parse_java_world(&path) {
                worlds.lock().unwrap().push(world);
            }
        }
        // Check if it's a Bedrock world
        else if is_bedrock_world(&path) {
            println!("Found Bedrock world: {:?}", path);
            if let Some(world) = parse_bedrock_world(&path) {
                worlds.lock().unwrap().push(world);
            }
        }
    });
    
    // PASS 2: Scan zip files (slow, optional)
    if scan_zips && !zip_entries.is_empty() {
        println!("Pass 2: Scanning {} zip files in parallel...", zip_entries.len());
        let _ = window.emit("scan-phase", "Scanning zip files...");
        
        // Parallel scan of zip files
        zip_entries.par_iter().for_each(|entry| {
            // Check if scan was cancelled
            if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            
            let path = entry.path();
            
            // Reset skip flag for this zip
            skip_zip_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            
            // Emit specific event for zip scanning
            let _ = window.emit("scanning-zip", path.to_string_lossy().to_string());
            println!("Checking zip file: {:?}", path);
            
            if let Some(world) = parse_zip_world(&path, skip_zip_flag.clone()) {
                println!("Found world in zip: {:?}", path);
                worlds.lock().unwrap().push(world);
            }
        });
    }
    
    // Final progress update
    let final_worlds = worlds.lock().unwrap();
    let _ = window.emit("scan-complete", final_worlds.len());
    
    final_worlds.clone()
}

fn is_java_world(path: &Path) -> bool {
    // Must have level.dat
    if !path.join("level.dat").exists() {
        return false;
    }
    
    // Must have at least one of these directories
    path.join("region").exists() 
        || path.join("entities").exists() 
        || path.join("poi").exists()
}

fn is_bedrock_world(path: &Path) -> bool {
    // Must have level.dat
    if !path.join("level.dat").exists() {
        return false;
    }
    
    // Must have db/ directory
    let db_path = path.join("db");
    if !db_path.exists() || !db_path.is_dir() {
        return false;
    }
    
    // Check for at least one .ldb file in db/
    if let Ok(entries) = fs::read_dir(&db_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            if let Some(ext) = entry.path().extension() {
                if ext == "ldb" {
                    return true;
                }
            }
        }
    }
    
    false
}

pub fn parse_java_world(path: &Path) -> Option<WorldInfo> {
    let level_dat_path = path.join("level.dat");
    let file = fs::File::open(&level_dat_path).ok()?;
    let mut decoder = GzDecoder::new(file);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes).ok()?;

    let mut cursor = std::io::Cursor::new(&bytes);
    let (tag, _) = quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::Uncompressed).ok()?;
    let level_dat: LevelDat = serde_json::from_value(serde_json::to_value(tag).ok()?).ok()?;
    let data = level_dat.Data;

    let icon_path = path.join("icon.png");
    let icon = if icon_path.exists() {
        fs::read(icon_path).ok().map(|b| general_purpose::STANDARD.encode(b))
    } else {
        None
    };

    let size = calculate_dir_size(path);
    let version = data.Version.map(|v| v.Name).unwrap_or_else(|| "Unknown".to_string());
    let mode = match data.GameType {
        1 => "Creative",
        2 => "Adventure",
        3 => "Spectator",
        _ => "Survival",
    }.to_string();

    Some(WorldInfo {
        id: get_or_create_world_id(path, false),
        name: data.LevelName,
        folder_name: path.file_name()?.to_string_lossy().to_string(),
        path: path.to_string_lossy().to_string(),
        version_id: version,
        last_played: data.LastPlayed,
        size,
        icon,
        seed: data.RandomSeed,
        game_mode: mode,
        platform: "Java".to_string(),
        is_zip: false,
    })
}

fn parse_zip_world(path: &Path, skip_flag: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Option<WorldInfo> {
    let file = fs::File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    
    // Look for level.dat inside zip
    // Note: It might be in a subdirectory. For MVP, scanning root of zip or 1 level deep is good.
    // Let's iterate file names to find level.dat
    let mut level_dat_index = None;
    let mut icon_index = None;
    
    for i in 0..archive.len() {
        // Check if user skipped this zip
        if skip_flag.load(std::sync::atomic::Ordering::Relaxed) {
            println!("Zip skipped by user");
            return None;
        }
        
        let file = archive.by_index(i).ok()?;
        if file.name().ends_with("level.dat") {
            level_dat_index = Some(i);
        } else if file.name().ends_with("icon.png") {
            icon_index = Some(i);
        }
    }

    if let Some(idx) = level_dat_index {
        let mut file = archive.by_index(idx).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        drop(file); // Explicitly drop to release borrow
        
        // level.dat is GZipped
        let mut decoder = GzDecoder::new(Cursor::new(buffer));
        let mut bytes = Vec::new();
        decoder.read_to_end(&mut bytes).ok()?;
        
    let mut cursor = std::io::Cursor::new(&bytes);
    let (tag, _) = quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::Uncompressed).ok()?;
    let level_dat: LevelDat = serde_json::from_value(serde_json::to_value(tag).ok()?).ok()?;
    let data = level_dat.Data;

        let icon = if let Some(idx) = icon_index {
             let mut file = archive.by_index(idx).ok()?;
             let mut buf = Vec::new();
             file.read_to_end(&mut buf).ok()?;
             Some(general_purpose::STANDARD.encode(buf))
        } else {
            None
        };

        let mode = match data.GameType {
            1 => "Creative",
            2 => "Adventure",
            3 => "Spectator",
            _ => "Survival",
        }.to_string();
         let version = data.Version.map(|v| v.Name).unwrap_or_else(|| "Unknown".to_string());

        return Some(WorldInfo {
            id: get_or_create_world_id(path, true),
            name: data.LevelName,
            folder_name: path.file_name()?.to_string_lossy().to_string(),
            path: path.to_string_lossy().to_string(),
            version_id: version,
            last_played: data.LastPlayed,
            size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            icon,
            seed: data.RandomSeed,
            game_mode: mode,
            platform: "Java (Zip)".to_string(),
            is_zip: true,
        });
    }

    None
}

pub fn parse_bedrock_world(path: &Path) -> Option<WorldInfo> {
    // Try to read world name from levelname.txt if it exists
    // Otherwise use folder name
    let name = if let Ok(level_name) = fs::read_to_string(path.join("levelname.txt")) {
        level_name.trim().to_string()
    } else {
        path.file_name()?.to_string_lossy().to_string()
    };
    
    // Bedrock worlds use LevelDB, harder to parse level.dat
    // For MVP, we just detect structure and use basic info
    
    Some(WorldInfo {
        id: get_or_create_world_id(path, false),
        name,
        folder_name: path.file_name()?.to_string_lossy().to_string(),
        path: path.to_string_lossy().to_string(),
        version_id: "Bedrock".to_string(),
        last_played: 0, // Would need to parse level.dat for this
        size: calculate_dir_size(path),
        icon: None, // Bedrock icons are often world_icon.jpeg
        seed: None,
        game_mode: "Unknown".to_string(),
        platform: "Bedrock".to_string(),
        is_zip: false,
    })
}

fn calculate_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}
