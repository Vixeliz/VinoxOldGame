use crate::networking::syncing::SentChunks;

use super::{
    generation::generate_chunk,
    storage::{insert_chunk, load_chunk, WorldDatabase},
};
use bevy::{
    ecs::{schedule::ShouldRun, system::SystemParam},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::FloatOrd,
};
use common::game::world::chunk::{
    ChunkComp, ChunkPos, CurrentChunks, RemoveChunk, SimulationDistance, ViewDistance,
};
use futures_lite::future;
use rand::Rng;

#[derive(Resource, Default)]
pub struct WorldSeed(pub u32);

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

#[derive(Default, Resource, Debug)]
pub struct ChunkQueue {
    pub create: Vec<IVec3>,
    pub remove: Vec<IVec3>,
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    // commands: Commands<'w, 's>,
    current_chunks: ResMut<'w, CurrentChunks>,
    // chunk_queue: ResMut<'w, ChunkQueue>,
    view_distance: Res<'w, ViewDistance>,
    chunk_query: Query<'w, 's, &'static ChunkComp>,
}

impl<'w, 's> ChunkManager<'w, 's> {
    // pub fn add_chunk_to_queue(&mut self, pos: IVec3) {
    //     self.chunk_queue.create.push(pos);
    // }

    pub fn get_chunks_around_chunk(
        &mut self,
        pos: IVec3,
        sent_chunks: &SentChunks,
    ) -> Vec<&ChunkComp> {
        let mut res = Vec::new();
        for x in -self.view_distance.horizontal..self.view_distance.horizontal {
            for y in -self.view_distance.vertical..self.view_distance.vertical {
                for z in -self.view_distance.horizontal..self.view_distance.horizontal {
                    let chunk_pos = IVec3::new(pos.x + x, pos.y + y, pos.z + z);
                    if !sent_chunks.chunks.contains(&chunk_pos) {
                        if let Some(entity) = self.current_chunks.get_entity(chunk_pos) {
                            if let Ok(chunk) = self.chunk_query.get(entity) {
                                res.push(chunk);
                            }
                        }
                    }
                }
            }
        }
        res.sort_unstable_by_key(|key| FloatOrd(key.pos.0.as_vec3().distance(pos.as_vec3())));

        res
    }
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
    database: Res<WorldDatabase>,
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
            if load_point.is_in_radius(
                chunk.pos.0,
                IVec2::new(-view_distance.horizontal, -view_distance.vertical),
                IVec2::new(view_distance.horizontal, view_distance.vertical),
            ) {
                continue;
            } else {
                commands.entity(entity).insert(RemoveChunk);
            }
        }
    }
}

pub fn unsend_chunks(
    chunks: Query<&ChunkComp>,
    mut load_points: Query<(&LoadPoint, &mut SentChunks)>,
    view_distance: Res<ViewDistance>,
) {
    for (load_point, mut sent_chunks) in load_points.iter_mut() {
        for chunk in chunks.iter() {
            if !load_point.is_in_radius(
                chunk.pos.0,
                IVec2::new(-view_distance.horizontal, -view_distance.vertical),
                IVec2::new(view_distance.horizontal, view_distance.vertical),
            ) {
                sent_chunks.chunks.remove(&chunk.pos.0);
            } else {
                continue;
            }
        }
    }
}

#[derive(Component)]
pub struct ChunkGenTask(Task<ChunkComp>);

pub fn process_task(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut ChunkGenTask)>,
    database: Res<WorldDatabase>,
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
    seed: Res<WorldSeed>,
) {
    let cloned_seed = seed.0;
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
                        chunk_data: generate_chunk(chunk_pos, cloned_seed),
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

pub struct ChunkGenerationPlugin;

impl Plugin for ChunkGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
            .insert_resource(ViewDistance {
                horizontal: 10,
                vertical: 4,
            })
            .insert_resource(SimulationDistance {
                width: 4,
                height: 4,
                depth: 4,
            })
            .insert_resource(WorldSeed(rand::thread_rng().gen_range(0..u32::MAX)))
            .add_system(clear_unloaded_chunks.with_run_criteria(should_update_chunks))
            .add_system(unsend_chunks.with_run_criteria(should_update_chunks))
            .add_system(generate_chunks_world.with_run_criteria(should_update_chunks))
            .add_system(process_queue.after(clear_unloaded_chunks))
            .add_system_to_stage(CoreStage::Last, destroy_chunks)
            .add_system(process_task);
    }
}
