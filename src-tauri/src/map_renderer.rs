use std::fs::{self, File};
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::{Arc, RwLock};
use tauri::Manager;
// use std::sync::atomic::{AtomicBool, Ordering};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rayon::prelude::*;
use flate2::read::{GzDecoder, ZlibDecoder};
use fastnbt::{ByteArray, LongArray};

use image::{GenericImageView, imageops::FilterType};

use lazy_static::lazy_static;

lazy_static! {
    static ref BLOCK_COLORS: HashMap<String, [u8; 3]> = {
        let mut m = HashMap::new();
        let content = include_str!("../data/block_colors.json");
        if let Ok(json) = serde_json::from_str::<HashMap<String, [u8; 3]>>(content) {
            m = json;
        }
        m
    };
    
    // Cache for texture colors: BlockName -> [r,g,b,a]
    static ref TEXTURE_COLOR_CACHE: RwLock<HashMap<String, [u8; 4]>> = RwLock::new(HashMap::new());

    // Cache for resized texture pixels: (BlockName, Size) -> Vec<u8> (RGBA)
    static ref RESIZED_TEXTURE_CACHE: RwLock<HashMap<(String, u32), Vec<u8>>> = RwLock::new(HashMap::new());

    // Texture Mapping: BlockName -> TextureID (e.g. "minecraft:stone" -> "minecraft:block/stone")
    static ref BLOCK_TEXTURES_MAP: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
    static ref MAPS_LOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    
    // Biome Cache
    static ref COLORMAP_GRASS: RwLock<Option<Vec<u8>>> = RwLock::new(None);
    static ref COLORMAP_FOLIAGE: RwLock<Option<Vec<u8>>> = RwLock::new(None);
    static ref BIOME_PARAMS: RwLock<HashMap<String, crate::biome_types::ExtractedBiome>> = RwLock::new(HashMap::new());
    static ref BIOMES_LOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
}

fn load_colormap(app_handle: &tauri::AppHandle, name: &str) -> Option<Vec<u8>> {
    if let Ok(cache_dir) = crate::java_assets::get_cache_dir(app_handle) {
        let path = cache_dir.join("colormaps").join(name);
        if path.exists() {
             if let Ok(img) = image::open(path) {
                 return Some(img.to_rgba8().into_raw());
             }
        }
    }
    None
}

fn ensure_biomes_loaded(app_handle: &tauri::AppHandle) {
    if BIOMES_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    // Load Colormaps
    *COLORMAP_GRASS.write().unwrap() = load_colormap(app_handle, "grass.png");
    *COLORMAP_FOLIAGE.write().unwrap() = load_colormap(app_handle, "foliage.png");

    // Load Biome Params (must match extract_biome_data write location)
    if let Ok(data_dir) = app_handle.path().app_data_dir() {
         let path = data_dir.join("data").join("biomes.json");
         if path.exists() {
             if let Ok(content) = fs::read_to_string(path) {
                 if let Ok(biomes) = serde_json::from_str::<Vec<crate::biome_types::ExtractedBiome>>(&content) {
                     let mut map = HashMap::new();
                     for b in biomes {
                         map.insert(b.id.clone(), b);
                     }
                     *BIOME_PARAMS.write().unwrap() = map;
                 }
             }
         }
    }

    let _ok = !BIOME_PARAMS.read().unwrap().is_empty();
    BIOMES_LOADED.store(true, std::sync::atomic::Ordering::Relaxed);
}

use std::sync::Mutex;
use std::collections::HashSet;

lazy_static! {
    static ref MISSING_BIOMES_LOGGED: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
}

fn log_missing_biome(id: &str) {
    let mut set = MISSING_BIOMES_LOGGED.lock().unwrap();
    if set.insert(id.to_string()) {
        println!("[MISSING BIOME] Could not find parameters for '{}'", id);
    }
}

fn sample_colormap(colormap: &[u8], heat: f32, dampness: f32) -> [u8; 3] {
    let x = (heat.clamp(0.0, 1.0) * 255.0) as u32;
    let y = ((1.0 - dampness.clamp(0.0, 1.0)) * 255.0) as u32; // Inverted Y-axis logic typically

    // Colormaps are 256x256 RGBA (usually triangle shape bottom-left to top-right)
    let idx = ((y * 256 + x) * 4) as usize;
    if idx + 2 < colormap.len() {
        [colormap[idx], colormap[idx+1], colormap[idx+2]]
    } else {
        [255, 255, 255] // Default white
    }
}

#[derive(Serialize, Clone)]
pub struct RegionCoords {
    pub x: i32,
    pub z: i32,
}

