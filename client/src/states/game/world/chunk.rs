use std::collections::{HashMap, HashSet};

use bevy::{ecs::schedule::ShouldRun, prelude::*};
use bevy_rapier3d::prelude::Collider;
use bimap::BiMap;
use common::game::world::chunk::{ChunkComp, RawChunk, CHUNK_SIZE};

use crate::{
    components::GameState,
    states::game::{networking::components::ControlledPlayer, rendering::meshing::build_mesh},
};

#[derive(Bundle)]
pub struct RenderedChunk {
    pub chunk: ChunkComp,
    #[bundle]
    pub mesh: PbrBundle,
    pub collider: Collider,
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
}

#[derive(Resource)]
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
        if (pos.x > (max_bound.x + self.chunk_pos.x) || pos.x < (min_bound.x + self.chunk_pos.x))
            || (pos.y > (max_bound.y + self.chunk_pos.y)
                || pos.y < (min_bound.y + self.chunk_pos.y))
            || (pos.z > (max_bound.x + self.chunk_pos.z)
                || pos.z < (min_bound.x + self.chunk_pos.z))
        {
            return false;
        } else {
            return true;
        }
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
        (pos.x / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
        (pos.y / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
        (pos.z / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
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
    mut commands: Commands,
    mut current_chunks: ResMut<CurrentChunks>,
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

pub fn update_borders(
    mut current_chunks: ResMut<CurrentChunks>,
    chunks: Query<&mut ChunkComp>,
    mut dirty_chunks: ResMut<DirtyChunks>,
) {
    for chunk_pos in dirty_chunks.chunks.iter() {
        let chunk_entity = current_chunks.get_entity(*chunk_pos).unwrap();
        // chunks.get(chunk_entity).unwrap().chunk_data.voxels
    }
}

/// Label for the stage housing the chunk loading systems.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash, StageLabel)]
pub struct ChunkLoadingStage;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Hash, SystemLabel)]

pub enum ChunkLoadingSystem {
    /// Runs chunk view distance calculations and queue events for chunk creations and deletions.
    UpdateChunks,
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
                        clear_unloaded_chunks
                            .label(ChunkLoadingSystem::UpdateChunks)
                            .after(ChunkLoadingSystem::UpdatePlayer)
                            .with_run_criteria(should_update_chunks),
                    )
                    .with_system(
                        build_mesh
                            .label(ChunkLoadingSystem::CreateChunks)
                            .after(ChunkLoadingSystem::UpdateChunks),
                    ),
            )
            .add_system_to_stage(CoreStage::Last, delete_chunks);
    }
}
