use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NbtFile {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub format: String, // "java", "bedrock", "snbt"
}

// Strict Typed NBT Structure
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum TypedNbt {
    #[serde(rename = "byte")]
    Byte(i8),
    #[serde(rename = "short")]
    Short(i16),
    #[serde(rename = "int")]
    Int(i32),
    #[serde(rename = "long")]
    Long(String), // String to preserve 64-bit precision in JSON
    #[serde(rename = "float")]
    Float(f32),
    #[serde(rename = "double")]
    Double(f64),
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "list")]
    List(Vec<TypedNbt>),
    #[serde(rename = "compound")]
    Compound(HashMap<String, TypedNbt>),
    #[serde(rename = "byte_array")]
    ByteArray(Vec<i8>),
    #[serde(rename = "int_array")]
    IntArray(Vec<i32>),
    #[serde(rename = "long_array")]
    LongArray(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NbtData {
    pub data: TypedNbt,
    pub format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub could_be_nbt: bool,
}

#[tauri::command]
pub fn list_nbt_files(world_path: String) -> Result<Vec<FileNode>, String> {
    let root = Path::new(&world_path);
    if !root.exists() {
        return Err("World path does not exist".to_string());
    }

    // We will scan specific directories of interest
    // 1. Root files (level.dat, level.dat_old)
    // 2. playerdata/
    // 3. stats/ (json, but good to have)
    // 4. data/
    
    let mut root_node = FileNode {
        name: "root".to_string(),
        path: world_path.clone(),
        relative_path: "".to_string(),
        is_dir: true,
        children: vec![],
        could_be_nbt: false,
    };

    let entries = fs::read_dir(root).map_err(|e| e.to_string())?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();
            
            // Skip .git, .DS_Store, etc
            if name.starts_with('.') { continue; }

            // Directories to recurse appropriately or file checks
            if is_dir {
                if name == "playerdata" || name == "data" || name == "region" { // region is special, but listed
                    let children = scan_dir(&path, &root);
                    root_node.children.push(FileNode {
                        name: name.clone(),
                        path: path.to_string_lossy().to_string(),
                        relative_path: name.clone(),
                        is_dir: true,
                        children,
                        could_be_nbt: false,
                    });
                }
            } else {
                 if is_nbt_candidate(&name) {
                    root_node.children.push(FileNode {
                        name: name.clone(),
                        path: path.to_string_lossy().to_string(),
                        relative_path: name.clone(),
                        is_dir: false,
                        children: vec![],
                        could_be_nbt: true,
                    });
                 }
            }
        }
    }
    
    // Sort directories first
    root_node.children.sort_by(|a, b| {
        if a.is_dir && !b.is_dir { return std::cmp::Ordering::Less; }
        if !a.is_dir && b.is_dir { return std::cmp::Ordering::Greater; }
        a.name.cmp(&b.name)
    });

    Ok(root_node.children)
}

fn scan_dir(dir: &Path, root: &Path) -> Vec<FileNode> {
    let mut nodes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                
                if name.starts_with('.') { continue; }

                if path.is_file() {
                    nodes.push(FileNode {
                        name: name.clone(),
                        path: path.to_string_lossy().to_string(),
                        relative_path: path.strip_prefix(root).unwrap_or(&path).to_string_lossy().to_string(),
                        is_dir: false,
                        children: vec![],
                        could_be_nbt: is_nbt_candidate(&name),
                    });
                }
            }
        }
    }
    nodes.sort_by_key(|n| n.name.clone());
    nodes
}

fn is_nbt_candidate(name: &str) -> bool {
    name.ends_with(".dat") || 
    name.ends_with(".dat_old") || 
    name.ends_with(".nbt") || 
    name.ends_with(".mcstructure") || 
    name.ends_with(".snbt")
}

