use crate::components::GameState;
use bevy::prelude::*;
use bevy_atmosphere::prelude::AtmospherePlugin;
use iyes_loopless::prelude::*;

use super::meshing::{
    create_chunk_material, process_queue, process_task, sort_chunks, sort_faces, ChunkMaterial,
    MeshChunkEvent, SortFaces,
};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AtmospherePlugin)
            .insert_resource(ChunkMaterial::default())
            .add_enter_system(GameState::Game, create_chunk_material)
            .add_system(process_queue.run_in_state(GameState::Game))
            .add_system(process_task.run_in_state(GameState::Game))
            .add_system(sort_faces.run_in_state(GameState::Game))
            .add_system(sort_chunks.run_in_state(GameState::Game))
            .add_event::<MeshChunkEvent>()
            .add_event::<SortFaces>();
    }
}
