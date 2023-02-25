use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_rapier3d::prelude::{Collider, ComputedColliderShape};
use common::game::world::chunk::{Chunk, ChunkComp, RawChunk, Voxel, VoxelVisibility, CHUNK_SIZE};

use crate::states::{game::world::chunk::RenderedChunk, loading::LoadableAssets};

pub const EMPTY: VoxelVisibility = VoxelVisibility::Empty;
pub const OPAQUE: VoxelVisibility = VoxelVisibility::Opaque;
pub const TRANSPARENT: VoxelVisibility = VoxelVisibility::Transparent;

#[derive(Copy, Clone, Debug)]
pub struct Quad {
    pub voxel: [usize; 3],
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct QuadGroups {
    pub groups: [Vec<Quad>; 6],
}

#[derive(PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(PartialEq, Eq)]
pub struct Side {
    pub axis: Axis,
    pub positive: bool,
}

impl Side {
    pub fn new(axis: Axis, positive: bool) -> Self {
        Self { axis, positive }
    }

    pub fn normal(&self) -> [f32; 3] {
        match (&self.axis, &self.positive) {
            (Axis::X, true) => [1.0, 0.0, 0.0],   // X+
            (Axis::X, false) => [-1.0, 0.0, 0.0], // X-
            (Axis::Y, true) => [0.0, 1.0, 0.0],   // Y+
            (Axis::Y, false) => [0.0, -1.0, 0.0], // Y-
            (Axis::Z, true) => [0.0, 0.0, 1.0],   // Z+
            (Axis::Z, false) => [0.0, 0.0, -1.0], // Z-
        }
    }

    pub fn normals(&self) -> [[f32; 3]; 4] {
        [self.normal(), self.normal(), self.normal(), self.normal()]
    }
}

pub struct Face<'a> {
    side: Side,
    quad: &'a Quad,
}

impl From<usize> for Side {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::new(Axis::X, false), // X-
            1 => Self::new(Axis::X, true),  // X+
            2 => Self::new(Axis::Y, false), // Y-
            3 => Self::new(Axis::Y, true),  // Y+
            4 => Self::new(Axis::Z, false), // Z-
            5 => Self::new(Axis::Z, true),  // Z+
            _ => unreachable!(),
        }
    }
}
impl QuadGroups {
    pub fn iter(&self) -> impl Iterator<Item = Face> {
        self.groups
            .iter()
            .enumerate()
            .flat_map(|(index, quads)| quads.iter().map(move |quad| (index, quad)))
            .map(|(index, quad)| Face {
                side: index.into(),
                quad,
            })
    }
}

impl<'a> Face<'a> {
    pub fn indices(&self, start: u32) -> [u32; 6] {
        [start, start + 2, start + 1, start + 1, start + 2, start + 3]
    }