#[tauri::command]
pub fn read_nbt_file(path: String) -> Result<NbtData, String> {
    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    let mut content = Vec::new();
    let mut file = fs::File::open(&file_path).map_err(|e| e.to_string())?;
    file.read_to_end(&mut content).map_err(|e| e.to_string())?;

    // Strategy 1: SNBT
    if path.ends_with(".snbt") {
        let text = String::from_utf8(content).map_err(|_| "Invalid UTF-8 in SNBT".to_string())?;
        match quartz_nbt::snbt::parse(&text) {
             Ok(compound) => {
                 let tag = quartz_nbt::NbtTag::Compound(compound);
                 return Ok(NbtData {
                     data: quartz_to_typed(&tag),
                     format: "snbt".to_string(),
                 });
             }
             Err(e) => return Err(format!("SNBT Parse Error: {}", e)),
        }
    }
    
    // Strategy 2: Java NBT via Quartz
    let mut cursor = Cursor::new(&content);
    match quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::GzCompressed) {
        Ok((compound, _)) => {
            let tag = quartz_nbt::NbtTag::Compound(compound);
            return Ok(NbtData {
                data: quartz_to_typed(&tag),
                format: "java".to_string(),
            });
        },
        Err(e) => eprintln!("Java GZ failed: {}", e),
    }

    let mut cursor = Cursor::new(&content);
    match quartz_nbt::io::read_nbt(&mut cursor, quartz_nbt::io::Flavor::Uncompressed) {
        Ok((compound, _)) => {
             let tag = quartz_nbt::NbtTag::Compound(compound);
             return Ok(NbtData {
                data: quartz_to_typed(&tag),
                format: "java_uncompressed".to_string(),
            });
        },
        Err(e) => eprintln!("Java Uncompressed failed: {}", e),
    }

    // Strategy 3: Bedrock via Zuri
    // Read raw LE NBT
    let mut cursor = Cursor::new(&content);
    // Note: zuri_nbt::NBTTag::read takes (reader, encoder)
    match zuri_nbt::NBTTag::read(&mut cursor, zuri_nbt::encoding::LittleEndian) {
        Ok(nbt_tag) => {
             return Ok(NbtData {
                 data: zuri_to_typed(&nbt_tag),
                 format: "bedrock".to_string(),
             });
        },
        Err(e) => eprintln!("Bedrock LE failed: {}", e),
    }
    
    // Header skip
    if content.len() > 8 {
         let mut cursor = Cursor::new(&content[8..]);
         match zuri_nbt::NBTTag::read(&mut cursor, zuri_nbt::encoding::LittleEndian) {
             Ok(nbt_tag) => {
                 return Ok(NbtData {
                     data: zuri_to_typed(&nbt_tag),
                     format: "bedrock_header".to_string(),
                 });
             },
             Err(e) => eprintln!("Bedrock Header failed: {}", e),
         }
    }

    Err("Could not auto-detect NBT format".to_string())
}

#[tauri::command]
pub fn write_nbt_file(path: String, data: TypedNbt, format: String) -> Result<(), String> {
    let file_path = Path::new(&path);
    
    // Create backup
    let backup_path = file_path.with_extension("backup");
    let _ = fs::copy(file_path, &backup_path);

    let file = fs::File::create(file_path).map_err(|e| e.to_string())?;

    match format.as_str() {
        "snbt" => {
            let tag = typed_to_quartz(&data).map_err(|e| e)?;
            if let quartz_nbt::NbtTag::Compound(c) = tag {
                let snbt = c.to_snbt();
                let mut writer = std::io::BufWriter::new(file);
                writer.write_all(snbt.as_bytes()).map_err(|e| e.to_string())?;
                writer.flush().map_err(|e| e.to_string())?;
            } else {
                return Err("Root must be a Compound tag".to_string());
            }
        },
        "java" => {
            let tag = typed_to_quartz(&data).map_err(|e| e)?;
            if let quartz_nbt::NbtTag::Compound(c) = tag {
                // Use explicit GzEncoder to ensure footer is written
                let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
                quartz_nbt::io::write_nbt(&mut encoder, Some(""), &c, quartz_nbt::io::Flavor::Uncompressed)
                    .map_err(|e| e.to_string())?;
                encoder.finish().map_err(|e| e.to_string())?;
                // File is sync'd when encoder drops/finishes effectively, but we can't sync file easily after move.
                // But finish() ensures GZip footer.
            } else {
                 return Err("Root must be a Compound tag".to_string());
            }
        },
        "java_uncompressed" => {
             let tag = typed_to_quartz(&data).map_err(|e| e)?;
             if let quartz_nbt::NbtTag::Compound(c) = tag {
                let mut writer = std::io::BufWriter::new(file);
                quartz_nbt::io::write_nbt(&mut writer, Some(""), &c, quartz_nbt::io::Flavor::Uncompressed)
                    .map_err(|e| e.to_string())?;
                writer.flush().map_err(|e| e.to_string())?;
             } else {
                 return Err("Root must be a Compound tag".to_string());
             }
        },
        "bedrock" => {
             let nbt = typed_to_zuri(&data).map_err(|e| e)?;
             let mut writer = std::io::BufWriter::new(file);
             nbt.write(&mut writer, zuri_nbt::encoding::LittleEndian).map_err(|e| e.to_string())?;
             writer.flush().map_err(|e| e.to_string())?;
        },
        "bedrock_header" => {
             let old_header = if backup_path.exists() {
                 let mut f = fs::File::open(&backup_path).map_err(|e| e.to_string())?;
                 let mut head = vec![0u8; 8];
                 f.read_exact(&mut head).unwrap_or_default();
                 head
             } else {
                 vec![4, 0, 0, 0, 0, 0, 0, 0] 
             };
             
             let nbt = typed_to_zuri(&data).map_err(|e| e)?;
             
             let mut writer = std::io::BufWriter::new(file);
             writer.write_all(&old_header).map_err(|e| e.to_string())?;
             nbt.write(&mut writer, zuri_nbt::encoding::LittleEndian).map_err(|e| e.to_string())?;
             writer.flush().map_err(|e| e.to_string())?;
        },
        _ => return Err("Unknown format".to_string()),
    }

    Ok(())
}

