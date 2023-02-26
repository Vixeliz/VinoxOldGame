use crate::networking::syncing::SentChunks;

use super::generation::generate_chunk;
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bimap::BiMap;
use common::{
    game::world::chunk::{ChunkComp, CHUNK_SIZE},
    networking::components::Player,
};
use futures_lite::future;
use std::collections::*;

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

#[derive(Resource, Default)]
pub struct CurrentLoadPoints {
    pub points: BiMap<IVec3, u64>,
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
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

#[derive(Default, Resource)]
pub struct SimulationDistance {
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

#[derive(Default, Resource)]
pub struct ChunkQueue {
    pub create: HashSet<IVec3>,
    pub remove: HashSet<IVec3>,
}

impl CurrentLoadPoints {
    fn is_in_radius(&self, pos: IVec3, min_bound: IVec3, max_bound: IVec3) -> Option<bool> {
        for point in self.points.left_values() {
            if (pos.x <= (max_bound.x + point.x) && pos.x >= (min_bound.x + point.x))
                && (pos.y <= (max_bound.y + point.y) && pos.y >= (min_bound.y + point.y))
                && (pos.z <= (max_bound.z + point.z) && pos.z >= (min_bound.z + point.z))
            {
                return Some(true);
            } else {
                return Some(false);
            }
        }
        None
    }
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    commands: Commands<'w, 's>,
    current_chunks: ResMut<'w, CurrentChunks>,
    chunk_queue: ResMut<'w, ChunkQueue>,
    current_load_points: ResMut<'w, CurrentLoadPoints>,
    view_distance: Res<'w, ViewDistance>,
    chunk_query: Query<'w, 's, &'static ChunkComp>,
}

impl<'w, 's> ChunkManager<'w, 's> {
    pub fn add_chunk_to_queue(&mut self, pos: IVec3) {
        self.chunk_queue.create.insert(pos);
    }
    pub fn get_chunks_around_chunk(&mut self, pos: IVec3) -> Vec<&ChunkComp> {
        let mut res = Vec::new();
        for x in -self.view_distance.width / 2..self.view_distance.width / 2 {
            for y in -self.view_distance.height / 2..self.view_distance.height / 2 {
                for z in -self.view_distance.depth / 2..self.view_distance.depth / 2 {
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

        res
    }

    pub fn world_to_chunk(&self, pos: Vec3) -> IVec3 {
        IVec3::new(
            (pos.x / CHUNK_SIZE as f32).round() as i32,
            (pos.y / CHUNK_SIZE as f32).round() as i32,
            (pos.z / CHUNK_SIZE as f32).round() as i32,
        )
    }

    pub fn add_point(&mut self, pos: IVec3, owner: u64) {
        self.current_load_points.points.insert(pos, owner);
    }
}

pub fn generate_chunks_world(
    view_distance: Res<ViewDistance>,
    current_load_points: Res<CurrentLoadPoints>,
    mut chunk_queue: ResMut<ChunkQueue>,
    current_chunks: ResMut<CurrentChunks>,
) {
    for point in current_load_points.points.left_values() {
        for x in -view_distance.width / 2..view_distance.width / 2 {
            for y in -view_distance.height / 2..view_distance.height / 2 {
                for z in -view_distance.depth / 2..view_distance.depth / 2 {
                    let pos = IVec3::new(point.x + x, point.y + y, point.z + z);
                    if current_chunks.get_entity(pos).is_some() {
                        break;
                    }
                    chunk_queue.create.insert(pos);
                }
            }
        }
    }
}

pub fn clear_unloaded_chunks(
    mut commands: Commands,
    mut current_chunks: ResMut<CurrentChunks>,
    current_load_points: Res<CurrentLoadPoints>,
    view_distance: Res<ViewDistance>,
    mut player_query: Query<&mut SentChunks, With<Player>>,
) {
    let mut changed_chunks = Vec::new();
    for chunk_pos in current_chunks.chunks.keys() {
        if let Some(loaded) = current_load_points.is_in_radius(
            *chunk_pos,
            IVec3::new(
                -view_distance.width / 2,
                -view_distance.height / 2,
                -view_distance.depth / 2,
            ),
            IVec3::new(
                view_distance.width / 2,
                view_distance.height / 2,
                view_distance.depth / 2,
            ),
        ) {
            if !loaded {
                commands
                    .get_entity(current_chunks.get_entity(*chunk_pos).unwrap())
                    .unwrap()
                    .despawn_recursive();
                changed_chunks.push(chunk_pos.clone());
            }
        }
    }
    for chunk_pos in changed_chunks {
        current_chunks.remove_entity(chunk_pos);
        for mut sent_chunks in player_query.iter_mut() {
            sent_chunks.chunks.remove(&chunk_pos);
        }
    }
}

#[derive(Component)]
pub struct ChunkGenTask(Task<ChunkComp>);

pub fn process_task(mut commands: Commands, mut chunk_query: Query<(Entity, &mut ChunkGenTask)>) {
    for (entity, mut chunk_task) in &mut chunk_query {
        if let Some(chunk) = future::block_on(future::poll_once(&mut chunk_task.0)) {
            commands.entity(entity).insert(chunk);
            commands.entity(entity).remove::<ChunkGenTask>();
        }
    }
}

// TODO: Check if a chunk already exist
pub fn process_queue(
    mut commands: Commands,
    mut chunk_queue: ResMut<ChunkQueue>,
    mut current_chunks: ResMut<CurrentChunks>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    chunk_queue
        .create
        .iter()
        .cloned()
        .map(|chunk_pos| {
            (
                chunk_pos,
                ChunkGenTask(task_pool.spawn(async move {
                    ChunkComp {
                        pos: chunk_pos,
                        chunk_data: generate_chunk(chunk_pos, 0),
                        dirty: false,
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
    chunk_queue.create.clear();
}

pub struct ChunkGenerationPlugin;

impl Plugin for ChunkGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
            .insert_resource(CurrentLoadPoints::default())
            .insert_resource(ViewDistance {
                width: 8,
                height: 6,
                depth: 8,
            })
            .insert_resource(SimulationDistance {
                width: 4,
                height: 4,
                depth: 4,
            })
            .add_system(process_queue)
            .add_system(generate_chunks_world)
            .add_system_to_stage(CoreStage::PostUpdate, clear_unloaded_chunks)
            .add_system(process_task);
    }
}