    pub fn positions(&self, voxel_size: f32) -> [[f32; 3]; 4] {
        let positions = match (&self.side.axis, &self.side.positive) {
            (Axis::X, false) => [
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
            ],
            (Axis::X, true) => [
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
            ],
            (Axis::Y, false) => [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
            (Axis::Y, true) => [
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ],
            (Axis::Z, false) => [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
            ],
            (Axis::Z, true) => [
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
        };

        let (x, y, z) = (
            (self.quad.voxel[0] - 1) as f32,
            (self.quad.voxel[1] - 1) as f32,
            (self.quad.voxel[2] - 1) as f32,
        );

        [
            [
                x * voxel_size + positions[0][0] * voxel_size,
                y * voxel_size + positions[0][1] * voxel_size,
                z * voxel_size + positions[0][2] * voxel_size,
            ],
            [
                x * voxel_size + positions[1][0] * voxel_size,
                y * voxel_size + positions[1][1] * voxel_size,
                z * voxel_size + positions[1][2] * voxel_size,
            ],
            [
                x * voxel_size + positions[2][0] * voxel_size,
                y * voxel_size + positions[2][1] * voxel_size,
                z * voxel_size + positions[2][2] * voxel_size,
            ],
            [
                x * voxel_size + positions[3][0] * voxel_size,
                y * voxel_size + positions[3][1] * voxel_size,
                z * voxel_size + positions[3][2] * voxel_size,
            ],
        ]
    }

    pub fn normals(&self) -> [[f32; 3]; 4] {
        self.side.normals()
    }

    pub fn uvs(&self, flip_u: bool, flip_v: bool) -> [[f32; 2]; 4] {
        match (flip_u, flip_v) {
            (true, true) => [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            (true, false) => [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            (false, true) => [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            (false, false) => [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
        }
    }

    pub fn ao(&self) -> [u8; 4] {
        [0; 4]
    }

    pub fn voxel(&self) -> [usize; 3] {
        self.quad.voxel
    }
}

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

// pub struct ChunkMesh {
//     pub positions: Vec<[f32; 3]>,
//     pub normals: Vec<[f32; 3]>,
//     pub uvs: Vec<[f32; 2]>,
//     pub indices: Vec<u32>,
//     pub ao: Vec<u8>,
// }

pub fn calculate_ao<C, T>(chunk: &C, current_side: Side, x: u32, y: u32, z: u32) -> [u8; 4]
where
    C: Chunk<Output = T>,
    T: Voxel,
{
    let neighbours: [T; 8];
    if current_side == Side::new(Axis::X, false) || current_side == Side::new(Axis::X, true) {
        // left or right
        neighbours = [
            chunk.get(x, y, z + 1),
            chunk.get(x, y - 1, z + 1),
            chunk.get(x, y - 1, z),
            chunk.get(x, y - 1, z - 1),
            chunk.get(x, y, z - 1),
            chunk.get(x, y + 1, z - 1),
            chunk.get(x, y + 1, z),
            chunk.get(x, y + 1, z + 1),
        ];
    } else if current_side == Side::new(Axis::Y, false) || current_side == Side::new(Axis::Y, true)
    {
        // bottom or top
        neighbours = [
            chunk.get(x, y, z + 1),
            chunk.get(x - 1, y, z + 1),
            chunk.get(x - 1, y, z),
            chunk.get(x - 1, y, z - 1),
            chunk.get(x, y, z - 1),
            chunk.get(x + 1, y, z - 1),
            chunk.get(x + 1, y, z),
            chunk.get(x + 1, y, z + 1),
        ];
    } else {
        // back or front
        neighbours = [
            chunk.get(x + 1, y, z),
            chunk.get(x + 1, y - 1, z),
            chunk.get(x, y - 1, z),
            chunk.get(x - 1, y - 1, z),
            chunk.get(x - 1, y, z),
            chunk.get(x - 1, y + 1, z),
            chunk.get(x, y + 1, z),
            chunk.get(x + 1, y + 1, z),
        ];
    }

    let mut ao = [0; 4];
    if neighbours[0].visibility() == VoxelVisibility::Opaque
        && neighbours[2].visibility() == VoxelVisibility::Opaque
    {
        ao[1] = 0;
    } else if neighbours[1].visibility() == VoxelVisibility::Opaque
        && (neighbours[0].visibility() == VoxelVisibility::Opaque
            || neighbours[2].visibility() == VoxelVisibility::Opaque)
    {
        ao[1] = 1;
    } else if neighbours[0].visibility() == VoxelVisibility::Opaque
        || neighbours[1].visibility() == VoxelVisibility::Opaque
        || neighbours[2].visibility() == VoxelVisibility::Opaque
    {
        ao[1] = 2;
    } else {
        ao[1] = 3;
    }
    if neighbours[2].visibility() == VoxelVisibility::Opaque
        && neighbours[4].visibility() == VoxelVisibility::Opaque
    {
        ao[0] = 0;
    } else if neighbours[3].visibility() == VoxelVisibility::Opaque
        && (neighbours[2].visibility() == VoxelVisibility::Opaque
            || neighbours[4].visibility() == VoxelVisibility::Opaque)
    {
        ao[0] = 1;
    } else if neighbours[2].visibility() == VoxelVisibility::Opaque
        || neighbours[3].visibility() == VoxelVisibility::Opaque
        || neighbours[4].visibility() == VoxelVisibility::Opaque
    {
        ao[0] = 2;
    } else {
        ao[0] = 3;
    }
    if neighbours[4].visibility() == VoxelVisibility::Opaque
        && neighbours[6].visibility() == VoxelVisibility::Opaque
    {
        ao[2] = 0;
    } else if neighbours[5].visibility() == VoxelVisibility::Opaque
        && (neighbours[4].visibility() == VoxelVisibility::Opaque
            || neighbours[6].visibility() == VoxelVisibility::Opaque)
    {
        ao[2] = 1;
    } else if neighbours[4].visibility() == VoxelVisibility::Opaque
        || neighbours[5].visibility() == VoxelVisibility::Opaque
        || neighbours[6].visibility() == VoxelVisibility::Opaque
    {
        ao[2] = 2;
    } else {
        ao[2] = 3;
    }
    if neighbours[6].visibility() == VoxelVisibility::Opaque
        && neighbours[0].visibility() == VoxelVisibility::Opaque
    {
        ao[3] = 0;
    } else if neighbours[7].visibility() == VoxelVisibility::Opaque
        && (neighbours[6].visibility() == VoxelVisibility::Opaque
            || neighbours[0].visibility() == VoxelVisibility::Opaque)
    {
        ao[3] = 1;
    } else if neighbours[6].visibility() == VoxelVisibility::Opaque
        || neighbours[7].visibility() == VoxelVisibility::Opaque
        || neighbours[0].visibility() == VoxelVisibility::Opaque
    {
        ao[3] = 2;
    } else {
        ao[3] = 3;
    }

    ao
}

pub fn generate_mesh<C, T>(chunk: &C) -> QuadGroups
where
    C: Chunk<Output = T>,
    T: Voxel,
{
    assert!(C::X >= 2);
    assert!(C::Y >= 2);
    assert!(C::Z >= 2);

    let mut buffer = QuadGroups::default();

    for i in 0..C::size() {
        let (x, y, z) = C::delinearize(i);

        if (x > 0 && x < (C::X - 1) as u32)
            && (y > 0 && y < (C::Y - 1) as u32)
            && (z > 0 && z < (C::Z - 1) as u32)
        {
            let voxel = chunk.get(x, y, z);

            match voxel.visibility() {
                EMPTY => continue,
                visibility => {
                    let neighbors = [
                        chunk.get(x - 1, y, z),
                        chunk.get(x + 1, y, z),
                        chunk.get(x, y - 1, z),
                        chunk.get(x, y + 1, z),
                        chunk.get(x, y, z - 1),
                        chunk.get(x, y, z + 1),
                    ];

                    for (i, neighbor) in neighbors.into_iter().enumerate() {
                        let other = neighbor.visibility();

                        let generate = match (visibility, other) {
                            (OPAQUE, EMPTY) | (OPAQUE, TRANSPARENT) | (TRANSPARENT, EMPTY) => true,

                            (TRANSPARENT, TRANSPARENT) => voxel != neighbor,

                            (_, _) => false,
                        };

                        if generate {
                            buffer.groups[i].push(Quad {
                                voxel: [x as usize, y as usize, z as usize],
                                width: 1,
                                height: 1,
                            });
                        }
                    }
                }
            }
        }
    }

    buffer
}

// pub fn generate_meshes(quads: QuadGroups) -> ChunkMesh {
//     let mesh = ChunkMesh {
//         positions: Vec::new(),
//         normals: Vec::new(),
//         indices: Vec::new(),
//         ao: Vec::new(),
//         uvs: Vec::new(),
//     };
//     for face in quads.iter() {}

//     mesh
// }

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
        let mesh_result = generate_mesh(&evt.raw_chunk);
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut ao = Vec::new();
        for face in mesh_result.iter() {
            positions.extend_from_slice(&face.positions(1.0)); // Voxel size is 1m
            indices.extend_from_slice(&face.indices(positions.len() as u32));
            normals.extend_from_slice(&face.normals());
            ao.extend_from_slice(&calculate_ao(
                &evt.raw_chunk,
                face.side,
                face.quad.voxel[0] as u32,
                face.quad.voxel[1] as u32,
                face.quad.voxel[2] as u32,
            ));

            // uvs.extend_from_slice(&face.uvs(false, true));
            let texture_index = block_atlas.get_texture_index(
                &loadable_assets
                    .block_textures
                    .get(
                        &evt.raw_chunk
                            .get_state_for_index(
                                evt.raw_chunk.voxels[RawChunk::linearize(UVec3::new(
                                    face.quad.voxel[0] as u32,
                                    face.quad.voxel[1] as u32,
                                    face.quad.voxel[2] as u32,
                                ))]
                                .value() as usize,
                            )
                            .unwrap(),
                    )
                    .unwrap()[0],
            );
            let face_coords = calculate_coords(
                texture_index.unwrap(),
                Vec2::new(16.0, 16.0),
                block_atlas.size,
            );
            uvs.push(face_coords[0]);
            uvs.push(face_coords[1]);
            uvs.push(face_coords[2]);
            uvs.push(face_coords[3]);
        }

        let final_ao = ao_convert(ao);
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

        mesh.set_indices(Some(Indices::U32(indices)));

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, final_ao);
        // let collider = if positions.len() >= 4 {
        //     Collider::from_bevy_mesh(&mesh.clone(), &ComputedColliderShape::TriMesh)
        //         .unwrap_or_default()
        // } else {
        let collider = Collider::cuboid(0.0, 0.0, 0.0);
        // };
        commands.spawn(RenderedChunk {
            collider,
            chunk: ChunkComp {
                chunk_data: evt.raw_chunk.clone(),
                pos: evt.pos.into(),
                dirty: true,
                entities: Vec::new(),
                saved_entities: Vec::new(),
            },
            mesh: PbrBundle {
                mesh: meshes.add(mesh.clone()),
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
    }
}

//IMPL Enum or struct that holds different geometry types and the vertex information for every face side ie up down left right

// pub fn old_mesh(
//     mut commands: Commands,
//     mut event: EventReader<MeshChunkEvent>,
//     mut loadable_assets: ResMut<LoadableAssets>,
//     texture_atlas: Res<Assets<TextureAtlas>>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
// ) {
//     let block_atlas = texture_atlas.get(&loadable_assets.block_atlas).unwrap();
//     // 0 and CHUNK_SIZE_PADDED dont get built into the mesh itself its data for meshing from other chunks this is just one solution
//     // TODO: Redo a lot of this code but for now just want a working implementation. The ao and custom geometry are the things I think need the most looking at
//     for evt in event.iter() {
//         let chunk = evt.raw_chunk.clone();
//         let mut res = ChunkMesh {
//             positions: Vec::new(),
//             normals: Vec::new(),
//             indices: Vec::new(),
//             ao: Vec::new(),
//             uvs: Vec::new(),
//         };

//         for x in 1..CHUNK_SIZE {
//             for y in 1..CHUNK_SIZE {
//                 for z in 1..CHUNK_SIZE {
//                     let index = RawChunk::linearize(UVec3::new(x, y, z));
//                     // Make sure this voxel is solid since we dont need to mesh air
//                     if chunk.voxels[index].visibility() == VoxelVisibility::Opaque {
//                         let neighbors = [
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x, y, z + 1))].visibility(),
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x + 1, y, z))].visibility(),
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x, y, z - 1))].visibility(),
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x - 1, y, z))].visibility(),
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x, y + 1, z))].visibility(),
//                             chunk.voxels[RawChunk::linearize(UVec3::new(x, y - 1, z))].visibility(),
//                         ]; // 0 is north, 1 can be west, 2 south, 3 east, 4 up, 5 down, south west 6, south east 7, north west 8, south east 9

