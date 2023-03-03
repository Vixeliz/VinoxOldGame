use crate::networking::syncing::SentChunks;

use super::{
    generation::generate_chunk,
    storage::{create_database, insert_chunk, load_chunk, WorldDatabase},
};
use bevy::{
    ecs::{schedule::ShouldRun, system::SystemParam},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::FloatOrd,
};
use common::{
    game::world::chunk::{ChunkComp, ChunkPos, CHUNK_SIZE},
    networking::components::Player,
};
use futures_lite::future;
use std::collections::*;

#[derive(Resource, Default, Debug)]
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

#[derive(Component, Default, Clone)]
pub struct LoadPoint(pub IVec3);

#[derive(Component, Default, Clone)]
pub struct SentChunk(pub u64);

impl LoadPoint {
    pub fn is_in_radius(&self, pos: IVec3, min_bound: IVec2, max_bound: IVec2) -> bool {
        if (pos.x > (max_bound.x + self.0.x) || pos.x < (min_bound.x + self.0.x))
            || (pos.y > (max_bound.y + self.0.y) || pos.y < (min_bound.y + self.0.y))
            || (pos.z > (max_bound.x + self.0.z) || pos.z < (min_bound.x + self.0.z))
        {
            return false;
        }
        true
    }
}

#[derive(Component, Default)]
pub struct RemoveChunk;

#[derive(Default, Resource)]
pub struct ViewDistance {
    pub vertical: i32,
    pub horizontal: i32,
}

#[derive(Default, Resource)]
pub struct SimulationDistance {
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

#[derive(Default, Resource, Debug)]
pub struct ChunkQueue {
    pub create: Vec<IVec3>,
    pub remove: Vec<IVec3>,
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    commands: Commands<'w, 's>,
    current_chunks: ResMut<'w, CurrentChunks>,
    chunk_queue: ResMut<'w, ChunkQueue>,
    view_distance: Res<'w, ViewDistance>,
    chunk_query: Query<'w, 's, &'static ChunkComp>,
}

impl<'w, 's> ChunkManager<'w, 's> {
    pub fn add_chunk_to_queue(&mut self, pos: IVec3) {
        self.chunk_queue.create.push(pos);
    }

