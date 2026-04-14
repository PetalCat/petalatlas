#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use crate::java_assets::{extract_colors_from_jar, detect_java_installation};

    #[test]
    fn list_jars() {
        match detect_java_installation() {
            Ok(caps) => {
                println!("Detected Capabilities: {:?}", caps);
                if let Some(path) = caps.jar_path {
                    println!("Found JAR: {}", path);
                } else {
                    println!("No JAR found via auto-detection");
                }
            },
            Err(e) => println!("Error detecting java: {}", e),
        }
    }

    #[test]
    fn generate_colors_from_env() {
        let mut jar_path = std::env::var("MC_JAR_PATH").unwrap_or_default();
        
        // Try reading from .env file if env var is not set
        if jar_path.is_empty() {
             let env_file = Path::new("../.env");
             if env_file.exists() {
                 if let Ok(content) = fs::read_to_string(env_file) {
                     for line in content.lines() {
                         if let Some(rest) = line.trim().strip_prefix("MC_JAR_PATH=") {
                             jar_path = rest.trim_matches('"').trim_matches('\'').to_string();
                             println!("Loaded MC_JAR_PATH from .env");
                             break;
                         }
                     }
                 }
             }
        }

        if jar_path.is_empty() {
            println!("Skipping color generation: MC_JAR_PATH not set in environment or .env");
            return;
        }

        let output_path = std::env::current_dir()
            .unwrap()
            .join("data/block_colors.json");
            
        // Ensure data dir exists
        if let Some(parent) = output_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        println!("Generating colors from JAR: {}", jar_path);
        
        // Use a dummy path for cache to test writing
        // let cache_dir = std::env::temp_dir().join("mcworlds_test_cache");
        // extract_colors_from_jar(&jar_path, Some(&cache_dir)) ...
        // For now, passing None as we want to update the JSONs primarily.
        
        match extract_colors_from_jar(&jar_path, None) {
            Ok((block_colors, block_textures)) => {
                // Save Colors
                let json = serde_json::to_string_pretty(&block_colors)
                    .expect("Failed to serialize colors");
                fs::write(&output_path, json)
                    .expect("Failed to write colors");
                
                // Save Textures Map
                let output_tex_path = output_path.parent().unwrap().join("block_textures.json");
                let tex_json = serde_json::to_string_pretty(&block_textures)
                    .expect("Failed to serialize textures");
                fs::write(&output_tex_path, tex_json)
                    .expect("Failed to write textures");

                println!("Success: Generated data for {} blocks.", block_colors.len());
            },
            Err(e) => panic!("Failed to generate colors: {}", e),
        }
    }
}
