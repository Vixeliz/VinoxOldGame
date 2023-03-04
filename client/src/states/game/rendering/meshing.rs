use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_rapier3d::prelude::Collider;
use common::game::world::chunk::{
    Chunk, ChunkComp, LoadableTypes, RawChunk, Voxel, VoxelVisibility, CHUNK_SIZE,
};
use futures_lite::future;
use itertools::Itertools;

use crate::states::{
    game::world::chunk::{
        ChunkCollider, ChunkQueue, CurrentChunks, PlayerChunk, RenderedChunk, ViewDistance,
    },
    loading::LoadableAssets,
};

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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
    pub fn indices(&self, start: u32, flipped: bool) -> [u32; 6] {
        if flipped {
            [start, start + 2, start + 1, start + 1, start + 2, start + 3]
        } else {
            [start, start + 3, start + 1, start, start + 2, start + 3]
        }
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

    pub fn voxel(&self) -> [usize; 3] {
        self.quad.voxel
    }
}

pub struct MeshChunkEvent {
    pub pos: IVec3,
}

// pub struct ChunkMesh {
//     pub positions: Vec<[f32; 3]>,
//     pub normals: Vec<[f32; 3]>,
//     pub uvs: Vec<[f32; 2]>,
//     pub indices: Vec<u32>,
//     pub ao: Vec<u8>,
// }

