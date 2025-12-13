use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Debug)]
pub struct NbtData {
    pub data: serde_json::Value,
}

#[tauri::command]
pub fn read_level_dat(path: String) -> Result<NbtData, String> {
    let level_dat_path = PathBuf::from(&path).join("level.dat");
    
    if !level_dat_path.exists() {
        return Err("level.dat not found".to_string());
    }

    // Read and decompress
    let file = fs::File::open(&level_dat_path)
        .map_err(|e| format!("Failed to open level.dat: {}", e))?;
    
    let mut decoder = GzDecoder::new(file);
    let mut bytes = Vec::new();
    decoder.read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to decompress level.dat: {}", e))?;

    // Parse NBT to generic value
    let nbt_value: fastnbt::Value = fastnbt::from_bytes(&bytes)
        .map_err(|e| format!("Failed to parse NBT: {}", e))?;

    // Convert to JSON for easier editing
    let json_value = nbt_to_json(&nbt_value);

    Ok(NbtData {
        data: json_value,
    })
}

#[tauri::command]
pub fn write_level_dat(path: String, data: serde_json::Value) -> Result<(), String> {
    let level_dat_path = PathBuf::from(&path).join("level.dat");
    
    if !level_dat_path.exists() {
        return Err("level.dat not found".to_string());
    }

    // Create backup first
    let backup_path = PathBuf::from(&path).join("level.dat.backup");
    fs::copy(&level_dat_path, &backup_path)
        .map_err(|e| format!("Failed to create backup: {}", e))?;

    // Convert JSON back to NBT
    let nbt_value = json_to_nbt(&data)
        .map_err(|e| format!("Failed to convert JSON to NBT: {}", e))?;

    // Serialize NBT
    let nbt_bytes = fastnbt::to_bytes(&nbt_value)
        .map_err(|e| format!("Failed to serialize NBT: {}", e))?;

    // Compress and write
    let file = fs::File::create(&level_dat_path)
        .map_err(|e| format!("Failed to create level.dat: {}", e))?;
    
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&nbt_bytes)
        .map_err(|e| format!("Failed to write compressed data: {}", e))?;
    
    encoder.finish()
        .map_err(|e| format!("Failed to finish compression: {}", e))?;

    Ok(())
}

// Convert NBT Value to JSON Value
fn nbt_to_json(nbt: &fastnbt::Value) -> serde_json::Value {
    match nbt {
        fastnbt::Value::Byte(b) => serde_json::Value::Number((*b).into()),
        fastnbt::Value::Short(s) => serde_json::Value::Number((*s).into()),
        fastnbt::Value::Int(i) => serde_json::Value::Number((*i).into()),
        fastnbt::Value::Long(l) => serde_json::Value::Number((*l).into()),
        fastnbt::Value::Float(f) => {
            serde_json::Number::from_f64(*f as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        fastnbt::Value::Double(d) => {
            serde_json::Number::from_f64(*d)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        fastnbt::Value::String(s) => serde_json::Value::String(s.clone()),
        fastnbt::Value::List(list) => {
            let json_list: Vec<serde_json::Value> = list.iter().map(nbt_to_json).collect();
            serde_json::Value::Array(json_list)
        }
        fastnbt::Value::Compound(compound) => {
            let mut map = serde_json::Map::new();
            for (key, value) in compound.iter() {
                map.insert(key.clone(), nbt_to_json(value));
            }
            serde_json::Value::Object(map)
        }
        fastnbt::Value::ByteArray(arr) => {
            let json_arr: Vec<serde_json::Value> = arr.iter().map(|b| serde_json::Value::Number((*b).into())).collect();
            serde_json::Value::Array(json_arr)
        }
        fastnbt::Value::IntArray(arr) => {
            let json_arr: Vec<serde_json::Value> = arr.iter().map(|i| serde_json::Value::Number((*i).into())).collect();
            serde_json::Value::Array(json_arr)
        }
        fastnbt::Value::LongArray(arr) => {
            let json_arr: Vec<serde_json::Value> = arr.iter().map(|l| serde_json::Value::Number((*l).into())).collect();
            serde_json::Value::Array(json_arr)
        }
    }
}

// Convert JSON Value back to NBT Value
fn json_to_nbt(json: &serde_json::Value) -> Result<fastnbt::Value, String> {
    match json {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(fastnbt::Value::Long(i))
            } else if let Some(f) = n.as_f64() {
                Ok(fastnbt::Value::Double(f))
            } else {
                Err("Invalid number".to_string())
            }
        }
        serde_json::Value::String(s) => Ok(fastnbt::Value::String(s.clone())),
        serde_json::Value::Bool(b) => Ok(fastnbt::Value::Byte(if *b { 1 } else { 0 })),
        serde_json::Value::Array(arr) => {
            let nbt_list: Result<Vec<fastnbt::Value>, String> = arr.iter().map(json_to_nbt).collect();
            Ok(fastnbt::Value::List(nbt_list?))
        }
        serde_json::Value::Object(obj) => {
            let mut compound = std::collections::HashMap::new();
            for (key, value) in obj.iter() {
                compound.insert(key.clone(), json_to_nbt(value)?);
            }
            Ok(fastnbt::Value::Compound(compound))
        }
        serde_json::Value::Null => Ok(fastnbt::Value::Byte(0)),
    }
}
