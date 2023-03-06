use std::collections::HashMap;

use bevy::prelude::*;

use serde::{Deserialize, Serialize};

use serde_big_array::Array;
use strum_macros::EnumString;

use crate::game::storage::{BlockType, EntityType};

pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_SIZE_PADDED: u32 = CHUNK_SIZE + 2;
pub const CHUNK_BOUND: u32 = CHUNK_SIZE + 1;
pub const TOTAL_CHUNK_SIZE: u32 = CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED;
pub const TOTAL_CHUNK_USIZE: usize = TOTAL_CHUNK_SIZE as usize;

#[derive(Component, Default)]
pub struct RemoveChunk;

#[derive(Resource, Default, Clone)]
pub struct LoadableTypes {
    pub entities: HashMap<String, EntityType>,
    pub blocks: HashMap<String, BlockType>,
}

#[derive(Resource, Default)]
pub struct CurrentChunks {
    pub chunks: HashMap<IVec3, Entity>,
}

impl CurrentChunks {
    pub fn insert_entity(&mut self, pos: IVec3, entity: Entity) {
        self.chunks.insert(pos, entity);
    }

    pub fn remove_entity(&mut self, pos: IVec3) -> Option<Entity> {
        self.chunks.remove(&pos)
    }

    pub fn get_entity(&self, pos: IVec3) -> Option<Entity> {
        self.chunks.get(&pos).copied()
    }
    pub fn all_neighbors_exist(&self, pos: IVec3, _min_bound: IVec2, _max_bound: IVec2) -> bool {
        self.chunks.contains_key(&(pos + IVec3::new(0, 1, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, -1, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(1, 0, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(-1, 0, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, 0, 1)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, 0, -1)))
    }
}

#[derive(Default, Resource)]
pub struct ViewDistance {
    pub horizontal: i32,
    pub vertical: i32,
}

#[derive(Default, Resource)]
pub struct SimulationDistance {
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

pub trait Chunk {
    type Output;

    const X: usize;
    const Y: usize;
    const Z: usize;

    fn size() -> usize {
        Self::X * Self::Y * Self::Z
    }

    fn linearize(pos: UVec3) -> usize {
        let x = pos.x as usize;
        let y = pos.y as usize;
        let z = pos.z as usize;
        x + (y * Self::X) + (z * Self::X * Self::Y)
    }

    fn delinearize(mut index: usize) -> (u32, u32, u32) {
        let z = index / (Self::X * Self::Y);
        index -= z * (Self::X * Self::Y);

        let y = index / Self::X;
        index -= y * Self::X;

        let x = index;

        (x as u32, y as u32, z as u32)
    }

    fn get(&self, x: u32, y: u32, z: u32, loadable_types: &LoadableTypes) -> Self::Output;
}

#[derive(Debug, PartialEq, EnumString, Default, Eq, Clone, Copy)]
pub enum VoxelVisibility {
    #[default]
    #[strum(ascii_case_insensitive)]
    Opaque,
    #[strum(ascii_case_insensitive)]
    Transparent,
    #[strum(ascii_case_insensitive)]
    Empty,
}

#[derive(Debug, PartialEq, EnumString, Default, Clone)]
pub enum GeometryType {
    #[default]
    #[strum(ascii_case_insensitive)]
    Block,
    #[strum(ascii_case_insensitive)]
    VerticalSlab,
    #[strum(ascii_case_insensitive)]
    HorizontalSlab,
    #[strum(ascii_case_insensitive)]
    Stairs,
}

#[derive(Component)]
pub struct ChunkPos(pub IVec3);

#[derive(Component)]
pub struct ChunkComp {
    pub pos: ChunkPos,
    pub chunk_data: RawChunk,
    pub entities: Vec<Entity>,
    pub saved_entities: Vec<String>,
}

pub trait Voxel: Eq {
    fn visibility(&self) -> VoxelVisibility;
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoxelType {
    Empty(u16),
    Opaque(u16),
    Transparent(u16),
}

impl Default for VoxelType {
    fn default() -> VoxelType {
        Self::Empty(0)
    }
}

impl Voxel for VoxelType {
    fn visibility(&self) -> VoxelVisibility {
        match self {
            Self::Empty(_) => VoxelVisibility::Empty,
            Self::Opaque(_) => VoxelVisibility::Opaque,
            Self::Transparent(_) => VoxelVisibility::Transparent,
        }
    }
}

macro_rules! as_variant {
    ($value:expr, $variant:path) => {
        match $value {
            $variant(x) => Some(x),
            _ => None,
        }
    };
}

impl VoxelType {
    pub fn value(self) -> u16 {
        match self {
            Self::Empty(_) => as_variant!(self, VoxelType::Empty).unwrap_or(0),
            Self::Opaque(_) => as_variant!(self, VoxelType::Opaque).unwrap_or(0),
            Self::Transparent(_) => as_variant!(self, VoxelType::Transparent).unwrap_or(0),
        }
    }
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawChunk {
    pub palette: Vec<String>, // The namespace string will also be semi-colon seperated with state data for blocks that need it
    pub voxels: Box<Array<u16, TOTAL_CHUNK_USIZE>>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, Component)]
pub struct LightChunk {
    pub voxels: [u8; TOTAL_CHUNK_USIZE],
}

impl LightChunk {
    pub fn get_voxel(&self, x: u32, y: u32, z: u32) -> u8 {
        let index = RawChunk::linearize(UVec3::new(x, y, z));
        self.voxels[index]
    }
    pub fn calculate_light(
        &mut self,
        raw_chunk: &RawChunk,
        _neighbors: [&RawChunk; 6],
        loadable_types: &LoadableTypes,
    ) {
        for i in 0..raw_chunk.voxels.len() {
            let (x, y, z) = RawChunk::delinearize(i);
            if (x > 0 && x < CHUNK_BOUND)
                && (y > 0 && y < CHUNK_BOUND)
                && (z > 0 && z < CHUNK_BOUND)
            {
                if let Some(light_val) = raw_chunk.get_data(i, loadable_types) {
                    let light_val = light_val.light_val;
                    if light_val > 0 {
                        for _l in 0..light_val {}
                    }
                }
            }
        }
    }
}

impl Default for RawChunk {
    fn default() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: Box::default(),
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
    }
}

pub fn world_to_chunk(pos: Vec3) -> IVec3 {
    IVec3::new(
        (pos.x / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.y / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.z / (CHUNK_SIZE as f32)).floor() as i32,
    )
}

pub fn world_to_voxel(voxel_pos: Vec3) -> (IVec3, UVec3) {
    (
        world_to_chunk(voxel_pos),
        UVec3::new(
            voxel_pos.x.rem_euclid(CHUNK_SIZE as f32).floor() as u32 + 1,
            voxel_pos.y.rem_euclid(CHUNK_SIZE as f32).floor() as u32 + 1,
            voxel_pos.z.rem_euclid(CHUNK_SIZE as f32).floor() as u32 + 1,
        ),
    )
}

impl Chunk for RawChunk {
    type Output = VoxelType;

