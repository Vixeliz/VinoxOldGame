use std::collections::{HashMap, HashSet};

use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_rapier3d::prelude::Collider;

use common::game::world::chunk::{
    Chunk, ChunkComp, LoadableTypes, RawChunk, VoxelType, CHUNK_BOUND, CHUNK_SIZE,
    CHUNK_SIZE_PADDED, TOTAL_CHUNK_SIZE,
};

use crate::states::game::{
    networking::components::ControlledPlayer,
    rendering::meshing::{build_mesh, MeshChunkEvent},
};

#[derive(Bundle)]
pub struct RenderedChunk {
    #[bundle]
    pub mesh: PbrBundle,
    // pub collider: Collider,
}

pub struct CreateChunkEvent {
    pub pos: IVec3,
    pub raw_chunk: RawChunk,
}

pub struct UpdateChunkEvent {
    pub pos: IVec3,
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
    pub fn all_neighbors_exist(&self, pos: IVec3, min_bound: IVec2, max_bound: IVec2) -> bool {
        if self.chunks.contains_key(&(pos + IVec3::new(0, 1, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, -1, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(1, 0, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(-1, 0, 0)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, 0, 1)))
            && self.chunks.contains_key(&(pos + IVec3::new(0, 0, -1)))
        {
            true
        } else {
            false
        }
    }
}

#[derive(Resource, Default)]
pub struct DirtyChunks {
    pub chunks: HashSet<IVec3>,
}

#[allow(dead_code)]
impl DirtyChunks {
    pub fn mark_dirty(&mut self, pos: IVec3) {
        self.chunks.insert(pos);
    }

    pub fn iter_dirty(&self) -> impl Iterator<Item = &IVec3> {
        self.chunks.iter()
    }