#[derive(Deserialize, Debug)]
pub struct ChunkRoot {
    #[serde(alias = "Level")]
    pub level: Option<ChunkLevel>,
    #[serde(alias = "sections")]
    pub sections: Option<Vec<Section>>,
    #[serde(alias = "biomes")]
    pub biomes: Option<BiomeData>, // 1.18+ Chunk-level biomes
    #[serde(alias = "DataVersion")]
    pub data_version: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct ChunkLevel {
    #[serde(alias = "Sections")]
    pub sections: Option<Vec<Section>>,
    #[serde(alias = "Biomes")]
    pub biomes_256: Option<ByteArray>, // Pre-1.15 Legacy Biomes
}

#[derive(Deserialize, Debug)]
struct Section {
    #[serde(alias = "Y")]
    y: i8,
    // 1.16+ Container
    #[serde(alias = "block_states")]
    block_states_container: Option<BlockStates>,
    
    // 1.13-1.15 Legacy
    #[serde(alias = "BlockStates")]
    legacy_block_states: Option<LongArray>,
    
    // Legacy Palette (sibling of BlockStates)
    #[serde(alias = "Palette")] 
    palette: Option<Vec<PaletteEntry>>,
    
    #[serde(alias = "biomes")]
    biomes: Option<BiomeData>,
    
    // Pre-1.13 Legacy
    #[serde(alias = "Blocks")]
    legacy_blocks_u8: Option<ByteArray>,
    #[serde(alias = "Data")]
    legacy_data_u8: Option<ByteArray>,
}

#[derive(Deserialize, Debug)]
struct BlockStates {
    #[serde(alias = "palette")]
    palette: Option<Vec<PaletteEntry>>,
    #[serde(alias = "data")]
    data: Option<LongArray>,
}

#[derive(Deserialize, Debug)]
struct BiomeData {
    #[serde(alias = "palette")]
    palette: Vec<String>,
    #[serde(alias = "data")]
    data: Option<LongArray>,
}

#[derive(Deserialize, Debug)]
struct PaletteEntry {
    #[serde(alias = "Name")]
    name: String,
}

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

fn normalize_biome_id(id: &str) -> String {
    if id.contains(':') { id.to_string() } else { format!("minecraft:{}", id) }
}

fn quart_index_4x4x4(qx: usize, qz: usize, qy: usize) -> usize {
    // 4x4x4 ordering: y * 16 + z * 4 + x
    qy * 16 + qz * 4 + qx
}

fn resolve_biome_id_any(
    root: &ChunkRoot,
    sections_all: &[&Section],
    section: &Section,
    bx: usize,
    bz: usize,
    by: usize
) -> String {
    // Common quart coords
    let qx = bx / 4;
    let qz = bz / 4;

    // Global Y for this block
    let global_y = section.y as i32 * 16 + by as i32;

    // ---- 1) 1.18+ chunk-level paletted container ----
    if let Some(biomes) = &root.biomes {
        if !biomes.palette.is_empty() {
            if let Some(data) = &biomes.data {
                // Figure out the chunk's min_y from present sections
                let min_sec_y = sections_all.iter().map(|s| s.y as i32).min().unwrap_or(section.y as i32);
                let min_y = min_sec_y * 16;

                // Total vertical quart count from the section span we actually have
                let max_sec_y = sections_all.iter().map(|s| s.y as i32).max().unwrap_or(section.y as i32);
                let total_blocks_y = (max_sec_y - min_sec_y + 1).max(1) * 16;
                let total_quart_y = (total_blocks_y as usize) / 4;

                let qy = (((global_y - min_y) / 4).max(0) as usize).min(total_quart_y.saturating_sub(1));
                // For 1.18+ linear indices: usually y * (sizeX * sizeZ) + z * sizeX + x??
                // Wait, standard is YZX or indexed by quart.
                // Standard 1.18 chunk biomes are (size_y) * 4 * 4.
                // Index = qy * 16 + qz * 4 + qx ?
                // Let's assume standard YZX for now as user suggested: qy * 16 + qz * 4 + qx
                let idx = qy * 16 + qz * 4 + qx;

                let p = get_palette_index(data, biomes.palette.len(), idx);
                if p < biomes.palette.len() {
                    return normalize_biome_id(&biomes.palette[p]);
                }
            } else {
                return normalize_biome_id(&biomes.palette[0]);
            }
        }
    }

    // ---- 2) 1.15–1.17 per-section paletted container ----
    if let Some(biomes) = &section.biomes {
        if !biomes.palette.is_empty() {
             if let Some(data) = &biomes.data {
                let qy = by / 4;
                let idx = quart_index_4x4x4(qx, qz, qy);
                let p = get_palette_index(data, biomes.palette.len(), idx);
                if p < biomes.palette.len() {
                    return normalize_biome_id(&biomes.palette[p]);
                }
             } else {
                 return normalize_biome_id(&biomes.palette[0]);
             }
        }
    }

    // ---- 3) pre-1.15 legacy 256-byte column biomes ----
    if let Some(level) = &root.level {
        if let Some(b) = &level.biomes_256 {
            if b.len() >= 256 {
                let idx = (bz * 16 + bx) as usize;
                if idx < b.len() {
                    let id = b[idx] as u8;
                    return match id {
                        0 => "minecraft:ocean",
                        1 => "minecraft:plains",
                        2 => "minecraft:desert",
                        3 => "minecraft:mountains",
                        4 => "minecraft:forest",
                        5 => "minecraft:taiga",
                        6 => "minecraft:swamp",
                        7 => "minecraft:river",
                        8 => "minecraft:nether_wastes",
                        9 => "minecraft:the_end",
                        10 => "minecraft:frozen_ocean",
                        11 => "minecraft:frozen_river",
                        12 => "minecraft:snowy_tundra",
                        13 => "minecraft:snowy_mountains",
                        14 => "minecraft:mushroom_fields",
                        15 => "minecraft:mushroom_field_shore",
                        16 => "minecraft:beach",
                        17 => "minecraft:desert_hills",
                        18 => "minecraft:wooded_hills",
                        19 => "minecraft:taiga_hills",
                        21 => "minecraft:jungle",
                        22 => "minecraft:jungle_hills",
                        _ => "minecraft:plains", // fallback
                    }.to_string();
                }
            }
        }
    }

    "minecraft:plains".to_string()
}

fn map_legacy_id(id: u8, _data: u8) -> &'static str {
    match id {
        0 => "minecraft:air",
        1 => "minecraft:stone",
        2 => "minecraft:grass_block",
        3 => "minecraft:dirt",
        4 => "minecraft:cobblestone",
        5 => "minecraft:oak_planks",
        6 => "minecraft:oak_sapling",
        7 => "minecraft:bedrock",
        8 | 9 => "minecraft:water",
        10 | 11 => "minecraft:lava",
        12 => "minecraft:sand",
        13 => "minecraft:gravel",
        14 => "minecraft:gold_ore",
        15 => "minecraft:iron_ore",
        16 => "minecraft:coal_ore",
        17 => "minecraft:oak_log",
        18 => "minecraft:oak_leaves",
        19 => "minecraft:sponge",
        20 => "minecraft:glass",
        21 => "minecraft:lapis_ore",
        22 => "minecraft:lapis_block",
        24 => "minecraft:sandstone",
        35 => "minecraft:white_wool",
        _ => "minecraft:stone", // Fallback to visible
    }
}

