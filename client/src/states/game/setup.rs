use std::f32::consts::PI;

use super::collision::plugin::CollisionPlugin;
use super::networking::plugin::NetworkingPlugin;
use super::rendering::meshing;
use super::rendering::plugin::RenderingPlugin;
use super::ui::plugin::UiPlugin;
use super::world::chunk::ChunkHandling;
use super::{
    input::plugin::InputPlugin,
    networking::{
        components::{ClientLobby, NetworkMapping},
        *,
    },
};
use belly::prelude::*;
use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_rapier3d::prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use common::networking::components::EntityBuffer;
use iyes_loopless::prelude::*;
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};

use crate::{
    components::{Game, GameState},
    systems::despawn_with,
};

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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

    let crosshair_handle: Handle<Image> = asset_server.load("crosshair.png");
    // let crosshair_handle = "crosshair.png";

    commands.add(eml! {
        <body s:padding="50px" s:position-type="absolute">
            <img src=crosshair_handle/>
        </body>
    });
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(UiPlugin)
            .add_plugin(NetworkingPlugin)
            .add_plugin(ChunkHandling)
            .add_plugin(InputPlugin)
            .add_enter_system(GameState::Game, setup)
            .add_exit_system(GameState::Game, despawn_with::<Game>)
            .add_plugin(CollisionPlugin)
            .add_plugin(RenderingPlugin);
    }
}
