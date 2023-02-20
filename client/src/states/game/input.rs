use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;

use super::networking::components::ControlledPlayer;
pub fn move_player(
    mut velocity_query: Query<&mut Velocity, With<ControlledPlayer>>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if let Ok(mut player_velocity) = velocity_query.get_single_mut() {
        let right = if input.pressed(KeyCode::D) { 1. } else { 0. };
        let left = if input.pressed(KeyCode::A) { 1. } else { 0. };
        player_velocity.linvel.x = (right - left) * 500. * time.delta_seconds();

        let forward = if input.pressed(KeyCode::W) { 1. } else { 0. };
        let back = if input.pressed(KeyCode::S) { 1. } else { 0. };
        player_velocity.linvel.z = (back - forward) * 500. * time.delta_seconds();

        let up = if input.pressed(KeyCode::Space) {
            1.
        } else {
            0.
        };
        let down = if input.pressed(KeyCode::C) { 1. } else { 0. };
        player_velocity.linvel.y = (up - down) * 500. * time.delta_seconds();
    }
}