// --- Converters (Quartz) ---
fn quartz_to_typed(tag: &quartz_nbt::NbtTag) -> TypedNbt {
    match tag {
        quartz_nbt::NbtTag::Byte(v) => TypedNbt::Byte(*v),
        quartz_nbt::NbtTag::Short(v) => TypedNbt::Short(*v),
        quartz_nbt::NbtTag::Int(v) => TypedNbt::Int(*v),
        quartz_nbt::NbtTag::Long(v) => TypedNbt::Long(v.to_string()),
        quartz_nbt::NbtTag::Float(v) => TypedNbt::Float(*v),
        quartz_nbt::NbtTag::Double(v) => TypedNbt::Double(*v),
        quartz_nbt::NbtTag::String(v) => TypedNbt::String(v.clone()),
        quartz_nbt::NbtTag::ByteArray(v) => TypedNbt::ByteArray(v.to_vec()),
        quartz_nbt::NbtTag::IntArray(v) => TypedNbt::IntArray(v.to_vec()),
        quartz_nbt::NbtTag::LongArray(v) => TypedNbt::LongArray(v.iter().map(|l| l.to_string()).collect()),
        quartz_nbt::NbtTag::List(list) => {
             TypedNbt::List(list.iter().map(quartz_to_typed).collect())
        },
        quartz_nbt::NbtTag::Compound(map) => {
            let mut hmap = HashMap::new();
            for (k, v) in map.inner().iter() {
                hmap.insert(k.to_string(), quartz_to_typed(v));
            }
            TypedNbt::Compound(hmap)
        }
    }
}

fn typed_to_quartz(typed: &TypedNbt) -> Result<quartz_nbt::NbtTag, String> {
    match typed {
        TypedNbt::Byte(v) => Ok(quartz_nbt::NbtTag::Byte(*v)),
        TypedNbt::Short(v) => Ok(quartz_nbt::NbtTag::Short(*v)),
        TypedNbt::Int(v) => Ok(quartz_nbt::NbtTag::Int(*v)),
        TypedNbt::Long(v) => {
            let val = v.parse::<i64>().map_err(|_| "Invalid Long".to_string())?;
            Ok(quartz_nbt::NbtTag::Long(val))
        },
        TypedNbt::Float(v) => Ok(quartz_nbt::NbtTag::Float(*v)),
        TypedNbt::Double(v) => Ok(quartz_nbt::NbtTag::Double(*v)),
        TypedNbt::String(v) => Ok(quartz_nbt::NbtTag::String(v.clone())),
        TypedNbt::ByteArray(v) => Ok(quartz_nbt::NbtTag::ByteArray(v.clone())),
        TypedNbt::IntArray(v) => Ok(quartz_nbt::NbtTag::IntArray(v.clone())),
        TypedNbt::LongArray(v) => {
            let mut longs = Vec::new();
            for s in v {
                longs.push(s.parse::<i64>().map_err(|_| "Invalid LongArray item".to_string())?);
            }
            Ok(quartz_nbt::NbtTag::LongArray(longs))
        },
        TypedNbt::List(list) => {
             let mut tags = Vec::new();
             for item in list {
                 tags.push(typed_to_quartz(item)?);
             }
             Ok(quartz_nbt::NbtTag::List(quartz_nbt::NbtList::from(tags)))
        },
        TypedNbt::Compound(map) => {
             let mut c = quartz_nbt::NbtCompound::new();
             for (k, v) in map {
                 c.insert(k, typed_to_quartz(v)?);
             }
             Ok(quartz_nbt::NbtTag::Compound(c))
        }
    }
}