//                         if neighbors[0] == VoxelVisibility::Empty
//                             || neighbors[0] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             // Front face
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[3]);
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 3);
//                             res.indices.push(start + 2);
//                             res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.normals.push([0.0, 0.0, -1.0]);
//                             res.normals.push([0.0, 0.0, -1.0]);
//                             res.normals.push([0.0, 0.0, -1.0]);
//                             res.normals.push([0.0, 0.0, -1.0]);
//                         }
//                         if neighbors[1] == VoxelVisibility::Empty
//                             || neighbors[1] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[3]);
//                             // West
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 3);
//                             res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.normals.push([1.0, 0.0, 0.0]);
//                             res.normals.push([1.0, 0.0, 0.0]);
//                             res.normals.push([1.0, 0.0, 0.0]);
//                             res.normals.push([1.0, 0.0, 0.0]);
//                         }
//                         if neighbors[2] == VoxelVisibility::Empty
//                             || neighbors[2] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             // Back face
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[3]);
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 3);
//                             res.positions.push([x as f32, y as f32, z as f32]);
//                             res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
//                             res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
//                             res.normals.push([0.0, 0.0, 1.0]);
//                             res.normals.push([0.0, 0.0, 1.0]);
//                             res.normals.push([0.0, 0.0, 1.0]);
//                             res.normals.push([0.0, 0.0, 1.0]);
//                         }

