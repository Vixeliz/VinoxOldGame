use bevy::prelude::*;
use common::game::world::chunk::{flatten_coord, RawChunk, VoxelVisibility, CHUNK_SIZE};

pub struct CreateChunkEvent {}

pub struct ChunkMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub ao: Vec<u8>,
}

//IMPL Enum or struct that holds different geometry types and the vertex information for every face side ie up down left right

pub fn build_mesh(chunk: &RawChunk) -> ChunkMesh {
    // 0 and CHUNK_SIZE_PADDED dont get built into the mesh itself its data for meshing from other chunks this is just one solution
    // TODO: Redo a lot of this code but for now just want a working implementation. The ao and custom geometry are the things I think need the most looking at
    let mut res = ChunkMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        ao: Vec::new(),
    };

    for x in 1..CHUNK_SIZE {
        for y in 1..CHUNK_SIZE {
            for z in 1..CHUNK_SIZE {
                let index = flatten_coord(UVec3::new(x, y, z));
                // Make sure this voxel is solid since we dont need to mesh air
                if chunk.get_visibility(chunk.voxels[index].value as usize)
                    == VoxelVisibility::Opaque
                {
                    let neighbors = [
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x, y, z + 1))].value as usize,
                        ),
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z))].value as usize,
                        ),
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x, y, z - 1))].value as usize,
                        ),
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z))].value as usize,
                        ),
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z))].value as usize,
                        ),
                        chunk.get_visibility(
                            chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z))].value as usize,
                        ),
                    ]; // 0 is north, 1 can be west, 2 south, 3 east, 4 up, 5 down, south west 6, south east 7, north west 8, south east 9

                    if neighbors[0] == VoxelVisibility::Empty
                        || neighbors[0] == VoxelVisibility::Translucent
                    {
                        // Front face
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[3]);
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 3);
                        res.indices.push(start + 2);
                        res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                        res.normals.push([0.0, 0.0, -1.0]);
                        res.normals.push([0.0, 0.0, -1.0]);
                        res.normals.push([0.0, 0.0, -1.0]);
                        res.normals.push([0.0, 0.0, -1.0]);
                    }
                    if neighbors[1] == VoxelVisibility::Empty
                        || neighbors[1] == VoxelVisibility::Translucent
                    {
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y - 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y + 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[3]);
                        // West
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 3);
                        res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                        res.normals.push([1.0, 0.0, 0.0]);
                        res.normals.push([1.0, 0.0, 0.0]);
                        res.normals.push([1.0, 0.0, 0.0]);
                        res.normals.push([1.0, 0.0, 0.0]);
                    }
                    if neighbors[2] == VoxelVisibility::Empty
                        || neighbors[2] == VoxelVisibility::Translucent
                    {
                        // Back face
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[3]);
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 3);
                        res.positions.push([x as f32, y as f32, z as f32]);
                        res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                        res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                        res.normals.push([0.0, 0.0, 1.0]);
                        res.normals.push([0.0, 0.0, 1.0]);
                        res.normals.push([0.0, 0.0, 1.0]);
                        res.normals.push([0.0, 0.0, 1.0]);
                    }
                    if neighbors[3] == VoxelVisibility::Empty
                        || neighbors[3] == VoxelVisibility::Translucent
                    {
                        // East
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y - 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y + 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[3]);
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 3);
                        res.indices.push(start + 2);
                        res.positions.push([x as f32, y as f32, z as f32]);
                        res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                        res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                        res.positions
                            .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                        res.normals.push([-1.0, 0.0, 0.0]);
                        res.normals.push([-1.0, 0.0, 0.0]);
                        res.normals.push([-1.0, 0.0, 0.0]);
                        res.normals.push([-1.0, 0.0, 0.0]);
                    }
                    if neighbors[4] == VoxelVisibility::Empty
                        || neighbors[4] == VoxelVisibility::Translucent
                    {
                        // Top face
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y + 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y + 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y + 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[3]);
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 3);
                        res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                        res.positions
                            .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                        res.normals.push([0.0, 1.0, 0.0]);
                        res.normals.push([0.0, 1.0, 0.0]);
                        res.normals.push([0.0, 1.0, 0.0]);
                        res.normals.push([0.0, 1.0, 0.0]);
                    }
                    if neighbors[5] == VoxelVisibility::Empty
                        || neighbors[5] == VoxelVisibility::Translucent
                    {
                        // Bottom face
                        let ao_neighbours = [
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x - 1, y - 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels[flatten_coord(UVec3::new(x + 1, y - 1, z)) as usize]
                                    .value as usize,
                            ),
                            chunk.get_visibility(
                                chunk.voxels
                                    [flatten_coord(UVec3::new(x + 1, y - 1, z + 1)) as usize]
                                    .value as usize,
                            ),
                        ];
                        let ao_result = calculate_ao(&ao_neighbours);
                        res.ao.push(ao_result[0]);
                        res.ao.push(ao_result[1]);
                        res.ao.push(ao_result[2]);
                        res.ao.push(ao_result[3]);
                        let start = res.positions.len() as u32;
                        res.indices.push(start);
                        res.indices.push(start + 1);
                        res.indices.push(start + 2);
                        res.indices.push(start + 1);
                        res.indices.push(start + 3);
                        res.indices.push(start + 2);
                        res.positions.push([x as f32, y as f32, z as f32]);
                        res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                        res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                        res.positions
                            .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                        res.normals.push([0.0, -1.0, 0.0]);
                        res.normals.push([0.0, -1.0, 0.0]);
                        res.normals.push([0.0, -1.0, 0.0]);
                        res.normals.push([0.0, -1.0, 0.0]);
                    }
                }
            }
        }
    }
    res
}

