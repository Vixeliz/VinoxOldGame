use std::collections::{HashMap, HashSet};

use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_rapier3d::prelude::Collider;

use common::game::world::chunk::{
    Chunk, ChunkComp, ChunkPos, LoadableTypes, RawChunk, VoxelType, CHUNK_BOUND, CHUNK_SIZE,
    CHUNK_SIZE_PADDED, TOTAL_CHUNK_SIZE,
};

use crate::states::game::{
    networking::components::ControlledPlayer,
    rendering::meshing::{build_mesh, MeshChunkEvent},
};

#[derive(Component)]
pub struct DirtyChunk;

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
    mut commands: Commands,
    current_chunks: ResMut<CurrentChunks>,
    mut chunk_set: ParamSet<(
        Query<(&ChunkComp, Entity), With<DirtyChunk>>,
        Query<&mut ChunkComp>,
    )>,
    // dirty_chunks: Query<(&ChunkComp, Entity), With<DirtyChunk>>,
    // mut all_chunks: Query<&mut ChunkComp>,
    mut mesh_event: EventWriter<MeshChunkEvent>,
    view_distance: Res<ViewDistance>,
) {
    let mut dirty_chunk_positions = Vec::new();
    for dirty_chunk in chunk_set.p0().iter() {
        dirty_chunk_positions.push(dirty_chunk.0.pos.0);
    }
    for dirty_chunk_pos in dirty_chunk_positions.iter() {
        if current_chunks.get_entity(*dirty_chunk_pos).is_some() {
            if current_chunks.all_neighbors_exist(
                *dirty_chunk_pos,
                IVec2::new(-view_distance.horizontal, -view_distance.vertical),
                IVec2::new(view_distance.horizontal, view_distance.vertical),
            ) {
                let dirty_entity = current_chunks.get_entity(*dirty_chunk_pos).unwrap();
                let neighbor_entities = [
                    current_chunks.get_entity(*dirty_chunk_pos).unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(0, -1, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(0, 1, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(-1, 0, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(1, 0, 0))
                        .unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(0, 0, -1))
                        .unwrap(),
                    current_chunks
                        .get_entity(*dirty_chunk_pos + IVec3::new(0, 0, 1))
                        .unwrap(),
                ];
                let mut new_chunks = Vec::new();
                if let Ok(chunk_data) = chunk_set.p1().get_many_mut(neighbor_entities) {
                    if chunk_data[0].chunk_data.palette == vec!["air".to_string()] {
                        commands.entity(dirty_entity).remove::<DirtyChunk>();
                        break;
                    }
                    // TODO: Try to figure out a better way to do this
                    let mut chunk_data_cloned = chunk_data.map(|x| x.chunk_data.clone());
                    for index in 0..chunk_data_cloned[0].voxels.len() {
                        let (x, y, z) = RawChunk::delinearize(index as usize);
                        match (x, y, z) {
                            (1..=CHUNK_SIZE, CHUNK_BOUND, 1..=CHUNK_SIZE) => {
                                let block_string =
                                    chunk_data_cloned[2].get_block(UVec3::new(x, 1, z)).unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 0, 1..=CHUNK_SIZE) => {
                                let block_string = chunk_data_cloned[1]
                                    .get_block(UVec3::new(x, CHUNK_SIZE, z))
                                    .unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (0, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                let block_string = chunk_data_cloned[3]
                                    .get_block(UVec3::new(CHUNK_SIZE, y, z))
                                    .unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (CHUNK_BOUND, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                let block_string =
                                    chunk_data_cloned[4].get_block(UVec3::new(1, y, z)).unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, 0) => {
                                let block_string = chunk_data_cloned[5]
                                    .get_block(UVec3::new(x, y, CHUNK_SIZE))
                                    .unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, CHUNK_BOUND) => {
                                let block_string =
                                    chunk_data_cloned[6].get_block(UVec3::new(x, y, 1)).unwrap();
                                chunk_data_cloned[0].add_block_state(&block_string);
                                chunk_data_cloned[0].set_block(UVec3::new(x, y, z), block_string);
                            }
                            (_, _, _) => {}
                        };
                    }
                    new_chunks.push(chunk_data_cloned[0].clone());
                }
                let mut chunk_set = chunk_set.p1();
                let mut chunk_data = chunk_set.get_mut(neighbor_entities[0]).unwrap();
                for chunk in new_chunks.iter() {
                    chunk_data.chunk_data = chunk.to_owned();
                    mesh_event.send(MeshChunkEvent {
                        pos: *dirty_chunk_pos,
                    });
                    commands.entity(dirty_entity).remove::<DirtyChunk>();
                }
            }
        } else {
            // let dirty_entity = current_chunks.get_entity(*dirty_chunk_pos).unwrap();
            // commands.entity(dirty_entity).remove::<DirtyChunk>();
        }
    }
}

pub fn receive_chunks(
    mut current_chunks: ResMut<CurrentChunks>,
    mut commands: Commands,
    mut event: EventReader<CreateChunkEvent>,
    _mesh_event: EventWriter<MeshChunkEvent>,
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
            if let Some(chunk_id) = current_chunks.get_entity(evt.pos) {
                commands.entity(chunk_id).insert(ChunkComp {
                    pos: ChunkPos(evt.pos),
                    chunk_data: evt.raw_chunk.to_owned(),
                    saved_entities: Vec::new(),
                    entities: Vec::new(),
                });
                if !(evt.raw_chunk.palette == vec!["air".to_string()]) {
                    commands.entity(chunk_id).insert(DirtyChunk);
                }
            } else {
                let chunk_id = commands
                    .spawn(ChunkComp {
                        pos: ChunkPos(evt.pos),
                        chunk_data: evt.raw_chunk.to_owned(),
                        saved_entities: Vec::new(),
                        entities: Vec::new(),
                    })
                    .id();
                current_chunks.insert_entity(evt.pos, chunk_id);
                if !(evt.raw_chunk.palette == vec!["air".to_string()]) {
                    commands.entity(chunk_id).insert(DirtyChunk);
                }
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
