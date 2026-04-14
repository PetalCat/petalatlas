use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtractedBiome {
    pub id: String,
    pub temperature: f32,
    pub downfall: f32,
    pub water_color: Option<i32>,
    pub grass_color: Option<i32>,
    pub foliage_color: Option<i32>,
}
