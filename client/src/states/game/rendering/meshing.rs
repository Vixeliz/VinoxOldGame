use std::{collections::HashSet, ops::Deref, time::Duration};

use bevy::{
    math::Vec3A,
    prelude::*,
    render::{mesh::Indices, primitives::Aabb, render_resource::PrimitiveTopology},
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_rapier3d::prelude::Collider;
use bevy_tweening::{lens::TransformPositionLens, *};
use common::game::world::chunk::{
    voxel_to_world, Chunk, ChunkComp, CurrentChunks, LoadableTypes, RawChunk, ViewDistance, Voxel,
    VoxelVisibility, CHUNK_SIZE,
};
use futures_lite::future;
use itertools::Itertools;

use crate::states::{
    game::world::chunk::{ChunkCollider, ChunkQueue, PlayerChangedPos, PlayerChunk, RenderedChunk},
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

    pub fn iter_with_ao<'a, C, V>(
        &'a self,
        chunk: &'a C,
        loadable_types: &'a LoadableTypes,
    ) -> impl Iterator<Item = FaceWithAO<'a>>
    where
        C: Chunk<Output = V>,
        V: Voxel,
    {
        self.iter()
            .map(|face| FaceWithAO::new(face, chunk, loadable_types))
    }
}

pub(crate) fn face_aos<C, V>(face: &Face, chunk: &C, loadable_types: &LoadableTypes) -> [u32; 4]
where
    C: Chunk<Output = V>,
    V: Voxel,
{
    let [x, y, z] = face.voxel();
    let (x, y, z) = (x as u32, y as u32, z as u32);

    match (face.side.axis, face.side.positive) {
        (Axis::X, false) => side_aos([
            chunk.get(x - 1, y, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
            chunk.get(x - 1, y, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
        ]),
        (Axis::X, true) => side_aos([
            chunk.get(x + 1, y, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
        ]),
        (Axis::Y, false) => side_aos([
            chunk.get(x - 1, y - 1, z, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x, y - 1, z - 1, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
        ]),
        (Axis::Y, true) => side_aos([
            chunk.get(x, y + 1, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
            chunk.get(x, y + 1, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
        ]),
        (Axis::Z, false) => side_aos([
            chunk.get(x - 1, y, z - 1, loadable_types),
            chunk.get(x - 1, y - 1, z - 1, loadable_types),
            chunk.get(x, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y - 1, z - 1, loadable_types),
            chunk.get(x + 1, y, z - 1, loadable_types),
            chunk.get(x + 1, y + 1, z - 1, loadable_types),
            chunk.get(x, y + 1, z - 1, loadable_types),
            chunk.get(x - 1, y + 1, z - 1, loadable_types),
        ]),
        (Axis::Z, true) => side_aos([
            chunk.get(x + 1, y, z + 1, loadable_types),
            chunk.get(x + 1, y - 1, z + 1, loadable_types),
            chunk.get(x, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y - 1, z + 1, loadable_types),
            chunk.get(x - 1, y, z + 1, loadable_types),
            chunk.get(x - 1, y + 1, z + 1, loadable_types),
            chunk.get(x, y + 1, z + 1, loadable_types),
            chunk.get(x + 1, y + 1, z + 1, loadable_types),
        ]),
    }
}

pub struct FaceWithAO<'a> {
    face: Face<'a>,
    aos: [u32; 4],
}

impl<'a> FaceWithAO<'a> {
    pub fn new<C, V>(face: Face<'a>, chunk: &C, loadable_types: &LoadableTypes) -> Self
    where
        C: Chunk<Output = V>,
        V: Voxel,
    {
        let aos = face_aos(&face, chunk, loadable_types);
        Self { face, aos }
    }

    pub fn aos(&self) -> [u32; 4] {
        self.aos
    }

    pub fn indices(&self, start: u32) -> [u32; 6] {
        let aos = self.aos();

        if (aos[1] + aos[2]) > (aos[0] + aos[3]) {
            [start, start + 2, start + 1, start + 1, start + 2, start + 3]
        } else {
            [start, start + 3, start + 1, start, start + 2, start + 3]
        }
    }
}

pub(crate) fn ao_value(side1: bool, corner: bool, side2: bool) -> u32 {
    match (side1, corner, side2) {
        (true, _, true) => 0,
        (true, true, false) | (false, true, true) => 1,
        (false, false, false) => 3,
        _ => 2,
    }
}

pub(crate) fn side_aos<V: Voxel>(neighbors: [V; 8]) -> [u32; 4] {
    let ns = [
        neighbors[0].visibility() == OPAQUE,
        neighbors[1].visibility() == OPAQUE,
        neighbors[2].visibility() == OPAQUE,
        neighbors[3].visibility() == OPAQUE,
        neighbors[4].visibility() == OPAQUE,
        neighbors[5].visibility() == OPAQUE,
        neighbors[6].visibility() == OPAQUE,
        neighbors[7].visibility() == OPAQUE,
    ];

    [
        ao_value(ns[0], ns[1], ns[2]),
        ao_value(ns[2], ns[3], ns[4]),
        ao_value(ns[6], ns[7], ns[0]),
        ao_value(ns[4], ns[5], ns[6]),
    ]
}

impl<'a> Deref for FaceWithAO<'a> {
    type Target = Face<'a>;

    fn deref(&self) -> &Self::Target {
        &self.face
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

    pub fn voxel(&self) -> [usize; 3] {
        self.quad.voxel
    }
}

pub struct MeshChunkEvent {
    pub pos: IVec3,
}

pub fn generate_mesh<C, T>(
    chunk: &C,
    loadable_types: &LoadableTypes,
    solid_pass: bool,
) -> QuadGroups
where
    C: Chunk<Output = T>,
    T: Voxel,
{
    assert!(C::X >= 2);
    assert!(C::Y >= 2);
    assert!(C::Z >= 2);

    let mut buffer = QuadGroups::default();

    for z in 1..C::Z - 1 {
        for y in 1..C::Y - 1 {
            for x in 1..C::X - 1 {
                let (x, y, z) = (x as u32, y as u32, z as u32);
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

                            let generate = if solid_pass {
                                match (visibility, other) {
                                    (OPAQUE, EMPTY) | (OPAQUE, TRANSPARENT) => true,

                                    (TRANSPARENT, TRANSPARENT) => voxel != neighbor,

                                    (_, _) => false,
                                }
                            } else {
                                match (visibility, other) {
                                    (TRANSPARENT, EMPTY) => true,

                                    (TRANSPARENT, TRANSPARENT) => voxel != neighbor,

                                    (_, _) => false,
                                }
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
    let mut checked = HashSet::new();
    for evt in event.iter() {
        if !checked.contains(&evt.pos) {
            checked.insert(evt.pos);
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
}

#[derive(Component)]
pub struct MeshedChunk {
    chunk_mesh: Mesh,
    transparent_mesh: Mesh,
    collider: Collider,
    transparent_collider: Collider,
    pos: IVec3,
}

#[derive(Component)]
pub struct ChunkGenTask(Task<MeshedChunk>);

#[derive(Resource, Default)]
pub struct ChunkMaterial {
    opaque: Handle<StandardMaterial>,
    transparent: Handle<StandardMaterial>,
}

pub fn create_chunk_material(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut chunk_material: ResMut<ChunkMaterial>,
    texture_atlas: Res<Assets<TextureAtlas>>,
    loadable_assets: ResMut<LoadableAssets>,
) {
    chunk_material.transparent = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(
            texture_atlas
                .get(&loadable_assets.block_atlas)
                .unwrap()
                .texture
                .clone(),
        ),
        perceptual_roughness: 1.0,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    chunk_material.opaque = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(
            texture_atlas
                .get(&loadable_assets.block_atlas)
                .unwrap()
                .texture
                .clone(),
        ),
        perceptual_roughness: 1.0,
        alpha_mode: AlphaMode::Opaque,
        ..default()
    });
}

#[allow(clippy::too_many_arguments)]
pub fn process_task(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut ChunkGenTask)>,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_material: Res<ChunkMaterial>,
    current_chunks: ResMut<CurrentChunks>,
    chunks: Query<&Handle<Mesh>>,
) {
    for (entity, mut chunk_task) in &mut chunk_query {
        if let Some(chunk) = future::block_on(future::poll_once(&mut chunk_task.0)) {
            if let Some(chunk_entity) = current_chunks.get_entity(chunk.pos) {
                commands.entity(chunk_entity).despawn_descendants();
                let tween = Tween::new(
                    EaseFunction::QuadraticInOut,
                    Duration::from_secs(1),
                    TransformPositionLens {
                        start: Vec3::new(
                            (chunk.pos[0] * (CHUNK_SIZE) as i32) as f32,
                            ((chunk.pos[1] * (CHUNK_SIZE) as i32) as f32) - CHUNK_SIZE as f32,
                            (chunk.pos[2] * (CHUNK_SIZE) as i32) as f32,
                        ),

                        end: Vec3::new(
                            (chunk.pos[0] * (CHUNK_SIZE) as i32) as f32,
                            (chunk.pos[1] * (CHUNK_SIZE) as i32) as f32,
                            (chunk.pos[2] * (CHUNK_SIZE) as i32) as f32,
                        ),
                    },
                )
                .with_repeat_count(RepeatCount::Finite(1));

                let chunk_pos = if chunks.get(chunk_entity).is_err() {
                    commands.entity(chunk_entity).insert(Animator::new(tween));
                    Vec3::new(
                        (chunk.pos[0] * (CHUNK_SIZE) as i32) as f32,
                        ((chunk.pos[1] * (CHUNK_SIZE) as i32) as f32) - CHUNK_SIZE as f32,
                        (chunk.pos[2] * (CHUNK_SIZE) as i32) as f32,
                    )
                } else {
                    Vec3::new(
                        (chunk.pos[0] * (CHUNK_SIZE) as i32) as f32,
                        (chunk.pos[1] * (CHUNK_SIZE) as i32) as f32,
                        (chunk.pos[2] * (CHUNK_SIZE) as i32) as f32,
                    )
                };

                //TODO: Spawn chunk collider at actual chunk position then have the mesh be a child instead?
                let trans_entity = commands
                    .spawn((
                        RenderedChunk {
                            aabb: Aabb {
                                center: Vec3A::new(
                                    (CHUNK_SIZE / 2) as f32,
                                    (CHUNK_SIZE / 2) as f32,
                                    (CHUNK_SIZE / 2) as f32,
                                ),
                                half_extents: Vec3A::new(
                                    (CHUNK_SIZE / 2) as f32,
                                    (CHUNK_SIZE / 2) as f32,
                                    (CHUNK_SIZE / 2) as f32,
                                ),
                            },
                            mesh: PbrBundle {
                                mesh: meshes.add(chunk.transparent_mesh.clone()),
                                material: chunk_material.transparent.clone(),
                                ..Default::default()
                            },
                        },
                        ChunkCollider {
                            collider: chunk.transparent_collider.clone(),
                        },
                    ))
                    .id();

                commands.entity(chunk_entity).insert((
                    RenderedChunk {
                        aabb: Aabb {
                            center: Vec3A::new(
                                (CHUNK_SIZE / 2) as f32,
                                (CHUNK_SIZE / 2) as f32,
                                (CHUNK_SIZE / 2) as f32,
                            ),
                            half_extents: Vec3A::new(
                                (CHUNK_SIZE / 2) as f32,
                                (CHUNK_SIZE / 2) as f32,
                                (CHUNK_SIZE / 2) as f32,
                            ),
                        },
                        mesh: PbrBundle {
                            mesh: meshes.add(chunk.chunk_mesh.clone()),
                            material: chunk_material.opaque.clone(),
                            transform: Transform::from_translation(chunk_pos),
                            ..Default::default()
                        },
                    },
                    ChunkCollider {
                        collider: chunk.collider.clone(),
                    },
                ));

                commands.entity(chunk_entity).push_children(&[trans_entity]);
                commands.entity(entity).despawn_recursive();
            } else {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
                    let mesh_result = generate_mesh(&raw_chunk, &cloned_types, true);
                    let mut positions = Vec::new();
                    let mut indices = Vec::new();
                    let mut normals = Vec::new();
                    let mut uvs = Vec::new();
                    let mut ao = Vec::new();
                    for face in mesh_result.iter_with_ao(&raw_chunk, &cloned_types) {
                        indices.extend_from_slice(&face.indices(positions.len() as u32));
                        positions.extend_from_slice(&face.positions(1.0)); // Voxel size is 1m
                        normals.extend_from_slice(&face.normals());
                        ao.extend_from_slice(&face.aos());

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
                                            raw_chunk.voxels[RawChunk::linearize(UVec3::new(
                                                face.voxel()[0] as u32,
                                                face.voxel()[1] as u32,
                                                face.voxel()[2] as u32,
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

                    //Transparent Mesh
                    let mesh_result = generate_mesh(&raw_chunk, &cloned_types, false);
                    let mut positions = Vec::new();
                    let mut indices = Vec::new();
                    let mut normals = Vec::new();
                    let mut uvs = Vec::new();
                    for face in mesh_result.iter() {
                        indices.extend_from_slice(&face.indices(positions.len() as u32));
                        positions.extend_from_slice(&face.positions(1.0)); // Voxel size is 1m
                        normals.extend_from_slice(&face.normals());

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
                                            raw_chunk.voxels[RawChunk::linearize(UVec3::new(
                                                face.voxel()[0] as u32,
                                                face.voxel()[1] as u32,
                                                face.voxel()[2] as u32,
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
                    let mut transparent_mesh = Mesh::new(PrimitiveTopology::TriangleList);
                    let transparent_collider = if !indices.is_empty() {
                        Collider::trimesh(col_vertices, col_indices)
                    } else {
                        Collider::cuboid(0.0, 0.0, 0.0)
                    };
                    transparent_mesh.set_indices(Some(Indices::U32(indices)));
                    transparent_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
                    transparent_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                    transparent_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

                    MeshedChunk {
                        transparent_mesh,
                        transparent_collider,
                        chunk_mesh: mesh,
                        pos: chunk_pos,
                        collider,
                    }
                })),
            )
        })
        .for_each(|(_chunk_pos, chunk)| {
            let _chunk_id = commands.spawn(chunk).id();
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

fn ao_convert(ao: Vec<u32>) -> Vec<[f32; 4]> {
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

pub struct SortFaces {
    chunk_pos: IVec3,
}

// TODO: Switch to using an event based system that will take in what chunk to sort. Sorting will be whenever the player changes blocks in their chunk that chunk will be sorted. When moving between chunks the surrounding chunks are sorted.
pub fn sort_faces(
    current_chunks: Res<CurrentChunks>,
    handles: Query<&Handle<Mesh>>,
    chunks: Query<&Children, With<ChunkComp>>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera_transform: Query<&GlobalTransform, With<Camera>>,
    mut events: EventReader<SortFaces>,
) {
    for evt in events.iter() {
        if let Ok(camera_transform) = camera_transform.get_single() {
            if let Some(chunk_entity) = current_chunks.get_entity(evt.chunk_pos) {
                if let Ok(children) = chunks.get(chunk_entity) {
                    if let Some(child_entity) = children.get(0) {
                        if let Ok(chunk_mesh_handle) = handles.get(*child_entity) {
                            if let Some(chunk_mesh) = meshes.get_mut(chunk_mesh_handle) {
                                let mut collected_indices = Vec::new();
                                let mut sorted_indices: Vec<([usize; 6], f32)> = Vec::new();
                                if let Some(vertex_array) =
                                    chunk_mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                                {
                                    if let Some(raw_array) = vertex_array.as_float3() {
                                        if let Some(indices) = chunk_mesh.indices() {
                                            for indice in indices.iter().chunks(6).into_iter() {
                                                let vec_ind: Vec<usize> = indice.collect();
                                                let x = (raw_array[vec_ind[1]][0]
                                                    + raw_array[vec_ind[3]][0]
                                                    + raw_array[vec_ind[4]][0]
                                                    + raw_array[vec_ind[5]][0])
                                                    / 4.0;
                                                let y = (raw_array[vec_ind[1]][1]
                                                    + raw_array[vec_ind[3]][1]
                                                    + raw_array[vec_ind[4]][1]
                                                    + raw_array[vec_ind[5]][1])
                                                    / 4.0;
                                                let z = (raw_array[vec_ind[1]][2]
                                                    + raw_array[vec_ind[3]][2]
                                                    + raw_array[vec_ind[4]][2]
                                                    + raw_array[vec_ind[5]][2])
                                                    / 4.0;
                                                let real_pos = voxel_to_world(
                                                    UVec3::new(x as u32, y as u32, z as u32),
                                                    evt.chunk_pos,
                                                );
                                                let dist = camera_transform
                                                    .translation()
                                                    .distance(real_pos);
                                                sorted_indices.push((
                                                    [
                                                        vec_ind[0], vec_ind[1], vec_ind[2],
                                                        vec_ind[3], vec_ind[4], vec_ind[5],
                                                    ],
                                                    dist,
                                                ));
                                            }
                                            sorted_indices
                                                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                                            sorted_indices.reverse();

                                            // This is horrible most definitely a better way to do this
                                            for indice in sorted_indices.iter() {
                                                collected_indices.push(indice.0[0] as u32);
                                                collected_indices.push(indice.0[1] as u32);
                                                collected_indices.push(indice.0[2] as u32);
                                                collected_indices.push(indice.0[3] as u32);
                                                collected_indices.push(indice.0[4] as u32);
                                                collected_indices.push(indice.0[5] as u32);
                                            }
                                        }
                                    }
                                }

                                chunk_mesh.set_indices(Some(Indices::U32(collected_indices)));
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn sort_chunks(
    player_chunk: Res<PlayerChunk>,
    player_changed: Res<PlayerChangedPos>,
    mut sort_face: EventWriter<SortFaces>,
) {
    if player_chunk.is_changed() {
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos,
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(1, 0, 0),
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(-1, 0, 0),
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(0, 1, 0),
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(0, -1, 0),
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(0, 0, 1),
        });
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos + IVec3::new(0, 0, -1),
        });
    }

    if player_changed.is_changed() {
        sort_face.send(SortFaces {
            chunk_pos: player_chunk.chunk_pos,
        });
    }
}
