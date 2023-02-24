use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_rapier3d::prelude::{Collider, ComputedColliderShape};
use common::game::world::chunk::{flatten_coord, Chunk, RawChunk, VoxelVisibility, CHUNK_SIZE};

use crate::states::{game::world::chunk::RenderedChunk, loading::LoadableAssets};

pub struct CreateChunkEvent {
    pub pos: IVec3,
    pub raw_chunk: RawChunk,
}

pub struct UpdateChunkEvent {
    pub pos: IVec3,
}

pub struct MeshChunkEvent {
    pub raw_chunk: RawChunk, //Temporary
    pub pos: IVec3,
}

pub struct ChunkMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub ao: Vec<u8>,
}

//IMPL Enum or struct that holds different geometry types and the vertex information for every face side ie up down left right

pub fn build_mesh(
    mut commands: Commands,
    mut event: EventReader<MeshChunkEvent>,
    mut loadable_assets: ResMut<LoadableAssets>,
    texture_atlas: Res<Assets<TextureAtlas>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let block_atlas = texture_atlas.get(&loadable_assets.block_atlas).unwrap();
    // 0 and CHUNK_SIZE_PADDED dont get built into the mesh itself its data for meshing from other chunks this is just one solution
    // TODO: Redo a lot of this code but for now just want a working implementation. The ao and custom geometry are the things I think need the most looking at
    for evt in event.iter() {
        let chunk = evt.raw_chunk.clone();
        let mut res = ChunkMesh {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            ao: Vec::new(),
            uvs: Vec::new(),
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            // Front face
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y + 1, z + 1)) as usize]
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y + 1, z)) as usize]
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            // Back face
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y + 1, z - 1)) as usize]
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            // East
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z)) as usize]
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            // Top face
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y + 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y + 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y + 1, z)) as usize]
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
                            let texture_index = block_atlas.get_texture_index(
                                &loadable_assets
                                    .block_textures
                                    .get(
                                        &evt.raw_chunk
                                            .get_state_for_index(chunk.voxels[index].value as usize)
                                            .unwrap(),
                                    )
                                    .unwrap()[0],
                            );
                            let face_coords = calculate_coords(
                                texture_index.unwrap(),
                                Vec2::new(16.0, 16.0),
                                block_atlas.size,
                            );
                            res.uvs.push(face_coords[0]);
                            res.uvs.push(face_coords[1]);
                            res.uvs.push(face_coords[2]);
                            res.uvs.push(face_coords[3]);
                            // Bottom face
                            let ao_neighbours = [
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z + 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x - 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z - 1)) as usize]
                                        .value as usize,
                                ),
                                chunk.get_visibility(
                                    chunk.voxels
                                        [flatten_coord(UVec3::new(x + 1, y - 1, z)) as usize]
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
        let finalao = ao_convert(res.ao, res.positions.len());
        let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, res.positions.clone());
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, res.normals);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, res.uvs);
        render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, finalao);
        render_mesh.set_indices(Some(Indices::U32(res.indices.clone())));
        let collider = if res.positions.len() >= 4 {
            Collider::from_bevy_mesh(&render_mesh.clone(), &ComputedColliderShape::TriMesh)
                .unwrap_or_default()
        } else {
            Collider::cuboid(0.0, 0.0, 0.0)
        };
        commands.spawn(RenderedChunk {
            collider,
            chunk: Chunk {
                chunk_data: evt.raw_chunk.clone(),
                pos: evt.pos.into(),
                dirty: true,
                entities: Vec::new(),
                saved_entities: Vec::new(),
            },
            mesh: PbrBundle {
                mesh: meshes.add(render_mesh.clone()),
                material: materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(
                        texture_atlas
                            .get(&loadable_assets.block_atlas)
                            .unwrap()
                            .texture
                            .clone(),
                    ),
                    alpha_mode: AlphaMode::Mask(1.0),
                    perceptual_roughness: 1.0,
                    ..default()
                }),
                transform: Transform::from_translation(Vec3::new(
                    (evt.pos[0] * (CHUNK_SIZE - 2) as i32) as f32,
                    (evt.pos[1] * (CHUNK_SIZE - 2) as i32) as f32,
                    (evt.pos[2] * (CHUNK_SIZE - 2) as i32) as f32,
                )),
                ..Default::default()
            },
        });
        // This is stupid and awful so ill come back to semi transparent objects
        // cmd2.spawn(PbrBundle {
        //     mesh: meshes.add(render_mesh),
        //     material: materials.add(StandardMaterial {
        //         base_color: Color::WHITE,
        //         // base_color_texture: Some(texture_handle.0.clone()),
        //         alpha_mode: AlphaMode::Blend,
        //         perceptual_roughness: 1.0,
        //         ..default()
        //     }),
        //     transform: Transform::from_translation(Vec3::new(
        //         (pos[0] * (CHUNK_SIZE / 2) as i32) as f32,
        //         (pos[1] * (CHUNK_SIZE / 2) as i32) as f32,
        //         (pos[2] * (CHUNK_SIZE / 2) as i32) as f32,
        //     )),
        //     ..Default::default()
        // });
    }
}

pub fn calculate_coords(index: usize, tile_size: Vec2, tilesheet_size: Vec2) -> [[f32; 2]; 4] {
    let mut face_tex = [[0.0; 2]; 4];
    let mut index = index as f32;
    // We need to start at 1.0 for calculations
    index += 1.0;
    let max_y = (tile_size.y) / tilesheet_size.y;
    face_tex[0][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[0][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[0][1] = 0.0;
    face_tex[1][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[1][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[1][1] = 0.0;
    face_tex[2][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[2][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[2][1] = max_y;
    face_tex[3][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[3][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[3][1] = max_y;
    face_tex
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

// TODO: move this out just testing rn
fn ao_convert(ao: Vec<u8>, num_vertices: usize) -> Vec<[f32; 4]> {
    let mut res = Vec::with_capacity(num_vertices);
    for value in ao {
        match value {
            0 => res.extend_from_slice(&[[0.3, 0.3, 0.3, 1.0]]),
            1 => res.extend_from_slice(&[[0.5, 0.5, 0.5, 1.0]]),
            2 => res.extend_from_slice(&[[0.75, 0.75, 0.75, 1.0]]),
            _ => res.extend_from_slice(&[[1.0, 1.0, 1.0, 1.0]]),
        }
    }
    res
}
