// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod scanner;
mod cache;
mod nbt_editor;
mod players;
mod java_assets;
mod map_renderer;
mod biome_types;
#[cfg(test)]
mod dev_tools;
use scanner::WorldInfo;
use cache::{save_worlds_cache, load_worlds_cache, update_world_in_cache};
use nbt_editor::{read_nbt_file, write_nbt_file, list_nbt_files};
use players::{get_players, get_player_details, get_player_full, save_player_data, fetch_player_profile, fetch_player_avatar, fetch_image, clear_player_caches, get_player_advancements};
use java_assets::{detect_java_installation, get_minecraft_versions, select_java_jar, search_for_client_jars, extract_texture, extract_language, extract_advancements, extract_registries, get_cached_asset, clear_asset_cache, extract_all_block_colors, extract_biome_data};
use map_renderer::{get_regions, get_region_map_image, get_highest_block_at};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use tauri::State;

struct ScanCancelFlag(Arc<AtomicBool>);
struct SkipZipFlag(Arc<AtomicBool>);

#[tauri::command]
async fn scan_worlds(
    window: tauri::Window, 
    path: String, 
    max_depth: usize,
    scan_zips: bool,
    cancel_flag: State<'_, ScanCancelFlag>,
    skip_zip_flag: State<'_, SkipZipFlag>
) -> Result<Vec<WorldInfo>, String> {
    cancel_flag.0.store(false, Ordering::Relaxed);
    skip_zip_flag.0.store(false, Ordering::Relaxed);
    Ok(scanner::scan_for_worlds(&path, max_depth, scan_zips, window, cancel_flag.0.clone(), skip_zip_flag.0.clone()))
}

#[tauri::command]
fn cancel_scan(cancel_flag: State<'_, ScanCancelFlag>) {
    cancel_flag.0.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn skip_zip(skip_zip_flag: State<'_, SkipZipFlag>) {
    skip_zip_flag.0.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(ScanCancelFlag(Arc::new(AtomicBool::new(false))))
        .manage(SkipZipFlag(Arc::new(AtomicBool::new(false))))
        .invoke_handler(tauri::generate_handler![
            greet, 
            scan_worlds, 
            cancel_scan, 
            skip_zip,
            save_worlds_cache,
            load_worlds_cache,
            update_world_in_cache,
            read_nbt_file,
            write_nbt_file,
            list_nbt_files,
            get_players,
            get_player_details,
            get_player_full,
            save_player_data,
            fetch_player_profile,
            fetch_player_avatar,
            fetch_image,
            clear_player_caches,
            get_player_advancements,
            // Java assets
            detect_java_installation,
            get_minecraft_versions,
            select_java_jar,
            search_for_client_jars,
            extract_texture,
            extract_language,
            extract_advancements,
            extract_registries,
            get_cached_asset,
            clear_asset_cache,
            extract_all_block_colors,
            extract_biome_data,
            // Map Renderer
            get_regions,
            get_region_map_image,
            get_highest_block_at
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
