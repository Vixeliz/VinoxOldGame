use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockDescriptor {
    pub namespace: String,
    pub block_name: String,
    pub textures: HashMap<String, String>,
    pub model: Option<String>, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    pub interactable: bool,
    pub friction: f32,
    pub break_time: f32,
    pub break_tool: String,
    pub walk_sound: Option<String>,
    pub break_sound: Option<String>,
    pub block_script: Option<String>,
    pub visibility: String,
    pub block_geometry: String,
}
