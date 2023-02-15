use std::collections::HashMap;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockDescriptor {
    namespace: String,
    block_name: String,
    textures: HashMap<String, String>,
    model: Option<String>, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    interactable: bool,
    friction: f32,
    break_time: f32,
    break_tool: String,
    walk_sound: Option<String>,
    break_sound: Option<String>,
    block_script: Option<String>
}