// TODO: Fix the one quad not being flipped
pub fn calculate_ao<C, T>(
    chunk: &C,
    current_side: Side,
    x: u32,
    y: u32,
    z: u32,
    loadable_types: &LoadableTypes,
) -> [u8; 4]
where
    C: Chunk<Output = T>,
    T: Voxel,
{
    let neighbours: [T; 8];
    if current_side == Side::new(Axis::X, false) {
        neighbours = [
            chunk.get(x - 1, y, z - 1, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
            chunk.get(x - 1, y - 1, z, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
        ];
    } else if current_side == Side::new(Axis::X, true) {
        neighbours = [
            chunk.get(x + 1, y, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
        ];
    } else if current_side == Side::new(Axis::Y, false) {
        neighbours = [
            chunk.get(x, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
            chunk.get(x, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
        ];
    } else if current_side == Side::new(Axis::Y, true) {
        neighbours = [
            chunk.get(x, y + 1, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
            chunk.get(x, y + 1, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
        ];
    } else if current_side == Side::new(Axis::Z, true) {
        neighbours = [
            chunk.get(x - 1, y, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
            chunk.get(x, y + 1, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
        ];
    } else {
        neighbours = [
            chunk.get(x + 1, y, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x, y - 1, z - 1, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
            chunk.get(x - 1, y, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
            chunk.get(x, y + 1, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
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

pub fn generate_mesh<C, T>(chunk: &C, loadable_types: &LoadableTypes) -> QuadGroups
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
            let voxel = chunk.get(x, y, z, loadable_types);
            match voxel.visibility() {
                EMPTY => continue,
                visibility => {
                    let neighbors = [
                        chunk.get(x - 1, y, z, loadable_types),
                        chunk.get(x + 1, y, z, loadable_types),
                        chunk.get(x, y - 1, z, loadable_types),
                        chunk.get(x, y + 1, z, loadable_types),
                        chunk.get(x, y, z - 1, loadable_types),
                        chunk.get(x, y, z + 1, loadable_types),
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

pub fn build_mesh(
    mut event: EventReader<MeshChunkEvent>,
    mut chunk_queue: ResMut<ChunkQueue>,
    player_chunk: Res<PlayerChunk>,
    view_distance: Res<ViewDistance>,
    chunks: Query<&ChunkComp>,
    current_chunks: Res<CurrentChunks>,
) {
    // let block_atlas = texture_atlas.get(&loadable_assets.block_atlas).unwrap();
    // 0 and CHUNK_SIZE_PADDED dont get built into the mesh itself its data for meshing from other chunks this is just one solution
    // TODO: Redo a lot of this code but for now just want a working implementation. The ao and custom geometry are the things I think need the most looking at
    for evt in event.iter() {
        if player_chunk.is_in_radius(
            evt.pos,
            IVec2::new(-view_distance.horizontal, -view_distance.vertical),
            IVec2::new(view_distance.horizontal, view_distance.vertical),
        ) {
            chunk_queue.mesh.push((
                evt.pos,
                chunks
                    .get(current_chunks.get_entity(evt.pos).unwrap())
                    .unwrap()
                    .chunk_data
                    .clone(),
            ));
        }
    }
}

#[derive(Component)]
pub struct MeshedChunk {
    chunk_mesh: Mesh,
    collider: Collider,
    pos: IVec3,
}

#[derive(Component)]
pub struct ChunkGenTask(Task<MeshedChunk>);

pub fn process_task(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut ChunkGenTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    texture_atlas: Res<Assets<TextureAtlas>>,
    loadable_assets: ResMut<LoadableAssets>,
    current_chunks: ResMut<CurrentChunks>,
) {
    let _block_atlas = texture_atlas.get(&loadable_assets.block_atlas).unwrap();
    for (entity, mut chunk_task) in &mut chunk_query {
        if let Some(chunk) = future::block_on(future::poll_once(&mut chunk_task.0)) {
            if let Some(chunk_entity) = current_chunks.get_entity(chunk.pos) {
                commands.entity(chunk_entity).insert((
                    RenderedChunk {
                        mesh: PbrBundle {
                            mesh: meshes.add(chunk.chunk_mesh.clone()),
                            material: materials.add(StandardMaterial {
                                base_color: Color::WHITE,
                                base_color_texture: Some(
                                    texture_atlas
                                        .get(&loadable_assets.block_atlas)
                                        .unwrap()
                                        .texture
                                        .clone(),
                                ),
                                perceptual_roughness: 1.0,
                                ..default()
                            }),
                            transform: Transform::from_translation(Vec3::new(
                                (chunk.pos[0] * (CHUNK_SIZE) as i32) as f32,
                                (chunk.pos[1] * (CHUNK_SIZE) as i32) as f32,
                                (chunk.pos[2] * (CHUNK_SIZE) as i32) as f32,
                            )),
                            ..Default::default()
                        },
                    },
                    ChunkCollider {
                        collider: chunk.collider.clone(),
                    },
                ));

                commands.entity(entity).despawn_recursive();
            } else {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

pub fn process_queue(
    mut chunk_queue: ResMut<ChunkQueue>,
    mut commands: Commands,
    loadable_assets: ResMut<LoadableAssets>,
    loadable_types: Res<LoadableTypes>,
    texture_atlas: Res<Assets<TextureAtlas>>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _current_chunks: ResMut<CurrentChunks>,
) {
    //TODO: Look into some other way to do this and profile it. Lots of clones for every chunk
    let task_pool = AsyncComputeTaskPool::get();
    let block_atlas: TextureAtlas = texture_atlas
        .get(&loadable_assets.block_atlas)
        .unwrap()
        .clone();
    chunk_queue
        .mesh
        .drain(..)
        .map(|(chunk_pos, raw_chunk)| {
            let cloned_types: LoadableTypes = loadable_types.clone();
            let cloned_assets: LoadableAssets = loadable_assets.clone();
            let clone_atlas: TextureAtlas = block_atlas.clone();
            (
                chunk_pos,
                ChunkGenTask(task_pool.spawn(async move {
                    let mesh_result = generate_mesh(&raw_chunk, &cloned_types);
                    let mut positions = Vec::new();
                    let mut indices = Vec::new();
                    let mut normals = Vec::new();
                    let mut uvs = Vec::new();
                    let mut ao = Vec::new();
                    for face in mesh_result.iter() {
                        // if face.quad.voxel[0] as u32 == CHUNK_SIZE - 1
                        //     && face.quad.voxel[1] == 1
                        //     && face.quad.voxel[2] == 1
                        // {
                        //     if face.side.axis == Axis::X && face.side.positive == false {
                        //         println!("positions: {:?}", face.positions(1.0));
                        //     }
                        //     // println!("face: axis {:?}:{:?}", face.side.axis, face.side.positive);
                        // }
                        let calculated_ao = calculate_ao(
                            &raw_chunk,
                            face.side,
                            face.quad.voxel[0] as u32,
                            face.quad.voxel[1] as u32,
                            face.quad.voxel[2] as u32,
                            &cloned_types,
                        );
                        if (calculated_ao[1] + calculated_ao[3])
                            > (calculated_ao[2] + calculated_ao[0])
                        {
                            indices.extend_from_slice(&face.indices(positions.len() as u32, true));
                        } else {
                            indices.extend_from_slice(&face.indices(positions.len() as u32, false));
                        }
                        positions.extend_from_slice(&face.positions(1.0)); // Voxel size is 1m
                        normals.extend_from_slice(&face.normals());
                        ao.extend_from_slice(&calculated_ao);

                        let matched_index = match (face.side.axis, face.side.positive) {
                            (Axis::X, false) => 2,
                            (Axis::X, true) => 3,
                            (Axis::Y, false) => 1,
                            (Axis::Y, true) => 0,
                            (Axis::Z, false) => 5,
                            (Axis::Z, true) => 4,
                        };
                        if let Some(texture_index) = clone_atlas.get_texture_index(
                            &cloned_assets
                                .block_textures
                                .get(
                                    &raw_chunk
                                        .get_state_for_index(
                                            raw_chunk.voxels.0[RawChunk::linearize(UVec3::new(
                                                face.quad.voxel[0] as u32,
                                                face.quad.voxel[1] as u32,
                                                face.quad.voxel[2] as u32,
                                            ))]
                                                as usize,
                                        )
                                        .unwrap(),
                                )
                                .unwrap()[matched_index],
                        ) {
                            let face_coords = calculate_coords(
                                texture_index,
                                Vec2::new(16.0, 16.0),
                                clone_atlas.size,
                            );
                            uvs.push(face_coords[0]);
                            uvs.push(face_coords[1]);
                            uvs.push(face_coords[2]);
                            uvs.push(face_coords[3]);
                        } else {
                            uvs.extend_from_slice(&face.uvs(false, false));
                        }
                    }
                    let col_vertices = positions
                        .iter()
                        .cloned()
                        .map(Vec3::from_array)
                        .collect::<Vec<_>>();

                    let col_indices = indices
                        .iter()
                        .cloned()
                        .tuples::<(u32, u32, u32)>()
                        .map(|(x, y, z)| [x, y, z])
                        .collect::<Vec<_>>();
                    let final_ao = ao_convert(ao);
                    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    let collider = if !indices.is_empty() {
                        Collider::trimesh(col_vertices, col_indices)
                    } else {
                        Collider::cuboid(0.0, 0.0, 0.0)
                    };
                    mesh.set_indices(Some(Indices::U32(indices)));
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
                    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, final_ao);

                    MeshedChunk {
                        chunk_mesh: mesh,
                        pos: chunk_pos,
                        collider,
                    }
                })),
            )
        })
        .for_each(|(_chunk_pos, chunk)| {
            let _chunk_id = commands.spawn(chunk).id();
            // current_chunks.insert_entity(chunk_pos, chunk_id);
        });
}

pub fn calculate_coords(index: usize, tile_size: Vec2, tilesheet_size: Vec2) -> [[f32; 2]; 4] {
    let mut face_tex = [[0.0; 2]; 4];
    let mut index = index as f32;
    // We need to start at 1.0 for calculations
    index += 1.0;
    let max_y = (tile_size.y) / tilesheet_size.y;
    face_tex[2][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[0][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[2][1] = 0.0;
    face_tex[3][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[1][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[3][1] = 0.0;
    face_tex[0][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[2][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[0][1] = max_y;
    face_tex[1][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[3][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[1][1] = max_y;
    face_tex
}
fn ao_convert(ao: Vec<u8>) -> Vec<[f32; 4]> {
    let mut res = Vec::new();
    for value in ao {
        match value {
            0 => res.extend_from_slice(&[[0.1, 0.1, 0.1, 1.0]]),
            1 => res.extend_from_slice(&[[0.25, 0.25, 0.25, 1.0]]),
            2 => res.extend_from_slice(&[[0.5, 0.5, 0.5, 1.0]]),
            _ => res.extend_from_slice(&[[1., 1., 1., 1.0]]),
        }
    }
    res
}