//                         if neighbors[3] == VoxelVisibility::Empty
//                             || neighbors[3] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             // East
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[3]);
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 3);
//                             res.indices.push(start + 2);
//                             res.positions.push([x as f32, y as f32, z as f32]);
//                             res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
//                             res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
//                             res.positions
//                                 .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.normals.push([-1.0, 0.0, 0.0]);
//                             res.normals.push([-1.0, 0.0, 0.0]);
//                             res.normals.push([-1.0, 0.0, 0.0]);
//                             res.normals.push([-1.0, 0.0, 0.0]);
//                         }
//                         if neighbors[4] == VoxelVisibility::Empty
//                             || neighbors[4] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             // Top face
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y + 1, z + 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[3]);
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 3);
//                             res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
//                             res.positions
//                                 .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
//                             res.normals.push([0.0, 1.0, 0.0]);
//                             res.normals.push([0.0, 1.0, 0.0]);
//                             res.normals.push([0.0, 1.0, 0.0]);
//                             res.normals.push([0.0, 1.0, 0.0]);
//                         }
//                         if neighbors[5] == VoxelVisibility::Empty
//                             || neighbors[5] == VoxelVisibility::Transparent
//                         {
//                             let texture_index = block_atlas.get_texture_index(
//                                 &loadable_assets
//                                     .block_textures
//                                     .get(
//                                         &evt.raw_chunk
//                                             .get_state_for_index(
//                                                 chunk.voxels[index].value() as usize
//                                             )
//                                             .unwrap(),
//                                     )
//                                     .unwrap()[0],
//                             );
//                             let face_coords = calculate_coords(
//                                 texture_index.unwrap(),
//                                 Vec2::new(16.0, 16.0),
//                                 block_atlas.size,
//                             );
//                             res.uvs.push(face_coords[0]);
//                             res.uvs.push(face_coords[1]);
//                             res.uvs.push(face_coords[2]);
//                             res.uvs.push(face_coords[3]);
//                             // Bottom face
//                             let ao_neighbours = [
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x - 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z - 1)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z)) as usize]
//                                     .visibility(),
//                                 chunk.voxels
//                                     [RawChunk::linearize(UVec3::new(x + 1, y - 1, z + 1)) as usize]
//                                     .visibility(),
//                             ];
//                             let ao_result = calculate_ao(&ao_neighbours);
//                             res.ao.push(ao_result[0]);
//                             res.ao.push(ao_result[1]);
//                             res.ao.push(ao_result[2]);
//                             res.ao.push(ao_result[3]);
//                             let start = res.positions.len() as u32;
//                             res.indices.push(start);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 2);
//                             res.indices.push(start + 1);
//                             res.indices.push(start + 3);
//                             res.indices.push(start + 2);
//                             res.positions.push([x as f32, y as f32, z as f32]);
//                             res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
//                             res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
//                             res.positions
//                                 .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
//                             res.normals.push([0.0, -1.0, 0.0]);
//                             res.normals.push([0.0, -1.0, 0.0]);
//                             res.normals.push([0.0, -1.0, 0.0]);
//                             res.normals.push([0.0, -1.0, 0.0]);
//                         }
//                     }
//                 }
//             }
//         }
//         let finalao = ao_convert(res.ao, res.positions.len());
//         let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);
//         render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, res.positions.clone());
//         render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, res.normals);
//         render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, res.uvs);
//         render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, finalao);
//         render_mesh.set_indices(Some(Indices::U32(res.indices.clone())));
//         let collider = if res.positions.len() >= 4 {
//             Collider::from_bevy_mesh(&render_mesh.clone(), &ComputedColliderShape::TriMesh)
//                 .unwrap_or_default()
//         } else {
//             Collider::cuboid(0.0, 0.0, 0.0)
//         };
//         commands.spawn(RenderedChunk {
//             collider,
//             chunk: ChunkComp {
//                 chunk_data: evt.raw_chunk.clone(),
//                 pos: evt.pos.into(),
//                 dirty: true,
//                 entities: Vec::new(),
//                 saved_entities: Vec::new(),
//             },
//             mesh: PbrBundle {
//                 mesh: meshes.add(render_mesh.clone()),
//                 material: materials.add(StandardMaterial {
//                     base_color: Color::WHITE,
//                     base_color_texture: Some(
//                         texture_atlas
//                             .get(&loadable_assets.block_atlas)
//                             .unwrap()
//                             .texture
//                             .clone(),
//                     ),
//                     alpha_mode: AlphaMode::Mask(1.0),
//                     perceptual_roughness: 1.0,
//                     ..default()
//                 }),
//                 transform: Transform::from_translation(Vec3::new(
//                     (evt.pos[0] * (CHUNK_SIZE - 2) as i32) as f32,
//                     (evt.pos[1] * (CHUNK_SIZE - 2) as i32) as f32,
//                     (evt.pos[2] * (CHUNK_SIZE - 2) as i32) as f32,
//                 )),
//                 ..Default::default()
//             },
//         });
//         // This is stupid and awful so ill come back to semi transparent objects
//         // cmd2.spawn(PbrBundle {
//         //     mesh: meshes.add(render_mesh),
//         //     material: materials.add(StandardMaterial {
//         //         base_color: Color::WHITE,
//         //         // base_color_texture: Some(texture_handle.0.clone()),
//         //         alpha_mode: AlphaMode::Blend,
//         //         perceptual_roughness: 1.0,
//         //         ..default()
//         //     }),
//         //     transform: Transform::from_translation(Vec3::new(
//         //         (pos[0] * (CHUNK_SIZE / 2) as i32) as f32,
//         //         (pos[1] * (CHUNK_SIZE / 2) as i32) as f32,
//         //         (pos[2] * (CHUNK_SIZE / 2) as i32) as f32,
//         //     )),
//         //     ..Default::default()
//         // });
//     }
// }

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

