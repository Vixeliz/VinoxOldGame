use crate::components::GameState;
use bevy::prelude::*;
use bevy_atmosphere::prelude::AtmospherePlugin;
use iyes_loopless::prelude::*;

use super::meshing::{process_queue, process_task, sort_faces, MeshChunkEvent};

pub struct RenderingPlugin;

impl Plugin for RenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AtmospherePlugin)
            .add_system(process_queue.run_in_state(GameState::Game))
            .add_system(process_task.run_in_state(GameState::Game))
            .add_system(sort_faces.run_in_state(GameState::Game))
            .add_event::<MeshChunkEvent>();
    }
}