    pub fn num_dirty(&self) -> usize {
        self.chunks.len()
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

#[derive(Default, Resource)]
pub struct ChunkQueue {
    pub mesh: Vec<(IVec3, RawChunk)>,
    pub remove: Vec<IVec3>,
}

#[derive(Default, Resource)]
pub struct PlayerChunk {
    pub chunk_pos: IVec3,
    pub raw_pos: Vec3,
}

impl PlayerChunk {
    pub fn is_in_radius(&self, pos: IVec3, min_bound: IVec2, max_bound: IVec2) -> bool {
        !((pos.x > (max_bound.x + self.chunk_pos.x) || pos.x < (min_bound.x + self.chunk_pos.x))
            || (pos.y > (max_bound.y + self.chunk_pos.y)
                || pos.y < (min_bound.y + self.chunk_pos.y))
            || (pos.z > (max_bound.x + self.chunk_pos.z)
                || pos.z < (min_bound.x + self.chunk_pos.z)))
    }
}
pub fn update_player_location(
    player_query: Query<&Transform, With<ControlledPlayer>>,
    mut player_chunk: ResMut<PlayerChunk>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        player_chunk.chunk_pos = world_to_chunk(player_transform.translation);
        player_chunk.raw_pos = player_transform.translation;
    }
}

pub fn world_to_chunk(pos: Vec3) -> IVec3 {
    IVec3::new(
        (pos.x / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.y / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.z / (CHUNK_SIZE as f32)).floor() as i32,
    )
}

pub fn delete_chunks(
    mut current_chunks: ResMut<CurrentChunks>,
    mut commands: Commands,
    mut chunk_queue: ResMut<ChunkQueue>,
) {
    for chunk in chunk_queue.remove.drain(..) {
        commands
            .entity(current_chunks.remove_entity(chunk).unwrap())
            .despawn_recursive();
    }
}

pub fn should_update_chunks(player_chunk: Res<PlayerChunk>) -> ShouldRun {
    if player_chunk.is_changed() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

pub fn clear_unloaded_chunks(
    _commands: Commands,
    current_chunks: ResMut<CurrentChunks>,
    view_distance: Res<ViewDistance>,
    player_chunk: Res<PlayerChunk>,
    mut chunk_queue: ResMut<ChunkQueue>,
) {
    for chunk_pos in current_chunks.chunks.keys() {
        if !player_chunk.is_in_radius(
            *chunk_pos,
            IVec2::new(-view_distance.horizontal, -view_distance.vertical),
            IVec2::new(view_distance.horizontal, view_distance.vertical),
        ) {
            chunk_queue.remove.push(*chunk_pos);
        }
    }
}

// Dirty chunks get marked in the following cases. A new neighbor spawns by them, the terrain is modified, or if a neighbor disapears
// This runs first then we remesh
pub fn update_borders(
    current_chunks: ResMut<CurrentChunks>,
    mut chunks: Query<&mut ChunkComp>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    _chunk_queue: ResMut<ChunkQueue>,
    mut mesh_event: EventWriter<MeshChunkEvent>,
    view_distance: Res<ViewDistance>,
    loadable_types: Res<LoadableTypes>,
) {
    let cloned_chunks = dirty_chunks.chunks.clone();
    for chunk_pos in cloned_chunks.iter() {
        if current_chunks.get_entity(*chunk_pos).is_some() {
            if current_chunks.all_neighbors_exist(
                *chunk_pos,
                IVec2::new(-view_distance.horizontal, -view_distance.vertical),
                IVec2::new(view_distance.horizontal, view_distance.vertical),
            ) {
                let neighbor_entities = [
                    current_chunks.get_entity(*chunk_pos).unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(0, -1, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(0, 1, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(-1, 0, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(1, 0, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(0, 0, -1))
                        .unwrap(),
                    current_chunks
                        .get_entity(*chunk_pos + IVec3::new(0, 0, 1))
                        .unwrap(),
                ];
                if let Ok(chunk_data) = chunks.get_many_mut(neighbor_entities) {
                    if chunk_data[0].chunk_data.palette == vec!["air".to_string()] {
                        dirty_chunks.chunks.remove(&chunk_pos);
                        break;
                    }
                    // TODO: Try to figure out a better way to do this
                    let mut chunk_data = chunk_data.map(|x| x.chunk_data.clone());
                    for index in 0..chunk_data[0].voxels.len() {
                        let (x, y, z) = RawChunk::delinearize(index as usize);
                        match (x, y, z) {
                            (1..=CHUNK_SIZE, CHUNK_BOUND, 1..=CHUNK_SIZE) => {
                                let block_string =
                                    chunk_data[2].get_block(UVec3::new(x, 1, z)).unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 0, 1..=CHUNK_SIZE) => {
                                let block_string = chunk_data[1]
                                    .get_block(UVec3::new(x, CHUNK_SIZE, z))
                                    .unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (0, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                let block_string = chunk_data[3]
                                    .get_block(UVec3::new(CHUNK_SIZE, y, z))
                                    .unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (CHUNK_BOUND, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                let block_string =
                                    chunk_data[4].get_block(UVec3::new(1, y, z)).unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, 0) => {
                                let block_string = chunk_data[5]
                                    .get_block(UVec3::new(x, y, CHUNK_SIZE))
                                    .unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, CHUNK_BOUND) => {
                                let block_string =
                                    chunk_data[6].get_block(UVec3::new(x, y, 1)).unwrap();
                                chunk_data[0].add_block_state(&block_string);
                                chunk_data[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (_, _, _) => {}
                        };
                    }
                    let mut chunk = chunks.get_mut(neighbor_entities[0]).unwrap();
                    chunk.chunk_data = chunk_data[0].to_owned();
                    mesh_event.send(MeshChunkEvent { pos: *chunk_pos });
                    dirty_chunks.chunks.remove(&chunk_pos);
                }
            }
        } else {
            dirty_chunks.chunks.remove(&chunk_pos);
        }
    }
}

pub fn receive_chunks(
    mut current_chunks: ResMut<CurrentChunks>,
    mut commands: Commands,
    mut event: EventReader<CreateChunkEvent>,
    _mesh_event: EventWriter<MeshChunkEvent>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    player_chunk: Res<PlayerChunk>,
    view_distance: Res<ViewDistance>,
    _loadable_types: Res<LoadableTypes>,
) {
    for evt in event.iter() {
        if player_chunk.is_in_radius(
            evt.pos,
            IVec2::new(-view_distance.horizontal, -view_distance.vertical),
            IVec2::new(view_distance.horizontal, view_distance.vertical),
        ) {
            let chunk_id = commands
                .spawn(ChunkComp {
                    pos: evt.pos,
                    chunk_data: evt.raw_chunk.to_owned(),
                    saved_entities: Vec::new(),
                    entities: Vec::new(),
                })
                .id();
            current_chunks.insert_entity(evt.pos, chunk_id);
            if !(evt.raw_chunk.palette == vec!["air".to_string()]) {
                dirty_chunks.mark_dirty(evt.pos);
            }
        }
    }
}

/// Label for the stage housing the chunk loading systems.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash, StageLabel)]
pub struct ChunkLoadingStage;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash, SystemLabel)]

pub enum ChunkLoadingSystem {
    /// Runs chunk view distance calculations and queue events for chunk creations and deletions.
    UpdateChunks,
    ReceiveChunks,
    DirtyChunks,
    /// Creates the voxel buffers to hold chunk data and attach them a chunk entity in the ECS world.
    CreateChunks,
    UpdatePlayer,
}

pub struct ChunkHandling;

impl Plugin for ChunkHandling {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
            .insert_resource(DirtyChunks::default())
            .insert_resource(PlayerChunk::default())
            .insert_resource(ViewDistance {
                horizontal: 8,
                vertical: 4,
            })
            .insert_resource(SimulationDistance {
                width: 4,
                height: 4,
                depth: 4,
            })
            .add_stage_after(
                CoreStage::Update,
                ChunkLoadingStage,
                SystemStage::parallel()
                    .with_system(update_player_location.label(ChunkLoadingSystem::UpdatePlayer))
                    .with_system(
                        receive_chunks
                            .label(ChunkLoadingSystem::ReceiveChunks)
                            .after(ChunkLoadingSystem::DirtyChunks),
                    )
                    .with_system(
                        clear_unloaded_chunks
                            .label(ChunkLoadingSystem::UpdateChunks)
                            .after(ChunkLoadingSystem::ReceiveChunks)
                            .with_run_criteria(should_update_chunks),
                    )
                    .with_system(
                        update_borders
                            .label(ChunkLoadingSystem::DirtyChunks)
                            .after(ChunkLoadingSystem::UpdatePlayer),
                    )
                    .with_system(
                        build_mesh
                            .label(ChunkLoadingSystem::CreateChunks)
                            .after(ChunkLoadingSystem::UpdateChunks),
                    ),
            )
            .add_system_to_stage(CoreStage::Last, delete_chunks)
            .add_event::<UpdateChunkEvent>()
            .add_event::<CreateChunkEvent>();
    }
}