// // pub fn calculate_ao(neighbours: &[VoxelVisibility; 8]) -> [u8; 4] {
// //     let mut ao = [0; 4];
// //     if neighbours[0] == VoxelVisibility::Opaque && neighbours[2] == VoxelVisibility::Opaque {
// //         ao[1] = 0;
// //     } else if neighbours[1] == VoxelVisibility::Opaque
// //         && (neighbours[0] == VoxelVisibility::Opaque || neighbours[2] == VoxelVisibility::Opaque)
// //     {
// //         ao[1] = 1;
// //     } else if neighbours[0] == VoxelVisibility::Opaque
// //         || neighbours[1] == VoxelVisibility::Opaque
// //         || neighbours[2] == VoxelVisibility::Opaque
// //     {
// //         ao[1] = 2;
// //     } else {
// //         ao[1] = 3;
// //     }
// //     if neighbours[2] == VoxelVisibility::Opaque && neighbours[4] == VoxelVisibility::Opaque {
// //         ao[0] = 0;
// //     } else if neighbours[3] == VoxelVisibility::Opaque
// //         && (neighbours[2] == VoxelVisibility::Opaque || neighbours[4] == VoxelVisibility::Opaque)
// //     {
// //         ao[0] = 1;
// //     } else if neighbours[2] == VoxelVisibility::Opaque
// //         || neighbours[3] == VoxelVisibility::Opaque
// //         || neighbours[4] == VoxelVisibility::Opaque
// //     {
// //         ao[0] = 2;
// //     } else {
// //         ao[0] = 3;
// //     }
// //     if neighbours[4] == VoxelVisibility::Opaque && neighbours[6] == VoxelVisibility::Opaque {
// //         ao[2] = 0;
// //     } else if neighbours[5] == VoxelVisibility::Opaque
// //         && (neighbours[4] == VoxelVisibility::Opaque || neighbours[6] == VoxelVisibility::Opaque)
// //     {
// //         ao[2] = 1;
// //     } else if neighbours[4] == VoxelVisibility::Opaque
// //         || neighbours[5] == VoxelVisibility::Opaque
// //         || neighbours[6] == VoxelVisibility::Opaque
// //     {
// //         ao[2] = 2;
// //     } else {
// //         ao[2] = 3;
// //     }
// //     if neighbours[6] == VoxelVisibility::Opaque && neighbours[0] == VoxelVisibility::Opaque {
// //         ao[3] = 0;
// //     } else if neighbours[7] == VoxelVisibility::Opaque
// //         && (neighbours[6] == VoxelVisibility::Opaque || neighbours[0] == VoxelVisibility::Opaque)
// //     {
// //         ao[3] = 1;
// //     } else if neighbours[6] == VoxelVisibility::Opaque
// //         || neighbours[7] == VoxelVisibility::Opaque
// //         || neighbours[0] == VoxelVisibility::Opaque
// //     {
// //         ao[3] = 2;
// //     } else {
// //         ao[3] = 3;
// //     }
// //     ao
// // }

//TODO: move this out just testing rn

fn ao_convert(ao: Vec<u8>) -> Vec<[f32; 4]> {
    let mut res = Vec::new();
    for value in ao {
        match value {
            0 => res.extend_from_slice(&[[0.3, 0.3, 0.3, 1.0]]),
            1 => res.extend_from_slice(&[[0.5, 0.5, 0.5, 1.0]]),
            2 => res.extend_from_slice(&[[0.75, 0.75, 0.75, 1.0]]),
            _ => res.extend_from_slice(&[[1., 1., 1., 1.0]]),
        }
    }
    return res;
}
