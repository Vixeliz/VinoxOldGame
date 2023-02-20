use crate::game::scripting::block::block_descriptor::BlockDescriptor;
use crate::game::scripting::entity::entity_descriptor::EntityDescriptor;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::EnumString;

#[derive(Debug, PartialEq, EnumString, Default)]
pub enum AiType {
    #[strum(ascii_case_insensitive)]
    #[default]
    Walk,
    #[strum(ascii_case_insensitive)]
    Fly,
    #[strum(ascii_case_insensitive)]
    Swim,
}

#[derive(Debug, PartialEq, EnumString, Default)]
pub enum BreakTool {
    #[strum(ascii_case_insensitive)]
    Shovel,
    #[strum(ascii_case_insensitive)]
    #[default]
    Pickaxe,
    #[strum(ascii_case_insensitive)]
    Axe,
}

#[derive(Debug)]
pub struct BlockType {
    pub namespace: String,
    pub block_name: String,
    pub textures: HashMap<String, String>,
    pub model: Option<String>, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    pub interactable: bool,
    pub friction: f32,
    pub break_time: f32,
    pub break_tool: BreakTool,
    pub walk_sound: Option<String>,
    pub break_sound: Option<String>,
    pub block_script: Option<String>,
    pub opaque: bool,
}

#[derive(Debug)]
pub struct EntityType {
    pub namespace: String,
    pub entity_name: String,
    pub model: String, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    pub friction: f32,
    pub attack: u32,
    pub entity_script: Option<String>,
    pub interactable: bool,
    pub ai_type: AiType,
}

pub fn convert_block(block_descriptor: Vec<BlockDescriptor>) -> HashMap<String, BlockType> {
    let mut result = HashMap::new();
    for raw_block in block_descriptor {
        result.insert(
            (raw_block.namespace.to_owned() + raw_block.block_name.as_str()).to_string(),
            BlockType {
                namespace: raw_block.namespace,
                block_name: raw_block.block_name,
                textures: raw_block.textures,
                model: raw_block.model,
                interactable: raw_block.interactable,
                friction: raw_block.friction,
                break_time: raw_block.break_time,
                break_tool: BreakTool::from_str(raw_block.break_tool.as_str()).unwrap_or_default(),
                walk_sound: raw_block.walk_sound,
                break_sound: raw_block.break_sound,
                block_script: raw_block.block_script,
                opaque: raw_block.opaque,
            },
        );
    }
    result
}

pub fn convert_entity(entity_descriptor: Vec<EntityDescriptor>) -> HashMap<String, EntityType> {
    HashMap::new()
}