// ------------------------------------------------------------------
fn log_debug(msg: &str) {
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/mcworlds_debug.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

/// Extracts the palette index from a compacted long array
fn get_palette_index(data: &[i64], palette_len: usize, block_idx: usize) -> usize {
    if data.is_empty() { return 0; }
    
    let bits_per_block = (palette_len as f64).log2().ceil() as usize;
    let bits_per_block = if bits_per_block < 4 { 4 } else { bits_per_block };
    
    let blocks_per_long = 64 / bits_per_block;
    let long_idx = block_idx / blocks_per_long;
    let sub_idx = block_idx % blocks_per_long;
    
    if long_idx >= data.len() { return 0; }
    
    let long_val = data[long_idx] as u64;
    let shift = sub_idx * bits_per_block;
    let mask = (1 << bits_per_block) - 1;
    
    ((long_val >> shift) & mask) as usize
}

fn ensure_textures_loaded(app_handle: &tauri::AppHandle) {
    if MAPS_LOADED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    // Try loading block_textures.json
    if let Ok(data_dir) = app_handle.path().app_data_dir() {
        let path = data_dir.join("data").join("block_textures.json");
        if path.exists() {
             if let Ok(content) = fs::read_to_string(path) {
                 if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                     let mut lock = BLOCK_TEXTURES_MAP.write().unwrap();
                     *lock = map;
                 }
             }
        }
    }
    
    MAPS_LOADED.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Resolves a texture ID for a block name
fn get_texture_id_for_block(block_name: &str) -> String {
    let map = BLOCK_TEXTURES_MAP.read().unwrap();
    if let Some(tex) = map.get(block_name) {
        // log_debug(&format!("Found texture map for {}: {}", block_name, tex));
        tex.clone()
    } else {
        // Heuristic fallback
        let bare = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
        format!("minecraft:block/{}", bare)
    }
}

/// Resolves the color of a block (Low Res / Fallback).
fn resolve_block_color(
    app_handle: &tauri::AppHandle, 
    block_name: &str, 
    use_texture: bool
) -> [u8; 4] {
    // 1. Block Color Mode
    if !use_texture {
        return if let Some(c) = BLOCK_COLORS.get(block_name) {
            [c[0], c[1], c[2], 255]
        } else {
            [0, 0, 0, 0]
        };
    }

    // 2. Texture Mode - Check Cache
    {
        let cache = TEXTURE_COLOR_CACHE.read().unwrap();
        if let Some(color) = cache.get(block_name) {
            return *color;
        }
    }

    ensure_textures_loaded(app_handle);

    // 3. Texture Mode - Load from Disk (Average Color)
    let texture_id = get_texture_id_for_block(block_name);
    let filename = crate::java_assets::get_texture_cache_filename(&texture_id);
    
    let color = if let Ok(cache_dir) = crate::java_assets::get_cache_dir(app_handle) {
        let file_path = cache_dir.join("textures").join(&filename);
        
        if file_path.exists() {
             if let Ok(img) = image::open(&file_path) {
                 let mut r = 0u64;
                 let mut g = 0u64;
                 let mut b = 0u64;
                 let mut count = 0u64;
                 
                 for p in img.pixels() {
                     let c = p.2; // Rgba
                     if c[3] > 10 {
                        r += c[0] as u64;
                        g += c[1] as u64;
                        b += c[2] as u64;
                        count += 1;
                     }
                 }
                 
                 if count > 0 {
                     [(r/count) as u8, (g/count) as u8, (b/count) as u8, 255]
                 } else {
                     [0, 0, 0, 0]
                 }
             } else {
                 if let Some(c) = BLOCK_COLORS.get(block_name) {
                    [c[0], c[1], c[2], 255]
                 } else {
                    [0, 0, 0, 0]
                 }
             }
        } else {
            // Fallback to block colors
            if let Some(c) = BLOCK_COLORS.get(block_name) {
                [c[0], c[1], c[2], 255]
            } else {
                 let bare_name = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
                 if let Some(c) = BLOCK_COLORS.get(bare_name) {
                     [c[0], c[1], c[2], 255]
                 } else {
                     // println!("[DEBUG] Unknown block color: {}", block_name);
                     [255, 0, 255, 255] 
                 }
            }
        }
    } else {
        [255, 0, 255, 255]
    };

    // 4. Update Cache
    {
        let mut cache = TEXTURE_COLOR_CACHE.write().unwrap();
        cache.insert(block_name.to_string(), color);
    }
    
    color
}

/// Helper to get or load a resized texture for blitting
fn get_block_texture_pixels(
    app_handle: &tauri::AppHandle, 
    block_name: &str, 
    size: u32
) -> Option<Vec<u8>> {
    let key = (block_name.to_string(), size);

    // 1. Check Read Cache
    {
        let cache = RESIZED_TEXTURE_CACHE.read().unwrap();
        if let Some(pixels) = cache.get(&key) {
            // log_debug(&format!("Cache hit for {}", block_name));
            return Some(pixels.clone());
        }
    }
    
    ensure_textures_loaded(app_handle);

    // 2. Load and Resize
    let texture_id = get_texture_id_for_block(block_name);
    let filename = crate::java_assets::get_texture_cache_filename(&texture_id);
    
    let pixels = if let Ok(cache_dir) = crate::java_assets::get_cache_dir(app_handle) {
        let file_path = cache_dir.join("textures").join(&filename);
        if file_path.exists() {
            if let Ok(img) = image::open(&file_path) {
                let resized = img.resize_exact(size, size, FilterType::Nearest);
                Some(resized.to_rgba8().into_raw())
            } else {
                log_debug(&format!("Failed to open image: {:?}", file_path));
                None
            }
        } else {
             // log_debug(&format!("Texture missing: {}", filename));
             if block_name != "minecraft:air" {
                 // log_debug(&format!("Texture file missing for block {}: {:?} (ID: {})", block_name, file_path, texture_id));
             }
            None
        }
    } else {
        log_debug("Failed to get cache dir");
        None
    };

    // 3. Write Cache (even if None, to avoid re-attempting? No, might appear later)
    if let Some(p) = &pixels {
        let mut cache = RESIZED_TEXTURE_CACHE.write().unwrap();
        cache.insert(key, p.clone());
    }

    pixels
}

// ------------------------------------------------------------------
// Commands
// ------------------------------------------------------------------

#[tauri::command]
pub fn get_regions(world_path: String) -> Result<Vec<RegionCoords>, String> {
    let path = Path::new(&world_path).join("region");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut regions = Vec::new();
    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with("r.") && name_str.ends_with(".mca") {
            let parts: Vec<&str> = name_str.split('.').collect();
            if parts.len() == 4 {
                if let (Ok(x), Ok(z)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                    regions.push(RegionCoords { x, z });
                }
            }
        }
    }
    
    regions.sort_by(|a, b| {
        if a.x != b.x {
            a.x.cmp(&b.x)
        } else {
            a.z.cmp(&b.z)
        }
    });

    Ok(regions)
}