    pub fn get_chunks_around_chunk(&mut self, pos: IVec3) -> Vec<&ChunkComp> {
        let mut res = Vec::new();
        for x in -self.view_distance.horizontal..self.view_distance.horizontal {
            for y in -self.view_distance.vertical..self.view_distance.vertical {
                for z in -self.view_distance.horizontal..self.view_distance.horizontal {
                    if let Some(entity) =
                        self.current_chunks
                            .get_entity(IVec3::new(pos.x + x, pos.y + y, pos.z + z))
                    {
                        if let Ok(chunk) = self.chunk_query.get(entity) {
                            res.push(chunk);
                        }
                    }
                }
            }
        }
        // res.sort_unstable_by_key(|key| FloatOrd(key.pos.0.as_vec3().distance(pos.as_vec3())));

        res
    }
}

pub fn world_to_chunk(pos: Vec3) -> IVec3 {
    IVec3::new(
        (pos.x / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.y / (CHUNK_SIZE as f32)).floor() as i32,
        (pos.z / (CHUNK_SIZE as f32)).floor() as i32,
    )
}

pub fn should_update_chunks(load_points: Query<&LoadPoint, Changed<LoadPoint>>) -> ShouldRun {
    if !load_points.is_empty() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

pub fn generate_chunks_world(
    view_distance: Res<ViewDistance>,
    load_points: Query<&LoadPoint>,
    mut chunk_queue: ResMut<ChunkQueue>,
    mut current_chunks: ResMut<CurrentChunks>,
    mut commands: Commands,
    mut database: Res<WorldDatabase>,
) {
    for point in load_points.iter() {
        for x in -view_distance.horizontal..view_distance.horizontal {
            for y in -view_distance.vertical..view_distance.vertical {
                for z in -view_distance.horizontal..view_distance.horizontal {
                    let pos = IVec3::new(x + point.0.x, y + point.0.y, z + point.0.z);
                    if current_chunks.get_entity(pos).is_none() {
                        let data = database.connection.lock().unwrap();
                        if let Some(chunk) = load_chunk(pos, &data) {
                            let chunk_id = commands
                                .spawn(ChunkComp {
                                    pos: ChunkPos(pos),
                                    chunk_data: chunk,
                                    entities: Vec::new(),
                                    saved_entities: Vec::new(),
                                })
                                .id();
                            current_chunks.insert_entity(pos, chunk_id);
                        } else {
                            chunk_queue.create.push(pos);
                        }
                    }
                }
            }
        }
    }
}

pub fn destroy_chunks(
    mut commands: Commands,
    mut current_chunks: ResMut<CurrentChunks>,
    remove_chunks: Query<&ChunkPos, With<RemoveChunk>>,
    mut load_points: Query<&mut SentChunks>,
) {
    for chunk in remove_chunks.iter() {
        for mut sent_chunks in load_points.iter_mut() {
            sent_chunks.chunks.remove(&chunk.0);
        }
        commands
            .entity(current_chunks.remove_entity(chunk.0).unwrap())
            .despawn_recursive();
    }
}

pub fn clear_unloaded_chunks(
    mut commands: Commands,
    chunks: Query<(&ChunkComp, Entity)>,
    load_points: Query<&LoadPoint>,
    view_distance: Res<ViewDistance>,
) {
    for (chunk, entity) in chunks.iter() {
        for load_point in load_points.iter() {
            if !load_point.is_in_radius(
                chunk.pos.0,
                IVec2::new(-view_distance.horizontal, -view_distance.vertical),
                IVec2::new(view_distance.horizontal, view_distance.vertical),
            ) {
                break;
            } else {
                commands.entity(entity).insert(RemoveChunk);
            }
        }
    }
}

#[derive(Component)]
pub struct ChunkGenTask(Task<ChunkComp>);

pub fn process_task(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut ChunkGenTask)>,
    mut database: Res<WorldDatabase>,
) {
    for (entity, mut chunk_task) in &mut chunk_query {
        if let Some(chunk) = future::block_on(future::poll_once(&mut chunk_task.0)) {
            let data = database.connection.lock().unwrap();
            insert_chunk(chunk.pos.0, &chunk.chunk_data, &data);
            commands.entity(entity).insert(chunk);
            commands.entity(entity).remove::<ChunkGenTask>();
        }
    }
}

pub fn process_queue(
    mut commands: Commands,
    mut chunk_queue: ResMut<ChunkQueue>,
    mut current_chunks: ResMut<CurrentChunks>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    chunk_queue
        .create
        .drain(..)
        .map(|chunk_pos| {
            (
                chunk_pos,
                ChunkGenTask(task_pool.spawn(async move {
                    ChunkComp {
                        pos: ChunkPos(chunk_pos),
                        chunk_data: generate_chunk(chunk_pos, 0),
                        entities: Vec::new(),
                        saved_entities: Vec::new(),
                    }
                })),
            )
        })
        .for_each(|(chunk_pos, chunk)| {
            let chunk_id = commands.spawn(chunk).id();
            current_chunks.insert_entity(chunk_pos, chunk_id);
        });
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
}

pub struct ChunkGenerationPlugin;

impl Plugin for ChunkGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
            .insert_resource(ViewDistance {
                vertical: 4,
                horizontal: 5,
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
                    .with_system(
                        clear_unloaded_chunks
                            .label(ChunkLoadingSystem::UpdateChunks)
                            .with_run_criteria(should_update_chunks),
                    )
                    .with_system(
                        generate_chunks_world
                            .label(ChunkLoadingSystem::UpdateChunks)
                            .with_run_criteria(should_update_chunks),
                    )
                    .with_system(
                        process_queue
                            .label(ChunkLoadingSystem::CreateChunks)
                            .after(ChunkLoadingSystem::UpdateChunks),
                    ),
            )
            .add_system_to_stage(CoreStage::Last, destroy_chunks)
            .add_system(process_task);
    }
}