pub fn calculate_ao(neighbours: &[VoxelVisibility; 8]) -> [u8; 4] {
    let mut ao = [0; 4];
    if neighbours[0] == VoxelVisibility::Opaque && neighbours[2] == VoxelVisibility::Opaque {
        ao[1] = 0;
    } else if neighbours[1] == VoxelVisibility::Opaque
        && (neighbours[0] == VoxelVisibility::Opaque || neighbours[2] == VoxelVisibility::Opaque)
    {
        ao[1] = 1;
    } else if neighbours[0] == VoxelVisibility::Opaque
        || neighbours[1] == VoxelVisibility::Opaque
        || neighbours[2] == VoxelVisibility::Opaque
    {
        ao[1] = 2;
    } else {
        ao[1] = 3;
    }
    if neighbours[2] == VoxelVisibility::Opaque && neighbours[4] == VoxelVisibility::Opaque {
        ao[0] = 0;
    } else if neighbours[3] == VoxelVisibility::Opaque
        && (neighbours[2] == VoxelVisibility::Opaque || neighbours[4] == VoxelVisibility::Opaque)
    {
        ao[0] = 1;
    } else if neighbours[2] == VoxelVisibility::Opaque
        || neighbours[3] == VoxelVisibility::Opaque
        || neighbours[4] == VoxelVisibility::Opaque
    {
        ao[0] = 2;
    } else {
        ao[0] = 3;
    }
    if neighbours[4] == VoxelVisibility::Opaque && neighbours[6] == VoxelVisibility::Opaque {
        ao[2] = 0;
    } else if neighbours[5] == VoxelVisibility::Opaque
        && (neighbours[4] == VoxelVisibility::Opaque || neighbours[6] == VoxelVisibility::Opaque)
    {
        ao[2] = 1;
    } else if neighbours[4] == VoxelVisibility::Opaque
        || neighbours[5] == VoxelVisibility::Opaque
        || neighbours[6] == VoxelVisibility::Opaque
    {
        ao[2] = 2;
    } else {
        ao[2] = 3;
    }
    if neighbours[6] == VoxelVisibility::Opaque && neighbours[0] == VoxelVisibility::Opaque {
        ao[3] = 0;
    } else if neighbours[7] == VoxelVisibility::Opaque
        && (neighbours[6] == VoxelVisibility::Opaque || neighbours[0] == VoxelVisibility::Opaque)
    {
        ao[3] = 1;
    } else if neighbours[6] == VoxelVisibility::Opaque
        || neighbours[7] == VoxelVisibility::Opaque
        || neighbours[0] == VoxelVisibility::Opaque
    {
        ao[3] = 2;
    } else {
        ao[3] = 3;
    }
    ao
}
