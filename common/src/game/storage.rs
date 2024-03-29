use crate::game::scripting::block::block_descriptor::BlockDescriptor;
use crate::game::scripting::entity::entity_descriptor::EntityDescriptor;
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::EnumString;

use super::world::chunk::{GeometryType, VoxelVisibility};

#[derive(Debug, PartialEq, EnumString, Default, Clone)]
pub enum AiType {
    #[strum(ascii_case_insensitive)]
    #[default]
    Walk,
    #[strum(ascii_case_insensitive)]
    Fly,
    #[strum(ascii_case_insensitive)]
    Swim,
}

#[derive(Debug, PartialEq, EnumString, Default, Clone)]
pub enum BreakTool {
    #[strum(ascii_case_insensitive)]
    Shovel,
    #[strum(ascii_case_insensitive)]
    #[default]
    Pickaxe,
    #[strum(ascii_case_insensitive)]
    Axe,
}

#[derive(Debug, Clone)]
pub struct BlockType {
    pub namespace: String,
    pub block_name: String,
    pub textures: HashMap<String, String>,
    pub interactable: bool,
    pub friction: f32,
    pub break_time: f32,
    pub break_tool: BreakTool,
    pub walk_sound: Option<String>,
    pub break_sound: Option<String>,
    pub block_script: Option<String>,
    pub visibility: VoxelVisibility,
    pub block_geometry: GeometryType,
    pub light_val: u8,
}

#[derive(Debug, Clone)]
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
                interactable: raw_block.interactable,
                friction: raw_block.friction,
                break_time: raw_block.break_time,
                break_tool: BreakTool::from_str(raw_block.break_tool.as_str()).unwrap_or_default(),
                walk_sound: raw_block.walk_sound,
                break_sound: raw_block.break_sound,
                block_script: raw_block.block_script,
                visibility: VoxelVisibility::from_str(raw_block.visibility.as_str())
                    .unwrap_or_default(),
                block_geometry: GeometryType::from_str(raw_block.block_geometry.as_str())
                    .unwrap_or_default(),
                light_val: raw_block.light_val,
            },
        );
    }
    result
}

pub fn convert_entity(_entity_descriptor: Vec<EntityDescriptor>) -> HashMap<String, EntityType> {
    HashMap::new()
}
