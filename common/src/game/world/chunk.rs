use bevy::prelude::*;
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{
    greedy_quads, visible_block_faces, GreedyQuadsBuffer, MergeVoxel, UnitQuadBuffer,
    Voxel as MeshableVoxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use serde_big_array::BigArray;

pub const CHUNK_SIZE: u8 = 32;
pub const TOTAL_CHUNK_SIZE: u16 =
    ((CHUNK_SIZE as u16 + 1) * (CHUNK_SIZE as u16 + 1) * (CHUNK_SIZE as u16 + 1));

#[derive(Component)]
pub struct Chunk {
    pub pos: IVec3,
    pub chunk_data: RawChunk,
    pub entities: Vec<Entity>,
    pub saved_entities: Vec<String>,
    pub dirty: bool,
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voxel {
    pub value: u16,
}

impl Voxel {
    pub const EMPTY_VOXEL: Voxel = Voxel { value: 0 };
}

impl MergeVoxel for Voxel {
    type MergeValue = u16;
    type MergeValueFacingNeighbour = u16;

    #[inline]
    fn merge_value(&self) -> Self::MergeValue {
        self.value
    }
    #[inline]
    fn merge_value_facing_neighbour(&self) -> Self::MergeValueFacingNeighbour {
        self.value * 2
    }
}

impl Default for Voxel {
    fn default() -> Self {
        Self::EMPTY_VOXEL
    }
}

impl MeshableVoxel for Voxel {
    #[inline]
    fn get_visibility(&self) -> block_mesh::VoxelVisibility {
        match *self {
            Self::EMPTY_VOXEL => block_mesh::VoxelVisibility::Empty,
            _ => block_mesh::VoxelVisibility::Opaque,
        }
    }
}

pub type ChunkShape = ConstShape3u32<
    { (CHUNK_SIZE + 1) as u32 },
    { (CHUNK_SIZE + 1) as u32 },
    { (CHUNK_SIZE + 1) as u32 },
>;

#[derive(Clone, Hash, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawChunk {
    pub palette: Vec<String>, // The namespace string will also be semi-colon seperated with state data for blocks that need it
    #[serde(with = "BigArray")]
    pub voxels: [Voxel; TOTAL_CHUNK_SIZE as usize],
}

impl Default for RawChunk {
    fn default() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: [Voxel { value: 0 }; TOTAL_CHUNK_SIZE as usize],
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
    }
}

impl RawChunk {
    // Very important to use this for creation because of air
    pub fn new() -> RawChunk {
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels: [Voxel { value: 0 }; TOTAL_CHUNK_SIZE as usize],
        };
        raw_chunk.palette.push("air".to_string());
        raw_chunk
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
            if let Some(block_data) = old_vec.get(self.voxels[i].value as usize) {
                if let Some(new_index) = self.get_index_for_state(block_data) {
                    self.voxels[i] = Voxel {
                        value: new_index as u16,
                    }; // TODO: Transluency
                } else {
                    self.voxels[i] = Voxel { value: 0 };
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
            let index = ChunkShape::linearize([pos.x, pos.y, pos.z]) as usize;
            if let Some(block_type) = self.get_index_for_state(&block_data) {
                self.voxels[index] = Voxel {
                    value: block_type as u16,
                }; // TODO: Set translucent based off of block
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
            let index = ChunkShape::linearize([pos.x, pos.y, pos.z]) as usize;
            self.get_state_for_index(self.voxels[index].value as usize)
                .map(|block_state| block_state)
        } else {
            warn!("Voxel position outside of this chunks bounds");
            None
        }
    }
}
