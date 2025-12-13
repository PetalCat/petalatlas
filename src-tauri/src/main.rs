// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod scanner;
mod cache;
mod nbt_editor;
mod players;
use scanner::WorldInfo;
use cache::{save_worlds_cache, load_worlds_cache, update_world_in_cache};
use nbt_editor::{read_level_dat, write_level_dat};
use players::{get_players, get_player_details, fetch_player_profile, fetch_player_avatar, fetch_image, clear_player_caches};
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
            read_level_dat,
            write_level_dat,
            get_players,
            get_player_details,
            fetch_player_profile,
            fetch_player_avatar,
            fetch_image,
            clear_player_caches
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
