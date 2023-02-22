use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use common::game::world::chunk::{Chunk, CHUNK_SIZE};
use std::collections::*;

use super::generation::generate_chunk;
use futures_lite::future;

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
    pub points: Vec<IVec3>,
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
    pub create: Vec<IVec3>,
    pub remove: Vec<IVec3>,
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    commands: Commands<'w, 's>,
    current_chunks: ResMut<'w, CurrentChunks>,
    chunk_queue: ResMut<'w, ChunkQueue>,
    current_load_points: ResMut<'w, CurrentLoadPoints>,
    view_distance: Res<'w, ViewDistance>,
    chunk_query: Query<'w, 's, &'static Chunk>,
}

impl<'w, 's> ChunkManager<'w, 's> {
    pub fn add_chunk_to_queue(&mut self, pos: IVec3) {
        self.chunk_queue.create.push(pos);
    }
    pub fn get_chunks_around_chunk(&mut self, pos: IVec3) -> Vec<&Chunk> {
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

    pub fn add_point(&mut self, pos: IVec3) {
        self.current_load_points.points.push(pos);
    }
}

pub fn generate_chunks_world(
    view_distance: Res<ViewDistance>,
    current_load_points: Res<CurrentLoadPoints>,
    mut chunk_queue: ResMut<ChunkQueue>,
    current_chunks: ResMut<CurrentChunks>,
) {
    for point in current_load_points.points.iter() {
        for x in -view_distance.width / 2..view_distance.width / 2 {
            for y in -view_distance.height / 2..view_distance.height / 2 {
                for z in -view_distance.depth / 2..view_distance.depth / 2 {
                    let pos = IVec3::new(point.x + x, point.y + y, point.z + z);
                    if current_chunks.get_entity(pos).is_some() {
                        break;
                    }
                    chunk_queue.create.push(pos);
                }
            }
        }
    }
}

#[derive(Component)]
pub struct ChunkGenTask(Task<Chunk>);

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
                    Chunk {
                        pos: chunk_pos,
                        chunk_data: generate_chunk(chunk_pos, 0),
                        dirty: false,
                        entities: Vec::new(),
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
                width: 6,
                height: 6,
                depth: 6,
            })
            .insert_resource(SimulationDistance {
                width: 4,
                height: 4,
                depth: 4,
            })
            .add_system(process_queue)
            .add_system(generate_chunks_world)
            .add_system(process_task);
    }
}
