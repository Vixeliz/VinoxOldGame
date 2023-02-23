use super::input;
use super::networking::{
    components::{ClientLobby, NetworkMapping},
    *,
};
use bevy::prelude::*;
use common::networking::components::EntityBuffer;
use iyes_loopless::prelude::*;

use crate::{
    components::{Game, GameState},
    systems::despawn_with,
};

pub fn setup(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.75,
    });
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NetworkMapping::default())
            .insert_resource(ClientLobby::default())
            .insert_resource(EntityBuffer::default())
            .add_enter_system(GameState::Game, setup)
            .add_exit_system(GameState::Game, despawn_with::<Game>)
            .add_system(syncing::client_sync_players.run_in_state(GameState::Game))
            .add_fixed_timestep_system(
                "network_update",
                0,
                syncing::client_send_naive_position.run_in_state(GameState::Game),
            )
            .add_system(syncing::lerp_new_location.run_in_state(GameState::Game))
            .add_system(input::camera_controller.run_in_state(GameState::Game));
    }
}
