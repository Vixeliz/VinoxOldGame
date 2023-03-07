use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::components::GameState;

use super::player::{
    movement_input_system, spawn_camera, update_visualizer_system, MouseSensitivity,
};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MouseSensitivity(1.0))
            .add_system(spawn_camera.run_in_state(GameState::Game))
            .add_system(movement_input_system.run_in_state(GameState::Game))
            .add_system(update_visualizer_system.run_in_state(GameState::Game));
    }
}
