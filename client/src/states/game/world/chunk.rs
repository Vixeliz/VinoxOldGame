use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;
use bimap::BiMap;
use common::game::world::chunk::{ChunkComp, RawChunk, CHUNK_SIZE};

use crate::states::game::networking::components::ControlledPlayer;

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
    pub mesh: Vec<(IVec3, RawChunk)>,
}

fn is_in_radius(pos: IVec3, min_bound: IVec3, max_bound: IVec3) -> Option<bool> {
    if (pos.x <= max_bound.x && pos.x >= min_bound.x)
        && (pos.y <= max_bound.y && pos.y >= min_bound.y)
        && (pos.z <= max_bound.z && pos.z >= min_bound.z)
    {
        return Some(true);
    } else {
        return Some(false);
    }
}

pub fn world_to_chunk(pos: Vec3) -> IVec3 {
    IVec3::new(
        (pos.x / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
        (pos.y / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
        (pos.z / (CHUNK_SIZE as f32 - 1.0)).floor() as i32,
    )
}

pub fn clear_unloaded_chunks(
    mut commands: Commands,
    mut current_chunks: ResMut<CurrentChunks>,
    view_distance: Res<ViewDistance>,
    player_query: Query<&Transform, With<ControlledPlayer>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        let mut changed_chunks = Vec::new();

        let player_chunk = world_to_chunk(player_transform.translation);

        for chunk_pos in current_chunks.chunks.keys() {
            if let Some(loaded) = is_in_radius(
                *chunk_pos,
                IVec3::new(
                    (-view_distance.width / 2) + player_chunk.x,
                    (-view_distance.height / 2) + player_chunk.y,
                    (-view_distance.depth / 2) + player_chunk.z,
                ),
                IVec3::new(
                    (view_distance.width / 2) + player_chunk.x,
                    (view_distance.height / 2) + player_chunk.y,
                    (view_distance.depth / 2) + player_chunk.z,
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
        }
    }
}

pub struct ChunkHandling;

impl Plugin for ChunkHandling {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
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
            .add_system_to_stage(CoreStage::PostUpdate, clear_unloaded_chunks);
    }
}
