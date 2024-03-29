use bevy::{ecs::schedule::ShouldRun, prelude::*, render::primitives::Aabb, utils::FloatOrd};
use bevy_rapier3d::prelude::Collider;

use common::game::world::chunk::{
    world_to_chunk, ChunkComp, ChunkPos, CurrentChunks, LoadableTypes, RawChunk, RemoveChunk,
    SimulationDistance, ViewDistance, CHUNK_BOUND, CHUNK_SIZE,
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
    pub aabb: Aabb,
    // pub collider: Collider,
}

#[derive(Bundle)]
pub struct ChunkCollider {
    pub collider: Collider,
}

pub struct CreateChunkEvent {
    pub pos: IVec3,
    pub raw_chunk: RawChunk,
}

pub struct SetBlockEvent {
    pub chunk_pos: IVec3,
    pub voxel_pos: UVec3,
    pub block_type: String,
}
pub struct UpdateChunkEvent {
    pub pos: IVec3,
}

#[derive(Default, Resource)]
pub struct ChunkQueue {
    pub mesh: Vec<(IVec3, RawChunk)>,
    pub remove: Vec<IVec3>,
}

#[derive(Default, Resource)]
pub struct PlayerChunk {
    pub chunk_pos: IVec3,
}

#[derive(Default, Resource)]
pub struct PlayerChangedPos {
    pub pos: Vec3,
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
    mut player_changed: ResMut<PlayerChangedPos>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        let new_chunk = world_to_chunk(player_transform.translation);
        if new_chunk != player_chunk.chunk_pos {
            player_chunk.chunk_pos = new_chunk;
        }
        if player_transform.translation.distance(player_changed.pos) >= 0.25 {
            player_changed.pos = player_transform.translation;
        }
    }
}

pub fn delete_chunks(
    mut current_chunks: ResMut<CurrentChunks>,
    mut commands: Commands,
    chunks: Query<(Entity, &ChunkComp), With<RemoveChunk>>,
) {
    for (chunk_entity, chunk_pos) in chunks.iter() {
        current_chunks.remove_entity(chunk_pos.pos.0);
        commands.entity(chunk_entity).despawn_recursive();
    }
}

pub fn should_update_chunks(player_chunk: Res<PlayerChunk>) -> ShouldRun {
    if player_chunk.is_changed() {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

pub fn set_block(
    mut commands: Commands,
    mut event: EventReader<SetBlockEvent>,
    current_chunks: Res<CurrentChunks>,
    mut chunks: Query<&mut ChunkComp>,
) {
    for evt in event.iter() {
        if let Some(chunk_entity) = current_chunks.get_entity(evt.chunk_pos) {
            if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
                chunk.chunk_data.add_block_state(&evt.block_type);
                chunk
                    .chunk_data
                    .set_block(evt.voxel_pos, evt.block_type.clone());

                match evt.voxel_pos.x {
                    1 => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(-1, 0, 0))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    CHUNK_SIZE => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(1, 0, 0))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    _ => {}
                }
                match evt.voxel_pos.y {
                    1 => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(0, -1, 0))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    CHUNK_SIZE => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(0, 1, 0))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    _ => {}
                }
                match evt.voxel_pos.z {
                    1 => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(0, 0, -1))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    CHUNK_SIZE => {
                        if let Some(neighbor_chunk) =
                            current_chunks.get_entity(evt.chunk_pos + IVec3::new(0, 0, 1))
                        {
                            commands.entity(neighbor_chunk).insert(DirtyChunk);
                        }
                    }
                    _ => {}
                }

                commands.entity(chunk_entity).insert(DirtyChunk);
            }
        }
    }
}

pub fn clear_unloaded_chunks(
    mut commands: Commands,
    view_distance: Res<ViewDistance>,
    player_chunk: Res<PlayerChunk>,
    chunks: Query<&ChunkComp>,
    current_chunks: Res<CurrentChunks>,
) {
    for chunk_pos in chunks.iter() {
        if !player_chunk.is_in_radius(
            chunk_pos.pos.0,
            IVec2::new(-view_distance.horizontal, -view_distance.vertical),
            IVec2::new(view_distance.horizontal, view_distance.vertical),
        ) {
            commands
                .entity(current_chunks.get_entity(chunk_pos.pos.0).unwrap())
                .insert(RemoveChunk);
        }
    }
}

