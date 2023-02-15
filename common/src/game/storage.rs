use std::collections::HashMap;

use serde::{Serialize, Deserialize};

pub enum AiType {
    walk,
    fly,
    swim
}

pub enum BreakTool {
    shovel,
    pickaxe,
    axe
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockType {
    namespace: String,
    block_name: String,
    textures: HashMap<String, String>,
    model: Option<String>, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    interactable: bool,
    friction: f32,
    break_time: f32,
    break_tool: BreakTool,
    walk_sound: Option<String>,
    break_sound: Option<String>,
    block_script: Option<String>
}


#[derive(Serialize, Deserialize, Debug)]
pub struct EntityType {
    namespace: String,
    entity_name: String,
    model: String, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    friction: f32,
    attack: u32,
    entity_script: Option<String>,
    interactable: bool,
    ai_type: AiType
}