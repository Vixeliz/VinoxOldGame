use bevy::prelude::*;
use common::networking::components::EntityBuffer;
use iyes_loopless::prelude::*;

use crate::components::GameState;

use super::{
    components::{ClientLobby, NetworkMapping},
    syncing::{client_send_naive_position, client_sync_players, get_id, lerp_new_location},
};

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(NetworkMapping::default())
            .insert_resource(ClientLobby::default())
            .insert_resource(EntityBuffer::default())
            .add_system(client_sync_players.run_in_state(GameState::Game))
            .add_system(get_id.run_in_state(GameState::Game))
            // .add_system_to_stage(
            //     CoreStage::Update,
            //     client_disconect.run_in_state(GameState::Game),
            // )
            .add_fixed_timestep_system(
                "network_update",
                0,
                client_send_naive_position.run_in_state(GameState::Game),
            )
            .add_system(lerp_new_location.run_in_state(GameState::Game));
    }
}