// Dirty chunks get marked in the following cases. A new neighbor spawns by them, the terrain is modified, or if a neighbor disapears
// This runs first then we remesh
#[allow(clippy::type_complexity)]
pub fn update_borders(
    mut commands: Commands,
    current_chunks: ResMut<CurrentChunks>,
    mut chunk_set: ParamSet<(
        Query<(&ChunkComp, Entity), With<DirtyChunk>>,
        Query<&mut ChunkComp>,
    )>,
    mut mesh_event: EventWriter<MeshChunkEvent>,
    view_distance: Res<ViewDistance>,
    player_chunk: Res<PlayerChunk>,
) {
    let mut dirty_chunk_positions = Vec::new();
    let mut sorted_chunk_positions = Vec::new();
    for dirty_chunk in chunk_set.p0().iter() {
        sorted_chunk_positions.push(dirty_chunk.0.pos.0);
    }

    sorted_chunk_positions.sort_unstable_by_key(|key| {
        FloatOrd(key.as_vec3().distance(player_chunk.chunk_pos.as_vec3()))
    });

    for dirty_chunk in sorted_chunk_positions.iter() {
        if dirty_chunk_positions.len() > 24 {
            break;
        }
        dirty_chunk_positions.push(*dirty_chunk);
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
                let mut new_chunk: RawChunk = RawChunk::default();
                if let Ok(chunk_data) = chunk_set.p1().get_many_mut(neighbor_entities) {
                    if chunk_data[0].chunk_data.palette == vec!["air".to_string()] {
                        commands.entity(dirty_entity).remove::<DirtyChunk>();
                        continue;
                    }
                    // TODO: Try to figure out a better way to do this
                    let mut chunk_data_cloned = chunk_data.map(|x| x.chunk_data.clone());
                    for z in 0..=CHUNK_BOUND {
                        for y in 0..=CHUNK_BOUND {
                            for x in 0..=CHUNK_BOUND {
                                match (x, y, z) {
                                    (1..=CHUNK_SIZE, CHUNK_BOUND, 1..=CHUNK_SIZE) => {
                                        let block_string = chunk_data_cloned[2]
                                            .get_block(UVec3::new(x, 1, z))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (1..=CHUNK_SIZE, 0, 1..=CHUNK_SIZE) => {
                                        let block_string = chunk_data_cloned[1]
                                            .get_block(UVec3::new(x, CHUNK_SIZE, z))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (0, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                        let block_string = chunk_data_cloned[3]
                                            .get_block(UVec3::new(CHUNK_SIZE, y, z))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (CHUNK_BOUND, 1..=CHUNK_SIZE, 1..=CHUNK_SIZE) => {
                                        let block_string = chunk_data_cloned[4]
                                            .get_block(UVec3::new(1, y, z))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, 0) => {
                                        let block_string = chunk_data_cloned[5]
                                            .get_block(UVec3::new(x, y, CHUNK_SIZE))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (1..=CHUNK_SIZE, 1..=CHUNK_SIZE, CHUNK_BOUND) => {
                                        let block_string = chunk_data_cloned[6]
                                            .get_block(UVec3::new(x, y, 1))
                                            .unwrap();
                                        chunk_data_cloned[0].add_block_state(&block_string);
                                        chunk_data_cloned[0]
                                            .set_block(UVec3::new(x, y, z), block_string);
                                    }
                                    (_, _, _) => {}
                                };
                            }
                        }
                    }
                    new_chunk = chunk_data_cloned[0].clone();
                }
                let mut chunk_set = chunk_set.p1();
                let mut chunk_data = chunk_set.get_mut(neighbor_entities[0]).unwrap();
                chunk_data.chunk_data = new_chunk.to_owned();
                mesh_event.send(MeshChunkEvent {
                    pos: *dirty_chunk_pos,
                });
                commands.entity(dirty_entity).remove::<DirtyChunk>();
            }
        } else {
            // let dirty_entity = current_chunks.get_entity(*dirty_chunk_pos).unwrap();
            // commands.entity(dirty_entity).remove::<DirtyChunk>();
        }
    }
}

#[allow(clippy::nonminimal_bool)]
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

pub struct ChunkHandling;

impl Plugin for ChunkHandling {
    fn build(&self, app: &mut App) {
        app.insert_resource(CurrentChunks::default())
            .insert_resource(ChunkQueue::default())
            .insert_resource(PlayerChunk::default())
            .insert_resource(PlayerChangedPos::default())
            .insert_resource(ViewDistance {
                horizontal: 10,
                vertical: 4,
            })
            .insert_resource(SimulationDistance {
                width: 4,
                height: 4,
                depth: 4,
            })
            .add_system(update_player_location)
            .add_system(update_borders.after(update_player_location))
            .add_system(receive_chunks.after(update_borders))
            .add_system(set_block.after(update_borders))
            .add_system(
                clear_unloaded_chunks
                    .after(receive_chunks)
                    .with_run_criteria(should_update_chunks),
            )
            .add_system(build_mesh.after(clear_unloaded_chunks))
            .add_system_to_stage(CoreStage::Last, delete_chunks)
            .add_event::<UpdateChunkEvent>()
            .add_event::<SetBlockEvent>()
            .add_event::<CreateChunkEvent>();
    }
}
