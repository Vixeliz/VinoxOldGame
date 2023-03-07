use bevy::prelude::*;
use bevy_rapier3d::prelude::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use iyes_loopless::prelude::*;

use crate::components::GameState;

use super::player::{collision_movement_system, interact};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .insert_resource(RapierConfiguration {
                gravity: Vec3::new(0.0, -25.0, 0.0),
                ..default()
            })
            .add_system(interact.run_in_state(GameState::Game))
            .add_system(collision_movement_system.run_in_state(GameState::Game));
    }
}