#[tauri::command]
pub async fn get_region_map_image(
    app_handle: tauri::AppHandle,
    world_path: String, 
    region_x: i32, 
    region_z: i32, 
    use_texture_colors: bool,
    scale: Option<u32>
) -> Result<Vec<u8>, String> {
    log_debug(&format!("Rendering region {},{} scale: {:?} use_textures: {}", region_x, region_z, scale, use_texture_colors));
    // println!("[DEBUG] Rendering region {},{} scale: {:?}", region_x, region_z, scale);

    ensure_biomes_loaded(&app_handle);
    let biomes_map = BIOME_PARAMS.read().unwrap();
    let grass_map = COLORMAP_GRASS.read().unwrap();
    let foliage_map = COLORMAP_FOLIAGE.read().unwrap();

    let region_file = Path::new(&world_path)
        .join("region")
        .join(format!("r.{}.{}.mca", region_x, region_z));

    if !region_file.exists() {
        return Err("Region file not found".to_string());
    }

    // 1. Read File (Option B: Read Whole File)
    let mut file = File::open(&region_file).map_err(|e| e.to_string())?;
    let mut file_data = Vec::new();
    if let Err(_) = file.read_to_end(&mut file_data) {
         let region_pixels = 512 * scale.unwrap_or(1).max(1).min(16);
         let expected_len = (region_pixels * region_pixels * 4) as usize;
         return Ok(vec![0u8; expected_len]);
    }
    
    let scale_factor = scale.unwrap_or(1).max(1).min(16);
    let region_pixels = 512 * scale_factor;
    let expected_len = (region_pixels * region_pixels * 4) as usize;

    if file_data.len() < 8192 {
        // File too small to contain valid header, return blank
         return Ok(vec![0u8; expected_len]);
    }

    // Shared references for Rayon
    let file_data = Arc::new(file_data);
    let app_handle = Arc::new(app_handle);

    // High fidelity mode if using textures and zoomed in
    let high_fidelity = use_texture_colors && scale_factor >= 4;

    // 2. Parallel Processing
    // Generate all chunk coordinates
    let chunk_coords: Vec<(u32, u32)> = (0..32).flat_map(|z| (0..32).map(move |x| (x, z))).collect();

    let rendered_chunks: Vec<((u32, u32), Vec<u8>)> = chunk_coords
        .par_iter()
        .map(|(cx, cz)| {
            let cx = *cx;
            let cz = *cz;

            // Read Offset from header (first 4KB of file_data)
            let loc_offset = 4 * (cz * 32 + cx) as usize;
            // Bound check just in case, though file size check > 8192 covers it
            if loc_offset + 3 >= file_data.len() { return ((cx, cz), Vec::new()); }
            
            let loc_bytes = &file_data[loc_offset..loc_offset+4];
            let offset = ((loc_bytes[0] as u32) << 16) | ((loc_bytes[1] as u32) << 8) | (loc_bytes[2] as u32);

            if offset < 2 { return ((cx, cz), Vec::new()); } // Empty or Invalid

            // Sector offset is absolute from file start
            let sector_offset = (offset as usize) * 4096;
            if sector_offset >= file_data.len() { return ((cx, cz), Vec::new()); }

            // Parse Chunk
            let chunk_wrapper = &file_data[sector_offset..];
            if chunk_wrapper.len() < 5 { return ((cx, cz), Vec::new()); }
            
            let len = ((chunk_wrapper[0] as u32) << 24) | ((chunk_wrapper[1] as u32) << 16) | ((chunk_wrapper[2] as u32) << 8) | (chunk_wrapper[3] as u32);
            let compression = chunk_wrapper[4];
            
            if len == 0 || (len as usize) > chunk_wrapper.len() - 4 { return ((cx, cz), Vec::new()); }
            
            let compressed_data = &chunk_wrapper[5..(len as usize + 4)];
            let mut decompressed = Vec::new();
            
            let res = match compression {
                1 => GzDecoder::new(Cursor::new(compressed_data)).read_to_end(&mut decompressed),
                2 => ZlibDecoder::new(Cursor::new(compressed_data)).read_to_end(&mut decompressed),
                _ => return ((cx, cz), Vec::new()),
            };
            if res.is_err() { return ((cx, cz), Vec::new()); }
            
            let chunk_root = match fastnbt::from_bytes::<ChunkRoot>(&decompressed) {
                Ok(r) => r,
                Err(_) => return ((cx, cz), Vec::new())
            };

            let sections_option = if let Some(s) = &chunk_root.sections { Some(s) }
                else if let Some(l) = &chunk_root.level { l.sections.as_ref() }
                else { None };
            // The biomes_map is already available from the outer scope due to closure capture.
            // let biomes_map = biomes_lock.read().unwrap(); // This line would cause a compile error as biomes_lock is not defined here.
            // log_debug(&format!("Biome params loaded:    let biomes_map = biomes_lock.read().unwrap();
    // log_debug(&format!("Biome params loaded: {}", biomes_map.len()));
    
    // Scan Chunk
    let mut chunk_img = vec![0u8; (16 * scale_factor * 16 * scale_factor * 4) as usize];

            if let Some(sections_ref) = sections_option {
                // We need a sorted copy for raycasting
                let mut sections: Vec<&Section> = sections_ref.iter().collect();
                sections.sort_by(|a, b| b.y.cmp(&a.y)); // Top to Bottom

                // DEBUG: REGION-WIDE AGGRESSIVE SCANNER
                // (Removed)
                
                // Raycast per block column (0..16, 0..16)
                for bz in 0..16 {
                    for bx in 0..16 {
                        let mut found_block: Option<(String, i32, String)> = None; // name, y, biome_id
                        
                        'section_scan: for section in &sections {
                            // Determine which format we have
                            let (palette, data) = if let Some(bs) = &section.block_states_container {
                                // 1.16+ Format
                                (bs.palette.as_ref(), bs.data.as_ref())
                            } else if let Some(pal) = &section.palette {
                                // Legacy Format
                                (Some(pal), section.legacy_block_states.as_ref())
                            } else {
                                (None, None)
                            };
                            
                            if let Some(pal) = palette {
                                // Let's iterate Y (top to bottom)
                                for by in (0..16).rev() {
                                    let block_idx = (by * 256) + (bz * 16) + bx;
                                    let pal_idx = if let Some(d) = data {
                                        get_palette_index(d, pal.len(), block_idx)
                                    } else { 0 };
                                    
                                    if pal_idx < pal.len() {
                                        let entry = &pal[pal_idx];
                                        if entry.name != "minecraft:air" && entry.name != "minecraft:cave_air" && entry.name != "minecraft:void_air" {
                                            // Found valid block
                                            let y = (section.y as i32 * 16) + by as i32;
                                            
                                            // Resolve Biome
                                            let biome_id = resolve_biome_id_any(
                                                &chunk_root,
                                                &sections,
                                                section,
                                                bx as usize,
                                                bz as usize,
                                                by as usize
                                            );

                                            found_block = Some((entry.name.clone(), y, biome_id));
                                            break 'section_scan;
                                        }
                                    }
                                }
                            } else if let Some(blocks) = &section.legacy_blocks_u8 {
                                // Pre-1.13 Format (Levels < 1.13)
                                for by in (0..16).rev() {
                                    let block_idx = (by * 256) + (bz * 16) + bx;
                                    if block_idx < blocks.len() {
                                        let id = blocks[block_idx] as u8;
                                        if id != 0 {
                                            // Found valid block
                                            let data_val = if let Some(d) = &section.legacy_data_u8 {
                                                if block_idx / 2 < d.len() {
                                                    let byte = d[block_idx / 2] as u8;
                                                    if block_idx % 2 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F }
                                                } else { 0 }
                                            } else { 0 };

                                            let y = (section.y as i32 * 16) + by as i32;
                                            // Resolve Biome for Legacy
                                            let biome_id = resolve_biome_id_any(
                                                &chunk_root,
                                                &sections,
                                                section,
                                                bx as usize,
                                                bz as usize,
                                                by as usize
                                            );
                                            found_block = Some((map_legacy_id(id, data_val).to_string(), y, biome_id));
                                            break 'section_scan;
                                        }
                                    }
                                }
                            }
                        }
                        
                        if let Some((block_name, y, biome_id)) = found_block {
                             // Height-based shading: -64 to 320
                             let shade = (((y + 64) as f32 / 384.0).max(0.0).min(1.0) * 0.4 + 0.6) as f32; // 0.6 to 1.0 brightness
                             
                             // Calculate Tint
                             // Calculate Tint
                             let mut tint = [255u8, 255, 255];
                             let mut use_tint = false;
                             
                             if let Some(biome) = biomes_map.get(&biome_id) {
                                  // Found biome
                                   if block_name == "minecraft:water" {
                                       // Water Tint
                                      if let Some(c) = biome.water_color {
                                          tint = [(c >> 16 & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c & 0xFF) as u8];
                                          use_tint = true;
                                      }
                                   } else if block_name.contains("leaves") || block_name == "minecraft:vine" {
                                      // Foliage Tint
                                      // Check override first
                                      if let Some(c) = biome.foliage_color {
                                            tint = [(c >> 16 & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c & 0xFF) as u8];
                                            use_tint = true;
                                      } else if let Some(map) = foliage_map.as_ref() {
                                          tint = sample_colormap(map, biome.temperature, biome.downfall);
                                          use_tint = true;
                                      }
                                   } else if block_name == "minecraft:grass_block" || block_name == "minecraft:grass" || block_name == "minecraft:tall_grass" || block_name == "minecraft:fern" {
                                      // Grass Tint
                                      // Check override first
                                      if let Some(c) = biome.grass_color {
                                            tint = [(c >> 16 & 0xFF) as u8, (c >> 8 & 0xFF) as u8, (c & 0xFF) as u8];
                                            use_tint = true;
                                      } else if let Some(map) = grass_map.as_ref() {
                                          tint = sample_colormap(map, biome.temperature, biome.downfall);
                                          use_tint = true;
                                      }
                                   }
                             } else {
                                 // Missing Biome Logic
                                 log_missing_biome(&biome_id);
                             }
                             
                             if use_tint {
                                 // log_debug(&format!("Tinting {} with biome {}", block_name, biome_id));
                             }

                             if high_fidelity {
                                 // Blit Texture
                                 if let Some(pixels) = get_block_texture_pixels(&app_handle, &block_name, scale_factor) {
                                     // Copy pixels
                                     // log_debug("Blitting pixels");
                                     for sy in 0..scale_factor {
                                         for sx in 0..scale_factor {
                                                 let tex_idx = ((sy * scale_factor + sx) * 4) as usize;
                                                 let px = bx as u32 * scale_factor + sx;
                                                 let pz = bz as u32 * scale_factor + sy;
                                                 let idx = ((pz * 16 * scale_factor + px) * 4) as usize;
                                                 
                                                 if tex_idx + 3 < pixels.len() {
                                                     let mut r = pixels[tex_idx] as f32;
                                                     let mut g = pixels[tex_idx+1] as f32;
                                                     let mut b = pixels[tex_idx+2] as f32;
                                                     
                                                     if use_tint {
                                                         r *= tint[0] as f32 / 255.0;
                                                         g *= tint[1] as f32 / 255.0;
                                                         b *= tint[2] as f32 / 255.0;
                                                     }

                                                     chunk_img[idx] = (r * shade) as u8;
                                                     chunk_img[idx+1] = (g * shade) as u8;
                                                     chunk_img[idx+2] = (b * shade) as u8;
                                                     chunk_img[idx+3] = pixels[tex_idx+3];
                                                 }
                                         }
                                     }
                                 } else {
                                     // Fallback color blit
                                     let color = resolve_block_color(&app_handle, &block_name, true);
                                     if color[3] > 0 {
                                         for sy in 0..scale_factor {
                                             for sx in 0..scale_factor {
                                                 let px = bx as u32 * scale_factor + sx;
                                                 let pz = bz as u32 * scale_factor + sy;
                                                 let idx = ((pz * 16 * scale_factor + px) * 4) as usize;
                                                  let mut r = color[0] as f32;
                                                  let mut g = color[1] as f32;
                                                  let mut b = color[2] as f32;
                                                  
                                                  if use_tint {
                                                      r *= tint[0] as f32 / 255.0;
                                                      g *= tint[1] as f32 / 255.0;
                                                      b *= tint[2] as f32 / 255.0;
                                                  }

                                                  chunk_img[idx] = (r * shade) as u8;
                                                  chunk_img[idx+1] = (g * shade) as u8;
                                                  chunk_img[idx+2] = (b * shade) as u8;
                                                  chunk_img[idx+3] = 255;
                                             }
                                         }
                                     }
                                 }
                             } else {
                                 // Simple Color Fill
                                 let color = resolve_block_color(&app_handle, &block_name, use_texture_colors);
                                 if color[3] > 0 {
                                     for sy in 0..scale_factor {
                                         for sx in 0..scale_factor {
                                             let px = bx as u32 * scale_factor + sx;
                                             let pz = bz as u32 * scale_factor + sy;
                                             let idx = ((pz * 16 * scale_factor + px) * 4) as usize;
                                              let mut r = color[0] as f32;
                                              let mut g = color[1] as f32;
                                              let mut b = color[2] as f32;
                                              
                                              if use_tint {
                                                  r *= tint[0] as f32 / 255.0;
                                                  g *= tint[1] as f32 / 255.0;
                                                  b *= tint[2] as f32 / 255.0;
                                              }
                                              
                                              chunk_img[idx] = (r * shade) as u8;
                                              chunk_img[idx+1] = (g * shade) as u8;
                                              chunk_img[idx+2] = (b * shade) as u8;
                                              chunk_img[idx+3] = 255;
                                         }
                                     }
                                 }
                             }
                        }
                    }
                }
            }
            
            ((cx, cz), chunk_img)
        })
        .collect();

    // 3. Assemble Output Image
    let mut final_img = vec![0u8; (region_pixels * region_pixels * 4) as usize];
    let chunk_pixel_width = 16 * scale_factor;
    
    for ((cx, cz), chunk_pixels) in rendered_chunks {
        if chunk_pixels.is_empty() { continue; }
        
        // Copy chunk_pixels into final_img at correct position
        let start_x = cx * chunk_pixel_width;
        let start_z = cz * chunk_pixel_width;
        
        for r in 0..chunk_pixel_width {
            let src_start = (r * chunk_pixel_width * 4) as usize;
            let src_end = src_start + (chunk_pixel_width * 4) as usize;
            
            let dst_row = start_z + r;
            let dst_start = ((dst_row * region_pixels + start_x) * 4) as usize;
            let dst_end = dst_start + (chunk_pixel_width * 4) as usize;
            
            if dst_end <= final_img.len() && src_end <= chunk_pixels.len() {
                final_img[dst_start..dst_end].copy_from_slice(&chunk_pixels[src_start..src_end]);
            }
        }
    }

    Ok(final_img)
}