// --- Converters (Zuri) ---
fn zuri_to_typed(tag: &zuri_nbt::NBTTag) -> TypedNbt {
    match tag {
        zuri_nbt::NBTTag::Byte(v) => TypedNbt::Byte(v.0 as i8),
        zuri_nbt::NBTTag::Short(v) => TypedNbt::Short(v.0),
        zuri_nbt::NBTTag::Int(v) => TypedNbt::Int(v.0),
        zuri_nbt::NBTTag::Long(v) => TypedNbt::Long(v.0.to_string()),
        zuri_nbt::NBTTag::Float(v) => TypedNbt::Float(v.0),
        zuri_nbt::NBTTag::Double(v) => TypedNbt::Double(v.0),
        zuri_nbt::NBTTag::String(v) => TypedNbt::String(v.0.clone()),
        zuri_nbt::NBTTag::ByteArray(v) => {
            let i8_vec: Vec<i8> = v.0.iter().map(|&b| b as i8).collect();
            TypedNbt::ByteArray(i8_vec)
        },
        zuri_nbt::NBTTag::IntArray(v) => TypedNbt::IntArray(v.0.clone()),
        zuri_nbt::NBTTag::LongArray(v) => TypedNbt::LongArray(v.0.iter().map(|l| l.to_string()).collect()),
        zuri_nbt::NBTTag::List(list) => {
             TypedNbt::List(list.iter().map(zuri_to_typed).collect())
        },
        zuri_nbt::NBTTag::Compound(map) => {
            let mut hmap = HashMap::new();
            for (k, v) in map.0.iter() {
                hmap.insert(k.clone(), zuri_to_typed(v));
            }
            TypedNbt::Compound(hmap)
        }
    }
}

fn typed_to_zuri(typed: &TypedNbt) -> Result<zuri_nbt::NBTTag, String> {
    match typed {
        TypedNbt::Byte(v) => Ok(zuri_nbt::NBTTag::Byte(zuri_nbt::tag::Byte(*v as u8))),
        TypedNbt::Short(v) => Ok(zuri_nbt::NBTTag::Short(zuri_nbt::tag::Short(*v))),
        TypedNbt::Int(v) => Ok(zuri_nbt::NBTTag::Int(zuri_nbt::tag::Int(*v))),
        TypedNbt::Long(v) => {
            let val = v.parse::<i64>().map_err(|_| "Invalid Long".to_string())?;
            Ok(zuri_nbt::NBTTag::Long(zuri_nbt::tag::Long(val)))
        },
        TypedNbt::Float(v) => Ok(zuri_nbt::NBTTag::Float(zuri_nbt::tag::Float(*v))),
        TypedNbt::Double(v) => Ok(zuri_nbt::NBTTag::Double(zuri_nbt::tag::Double(*v))),
        TypedNbt::String(v) => Ok(zuri_nbt::NBTTag::String(zuri_nbt::tag::String::from(v.clone()))),
        TypedNbt::ByteArray(v) => {
            // Zuri uses u8 for byte array, Java uses i8.
            let u8_vec: Vec<u8> = v.iter().map(|&b| b as u8).collect();
            Ok(zuri_nbt::NBTTag::ByteArray(zuri_nbt::tag::ByteArray::from(u8_vec)))
        },
        TypedNbt::IntArray(v) => Ok(zuri_nbt::NBTTag::IntArray(zuri_nbt::tag::IntArray::from(v.clone()))),
        TypedNbt::LongArray(v) => {
            let mut longs = Vec::new();
            for s in v {
                longs.push(s.parse::<i64>().map_err(|_| "Invalid LongArray item".to_string())?);
            }
            Ok(zuri_nbt::NBTTag::LongArray(zuri_nbt::tag::LongArray::from(longs)))
        },
        TypedNbt::List(list) => {
             let mut tags = Vec::new();
             for item in list {
                 tags.push(typed_to_zuri(item)?);
             }
             Ok(zuri_nbt::NBTTag::List(zuri_nbt::tag::List::from(tags)))
        },
        TypedNbt::Compound(map) => {
             let mut compound = std::collections::HashMap::new();
             for (k, v) in map {
                 compound.insert(k.clone(), typed_to_zuri(v)?);
             }
             Ok(zuri_nbt::NBTTag::Compound(zuri_nbt::tag::Compound(compound)))
        }
    }
}
