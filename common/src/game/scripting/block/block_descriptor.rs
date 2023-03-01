use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockDescriptor {
    pub namespace: String,
    pub block_name: String,
    pub textures: HashMap<String, String>,
    pub interactable: bool,
    pub friction: f32,
    pub break_time: f32,
    pub break_tool: String,
    pub walk_sound: Option<String>,
    pub break_sound: Option<String>,
    pub block_script: Option<String>,
    pub visibility: String,
    pub block_geometry: String,
    pub light_val: u8,
}
