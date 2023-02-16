use bevy::prelude::*;
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{
    greedy_quads, visible_block_faces, GreedyQuadsBuffer, MergeVoxel, UnitQuadBuffer,
    Voxel as MeshableVoxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use indexmap::IndexMap;

pub const CHUNK_SIZE: u8 = 32;
pub const TOTAL_CHUNK_SIZE: u16 =
    ((CHUNK_SIZE as u16 + 1) * (CHUNK_SIZE as u16 + 1) * (CHUNK_SIZE as u16 + 1));

#[derive(Component)]
pub struct Chunk {
    pub pos: IVec3,
    pub chunk_data: RawChunk,
    pub entities: Vec<Entity>,
    pub dirty: bool,
}

#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq)]
pub struct Voxel((u16, bool)); // Having this bool is mildly annoying but i'm not sure of a better way to do this

impl Voxel {
    pub const EMPTY_VOXEL: Voxel = Voxel((0, false));
}

impl MergeVoxel for Voxel {
    type MergeValue = u16;
    type MergeValueFacingNeighbour = u16;

    #[inline]
    fn merge_value(&self) -> Self::MergeValue {
        self.0 .0
    }
    #[inline]
    fn merge_value_facing_neighbour(&self) -> Self::MergeValueFacingNeighbour {
        self.0 .0 * 2
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
            _ => {
                if self.0 .1 {
                    block_mesh::VoxelVisibility::Opaque
                } else {
                    block_mesh::VoxelVisibility::Translucent
                }
            }
        }
    }
}

pub type ChunkShape = ConstShape3u32<
    { (CHUNK_SIZE + 1) as u32 },
    { (CHUNK_SIZE + 1) as u32 },
    { (CHUNK_SIZE + 1) as u32 },
>;

pub struct RawChunk {
    pub palette: Vec<String>, // The namespace string will also be semi-colon seperated with state data for blocks that need it
    pub voxels: [Voxel; TOTAL_CHUNK_SIZE as usize],
}

impl RawChunk {
    pub fn new() -> Option<RawChunk> {
        Some(RawChunk {
            palette: Vec::new(),
            voxels: [Voxel((0, false)); TOTAL_CHUNK_SIZE as usize],
        })
    }

    pub fn get_index_for_state(&self, block_data: &String) -> Option<usize> {
        if let Some(index) = self.palette.iter().position(|i| i.eq(block_data)) {
            Some(index)
        } else {
            None
        }
    }

    pub fn get_state_for_index(&self, index: usize) -> Option<String> {
        if let Some(state) = self.palette.get(index) {
            Some(state.to_owned())
        } else {
            None
        }
    }

    pub fn add_block_state(&mut self, block_data: &String) {
        if let Some(id) = self.get_index_for_state(block_data) {
            warn!("Block data: {}, already exist!", block_data);
        } else {
            self.palette.push(block_data.to_owned());
        }
    }
    pub fn remove_block_state(&mut self, block_data: &String) {
        if let Some(id) = self.get_index_for_state(block_data) {
            self.palette.remove(id);
        } else {
            warn!("Block data: {}, doesn't exist!", block_data);
        }
    }
    pub fn set_voxel(&mut self, index: usize, block_data: String) {}
}

#[cfg(test)]
mod tests {
    use crate::game::world::chunk::*;
    use bevy::prelude::*;
    use std::collections::HashMap;
    #[test]
    fn chunk_type() {
        let mut voxels = [Voxel((0, false)); ChunkShape::SIZE as usize];
        for z in 1..CHUNK_SIZE {
            for y in 1..CHUNK_SIZE {
                for x in 1..CHUNK_SIZE {
                    let i = ChunkShape::linearize([x.into(), y.into(), z.into()]);
                    voxels[i as usize] = Voxel((1, true));
                }
            }
        }
        let mut raw_chunk = RawChunk {
            palette: Vec::new(),
            voxels,
        };
        raw_chunk.add_block_state(&"test".to_string());
        raw_chunk.add_block_state(&"test1".to_string());
        raw_chunk.remove_block_state(&"test".to_string());
        println!(
            "{:?}\n",
            raw_chunk.get_index_for_state(&"test1".to_string())
        );
        println!("{:?}\n", raw_chunk.get_state_for_index(0));
    }
}
