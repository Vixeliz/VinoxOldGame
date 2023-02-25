use std::collections::HashMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use serde_big_array::BigArray;
use strum_macros::EnumString;

use crate::game::storage::{BlockType, EntityType};

pub const CHUNK_SIZE: u32 = 32;
pub const CHUNK_SIZE_PADDED: u32 = CHUNK_SIZE + 1;
pub const TOTAL_CHUNK_SIZE: u32 = CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED;

#[derive(Resource, Default)]
pub struct LoadableTypes {
    pub entities: HashMap<String, EntityType>,
    pub blocks: HashMap<String, BlockType>,
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

#[derive(Debug, PartialEq, EnumString, Default)]
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
pub struct ChunkComp {
    pub pos: IVec3,
    pub chunk_data: RawChunk,
    pub entities: Vec<Entity>,
    pub saved_entities: Vec<String>,
    pub dirty: bool,
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

impl<'s> Voxel for VoxelType {
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
    #[serde(with = "BigArray")]
    pub voxels: [u16; TOTAL_CHUNK_SIZE as usize],
}

impl<'s> Default for RawChunk {
    fn default() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: [0; TOTAL_CHUNK_SIZE as usize],
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
    }
}

impl Chunk for RawChunk {
    type Output = VoxelType;

    const X: usize = CHUNK_SIZE_PADDED as usize;
    const Y: usize = CHUNK_SIZE_PADDED as usize;
    const Z: usize = CHUNK_SIZE_PADDED as usize;

    fn get(&self, x: u32, y: u32, z: u32, loadable_types: &LoadableTypes) -> Self::Output {
        self.get_voxel(
            RawChunk::linearize(UVec3::new(x, y, z)) as u16,
            loadable_types,
        )
    }
}

impl RawChunk {
    // Very important to use this for creation because of air
    pub fn new() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: [0; TOTAL_CHUNK_SIZE as usize],
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
    }

    pub fn get_voxel(&self, index: u16, loadable_types: &LoadableTypes) -> VoxelType {
        let block_state = self
            .get_state_for_index(self.voxels[index as usize] as usize)
            .unwrap();
        if block_state.eq("air") {
            VoxelType::Empty(0)
        } else {
            let voxel_visibility = loadable_types.blocks.get(&block_state).unwrap().visibility;
            match voxel_visibility {
                VoxelVisibility::Empty => VoxelType::Empty(index),
                VoxelVisibility::Opaque => VoxelType::Opaque(index),
                VoxelVisibility::Transparent => VoxelType::Transparent(index),
            }
        }
    }

    pub fn get_index_for_state(&self, block_data: &String) -> Option<usize> {
        self.palette
            .iter()
            .position(|i| i.eq(block_data))
            .map(|index| index)
    }

    pub fn get_state_for_index(&self, index: usize) -> Option<String> {
        self.palette.get(index).map(|state| state.to_owned())
    }

    // This is most likely a VERY awful way to handle this however for now I just want a working solution ill
    // rewrite this if it causes major performance issues
    pub fn update_chunk_pal(&mut self, old_vec: &Vec<String>) {
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
        if let Some(id) = self.get_index_for_state(block_data) {
            warn!("Block data: {}, already exist!", block_data);
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
    // This actual chunks data starts at 1,1,1 and ends at chunk_size - 1
    pub fn set_block(&mut self, pos: UVec3, block_data: String) {
        if pos.x > 0
            && pos.x < (CHUNK_SIZE) as u32
            && pos.y > 0
            && pos.y < (CHUNK_SIZE) as u32
            && pos.z > 0
            && pos.z < (CHUNK_SIZE) as u32
        {
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
        } else {
            warn!("Voxel position outside of this chunks bounds");
        }
    }
    pub fn get_block(&mut self, pos: UVec3) -> Option<String> {
        if pos.x > 0
            && pos.x < (CHUNK_SIZE) as u32
            && pos.y > 0
            && pos.y < (CHUNK_SIZE) as u32
            && pos.z > 0
            && pos.z < (CHUNK_SIZE) as u32
        {
            let index = RawChunk::linearize(pos);
            self.get_state_for_index(self.voxels[index] as usize)
                .map(|block_state| block_state)
        } else {
            warn!("Voxel position outside of this chunks bounds");
            None
        }
    }
    pub fn get_visibility(&self, index: usize) -> VoxelVisibility {
        if index == 0 {
            // When we switch to events we will actually get the visibility from the files
            VoxelVisibility::Empty
        } else {
            VoxelVisibility::Opaque
        }
    }
}