#[derive(Serialize, Clone, Debug)]
pub struct TopBlock {
    pub name: String,
    pub y: i32,
    pub biome: Option<String>,
}

#[tauri::command]
pub fn get_highest_block_at(
    world_path: String,
    x: i32,
    z: i32
) -> Result<Option<TopBlock>, String> {
    let region_x = x >> 9; // Divide by 512
    let region_z = z >> 9;
    
    let region_file = Path::new(&world_path)
        .join("region")
        .join(format!("r.{}.{}.mca", region_x, region_z));

    if !region_file.exists() {
        return Ok(None);
    }

    let mut file = File::open(&region_file).map_err(|e| e.to_string())?;
    
    // Calculate chunk coordinates within region (0..31)
    let cx = ((x % 512) + 512) % 512 / 16;
    let cz = ((z % 512) + 512) % 512 / 16;
    
    // Chunk location offset
    let loc_offset = 4 * (cz * 32 + cx) as usize;
    
    // We need to seek to this offset
    use std::io::Seek;
    file.seek(std::io::SeekFrom::Start(loc_offset as u64)).map_err(|e| e.to_string())?;
    
    let mut loc_bytes = [0u8; 4];
    file.read_exact(&mut loc_bytes).map_err(|e| e.to_string())?;
    
    let offset = ((loc_bytes[0] as u32) << 16) | ((loc_bytes[1] as u32) << 8) | (loc_bytes[2] as u32);
    if offset < 2 { return Ok(None); }
    
    let sector_offset = (offset as usize) * 4096;
    
    file.seek(std::io::SeekFrom::Start(sector_offset as u64)).map_err(|e| e.to_string())?;
    
    let mut header = [0u8; 5];
    file.read_exact(&mut header).map_err(|e| e.to_string())?;
    
    let len = ((header[0] as u32) << 24) | ((header[1] as u32) << 16) | ((header[2] as u32) << 8) | (header[3] as u32);
    let compression = header[4];
    
    if len == 0 { return Ok(None); }
    
    let mut compressed_data = vec![0u8; len as usize - 1]; // -1 for compression byte
    file.read_exact(&mut compressed_data).map_err(|e| e.to_string())?;
    
    let mut decompressed = Vec::new();
    let res = match compression {
        1 => GzDecoder::new(Cursor::new(compressed_data)).read_to_end(&mut decompressed),
        2 => ZlibDecoder::new(Cursor::new(compressed_data)).read_to_end(&mut decompressed),
        _ => return Ok(None),
    };
    
    if res.is_err() { return Ok(None); }
    
    let root: ChunkRoot = fastnbt::from_bytes(&decompressed).map_err(|e| e.to_string())?;
    
    let sections_ref = if let Some(s) = &root.sections { Some(s) }
        else if let Some(l) = &root.level { l.sections.as_ref() }
        else { None };

    if let Some(sections) = sections_ref {
        // Find highest block
        // Sort sections top to bottom
        let mut sorted_sections: Vec<&Section> = sections.iter().collect();
        sorted_sections.sort_by(|a, b| b.y.cmp(&a.y));
        
        let bx = ((x % 16) + 16) % 16;
        let bz = ((z % 16) + 16) % 16;
        
        for section in sorted_sections {
            let (palette, data) = if let Some(bs) = &section.block_states_container {
                (bs.palette.as_ref(), bs.data.as_ref())
            } else if let Some(pal) = &section.palette {
                (Some(pal), section.legacy_block_states.as_ref())
            } else {
                (None, None)
            };
            
            if let Some(pal) = palette {
                for by in (0..16).rev() {
                    let block_idx = (by * 256) + (bz as usize * 16) + bx as usize;
                    let pal_idx = if let Some(d) = data {
                        get_palette_index(d, pal.len(), block_idx)
                    } else { 0 };
                    
                    if pal_idx < pal.len() {
                        let name = &pal[pal_idx].name;
                        if name != "minecraft:air" && name != "minecraft:cave_air" && name != "minecraft:void_air" {
                            let y = (section.y as i32 * 16) + by as i32;
                            
                            // Resolve Biome
                            // We need all sections as refs for the resolver
                            let all_sections: Vec<&Section> = sections.iter().collect();
                            let biome = resolve_biome_id_any(
                                &root,
                                &all_sections,
                                section,
                                bx as usize,
                                bz as usize,
                                by as usize
                            );
                            
                            return Ok(Some(TopBlock { name: name.clone(), y, biome: Some(biome) }));
                        }
                    }
                }
            } else if let Some(blocks) = &section.legacy_blocks_u8 {
                 for by in (0..16).rev() {
                    let block_idx = (by * 256) + (bz as usize * 16) + bx as usize;
                    if block_idx < blocks.len() {
                        let id = blocks[block_idx];
                        if id != 0 {
                             let data_val = if let Some(d) = &section.legacy_data_u8 {
                                 // Extract nibble
                                 let d_byte = d[block_idx / 2] as u8;
                                 if block_idx % 2 == 0 { d_byte & 0x0F } else { (d_byte >> 4) & 0x0F }
                             } else { 0 };
                             
                             let name = map_legacy_id(id as u8, data_val);
                             if name != "minecraft:air" {
                                let y = (section.y as i32 * 16) + by as i32;
                                
                                // Resolve Biome
                                let all_sections: Vec<&Section> = sections.iter().collect();
                                let biome = resolve_biome_id_any(
                                    &root,
                                    &all_sections,
                                    section,
                                    bx as usize,
                                    bz as usize,
                                    by as usize
                                );
                                
                                return Ok(Some(TopBlock { name: name.to_string(), y, biome: Some(biome) }));
                             }
                        }
                    }
                 }
            }
        }
    }

    Ok(None)
}

