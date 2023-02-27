use std::f32::consts::PI;

use super::input;
use super::networking::{
    components::{ClientLobby, NetworkMapping},
    *,
};
use super::rendering::meshing;
use super::world::chunk::ChunkHandling;
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy_rapier3d::prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use common::networking::components::EntityBuffer;
use iyes_loopless::prelude::*;

use crate::{
    components::{Game, GameState},
    systems::despawn_with,
};

pub fn setup(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            color: Color::rgb_u8(178, 255, 238),
            illuminance: 2500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(0.0, 100.0, 0.0))
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        color: Color::rgb_u8(255, 251, 233),
        brightness: 1.0,
    });
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(ChunkHandling)
            .add_plugin(AtmospherePlugin)
            .insert_resource(RapierConfiguration { ..default() })
            .insert_resource(NetworkMapping::default())
            .insert_resource(ClientLobby::default())
            .insert_resource(EntityBuffer::default())
            .add_enter_system(GameState::Game, setup)
            .add_exit_system(GameState::Game, despawn_with::<Game>)
            .add_system(syncing::client_sync_players.run_in_state(GameState::Game))
            .add_system_to_stage(
                CoreStage::Update,
                syncing::client_disconect.run_in_state(GameState::Game),
            )
            .add_fixed_timestep_system(
                "network_update",
                0,
                syncing::client_send_naive_position.run_in_state(GameState::Game),
            )
            .add_system(syncing::lerp_new_location.run_in_state(GameState::Game))
            .add_system(input::camera_controller.run_in_state(GameState::Game))
            .add_system(meshing::process_queue.run_in_state(GameState::Game))
            .add_system(meshing::process_task.run_in_state(GameState::Game))
            .add_event::<crate::states::game::rendering::meshing::MeshChunkEvent>();
    }
}