    const X: usize = CHUNK_SIZE_PADDED as usize;
    const Y: usize = CHUNK_SIZE_PADDED as usize;
    const Z: usize = CHUNK_SIZE_PADDED as usize;

    fn get(&self, x: u32, y: u32, z: u32, loadable_types: &LoadableTypes) -> Self::Output {
        self.get_voxel(RawChunk::linearize(UVec3::new(x, y, z)), loadable_types)
    }
}

impl RawChunk {
    // Very important to use this for creation because of air
    pub fn new() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: Box::default(),
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
    }

    pub fn get_voxel(&self, index: usize, loadable_types: &LoadableTypes) -> VoxelType {
        let block_state = self
            .get_state_for_index(self.voxels[index] as usize)
            .unwrap();
        let block_id = self.get_index_for_state(&block_state).unwrap() as u16;
        if block_state.eq("air") {
            VoxelType::Empty(0)
        } else {
            let voxel_visibility = loadable_types.blocks.get(&block_state).unwrap().visibility;
            match voxel_visibility {
                VoxelVisibility::Empty => VoxelType::Empty(block_id),
                VoxelVisibility::Opaque => VoxelType::Opaque(block_id),
                VoxelVisibility::Transparent => VoxelType::Transparent(block_id),
            }
        }
    }

    pub fn get_data(&self, index: usize, loadable_types: &LoadableTypes) -> Option<BlockType> {
        let block_state = self
            .get_state_for_index(self.voxels[index] as usize)
            .unwrap();
        let _block_id = self.get_index_for_state(&block_state).unwrap() as u16;
        if block_state.eq("air") {
            None
        } else {
            Some(loadable_types.blocks.get(&block_state).unwrap().clone())
        }
    }

    pub fn get_index_for_state(&self, block_data: &String) -> Option<usize> {
        self.palette.iter().position(|i| i.eq(block_data))
    }

    pub fn get_state_for_index(&self, index: usize) -> Option<String> {
        self.palette.get(index).map(|state| state.to_owned())
    }

    // This is most likely a VERY awful way to handle this however for now I just want a working solution ill
    // rewrite this if it causes major performance issues
    pub fn update_chunk_pal(&mut self, old_vec: &[String]) {
        for i in 0..self.voxels.len() {
            if let Some(block_data) = old_vec.get(self.voxels[i] as usize) {
                if let Some(new_index) = self.get_index_for_state(block_data) {
                    self.voxels[i] = new_index as u16;
                } else {
                    self.voxels[i] = 0;
                }
            }
        }
    }

    pub fn add_block_state(&mut self, block_data: &String) {
        let old_vec = self.palette.clone();
        if let Some(_id) = self.get_index_for_state(block_data) {
        } else {
            self.palette.push(block_data.to_owned());
            self.update_chunk_pal(&old_vec);
        }
    }
    pub fn remove_block_state(&mut self, block_data: &String) {
        if block_data.eq(&"air".to_string()) {
            return;
        }
        let old_vec = self.palette.clone();
        if let Some(id) = self.get_index_for_state(block_data) {
            self.palette.remove(id);
            self.update_chunk_pal(&old_vec);
        } else {
            warn!("Block data: {}, doesn't exist!", block_data);
        }
    }
    // This actual chunks data starts at 1,1,1 and ends at chunk_size
    pub fn set_block(&mut self, pos: UVec3, block_data: String) {
        let index = RawChunk::linearize(pos);
        if let Some(block_type) = self.get_index_for_state(&block_data) {
            if block_type == 0 {
                self.voxels[index] = 0;
            } else {
                self.voxels[index] = block_type as u16; // Set based off of transluency
            }
        } else {
            warn!("Voxel doesn't exist");
        }
    }
    pub fn get_block(&mut self, pos: UVec3) -> Option<String> {
        let index = RawChunk::linearize(pos);
        self.get_state_for_index(self.voxels[index] as usize)
    }
}
