use bevy::{
    input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy_rapier3d::prelude::Velocity;

use super::{
    networking::components::ControlledPlayer,
    world::chunk::{DirtyChunks, PlayerChunk},
};
// pub fn move_player(
//     mut velocity_query: Query<&mut Velocity, With<ControlledPlayer>>,
//     input: Res<Input<KeyCode>>,
//     time: Res<Time>,
// ) {
//     if let Ok(mut player_velocity) = velocity_query.get_single_mut() {
//         let right = if input.pressed(KeyCode::D) { 1. } else { 0. };
//         let left = if input.pressed(KeyCode::A) { 1. } else { 0. };
//         player_velocity.linvel.x = (right - left) * 500. * time.delta_seconds();

//         let forward = if input.pressed(KeyCode::W) { 1. } else { 0. };
//         let back = if input.pressed(KeyCode::S) { 1. } else { 0. };
//         player_velocity.linvel.z = (back - forward) * 500. * time.delta_seconds();

//         let up = if input.pressed(KeyCode::Space) {
//             1.
//         } else {
//             0.
//         };
//         let down = if input.pressed(KeyCode::C) { 1. } else { 0. };
//         player_velocity.linvel.y = (up - down) * 500. * time.delta_seconds();
//     }
// }

// Under mit license from: https://github.com/DGriffin91/bevy_basic_camera
/// Provides basic movement functionality to the attached camera
#[derive(Component, Clone)]
pub struct CameraController {
    pub enabled: bool,
    pub initialized: bool,
    pub sensitivity: f32,
    pub key_forward: KeyCode,
    pub key_back: KeyCode,
    pub key_left: KeyCode,
    pub key_right: KeyCode,
    pub key_up: KeyCode,
    pub key_down: KeyCode,
    pub key_run: KeyCode,
    pub mouse_key_enable_mouse: MouseButton,
    pub keyboard_key_enable_mouse: KeyCode,
    pub walk_speed: f32,
    pub run_speed: f32,
    pub friction: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub velocity: Vec3,
}

impl CameraController {
    pub fn print_controls(self) -> Self {
        println!(
            "
===============================
======= Camera Controls =======
===============================
    {:?} - Forward
    {:?} - Backward
    {:?} - Left
    {:?} - Right
    {:?} - Up
    {:?} - Down
    {:?} - Run
    {:?}/{:?} - EnableMouse
",
            self.key_forward,
            self.key_back,
            self.key_left,
            self.key_right,
            self.key_up,
            self.key_down,
            self.key_run,
            self.mouse_key_enable_mouse,
            self.keyboard_key_enable_mouse,
        );
        self
    }
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 0.25,
            key_forward: KeyCode::W,
            key_back: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::Space,
            key_down: KeyCode::C,
            key_run: KeyCode::LShift,
            mouse_key_enable_mouse: MouseButton::Left,
            keyboard_key_enable_mouse: KeyCode::M,
            walk_speed: 500.0,
            run_speed: 1500.0,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

pub fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_button_input: Res<Input<MouseButton>>,
    key_input: Res<Input<KeyCode>>,
    mut move_toggled: Local<bool>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
    mut velocity_query: Query<&mut Velocity, With<ControlledPlayer>>,
    mut dirty_chunks: ResMut<DirtyChunks>,
    player_chunk: Res<PlayerChunk>,
) {
    let dt = time.delta_seconds();

    if let Ok((mut transform, mut options)) = query.get_single_mut() {
        if !options.initialized {
            let (_roll, yaw, pitch) = transform.rotation.to_euler(EulerRot::ZYX);
            options.yaw = yaw;
            options.pitch = pitch;
            options.initialized = true;
        }
        if !options.enabled {
            return;
        }

        if key_input.just_pressed(KeyCode::E) {
            dirty_chunks.mark_dirty(player_chunk.chunk_pos);
        }
        // Handle key input
        let mut axis_input = Vec3::ZERO;
        if key_input.pressed(options.key_forward) {
            axis_input.z += 1.0;
        }
        if key_input.pressed(options.key_back) {
            axis_input.z -= 1.0;
        }
        if key_input.pressed(options.key_right) {
            axis_input.x += 1.0;
        }
        if key_input.pressed(options.key_left) {
            axis_input.x -= 1.0;
        }
        if key_input.pressed(options.key_up) {
            axis_input.y += 1.0;
        }
        if key_input.pressed(options.key_down) {
            axis_input.y -= 1.0;
        }
        if key_input.just_pressed(options.keyboard_key_enable_mouse) {
            *move_toggled = !*move_toggled;
        }

        // Apply movement update
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(options.key_run) {
                options.run_speed
            } else {
                options.walk_speed
            };
            options.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = options.friction.clamp(0.0, 1.0);
            options.velocity *= 1.0 - friction;
            if options.velocity.length_squared() < 1e-6 {
                options.velocity = Vec3::ZERO;
            }
        }
        let mut forward = transform.forward();
        forward.y = 0.0;
        let right = transform.right();
        let translation_delta = options.velocity.x * dt * right
            + options.velocity.y * dt * Vec3::Y
            + options.velocity.z * dt * forward;

        // Handle mouse input
        let mut mouse_delta = Vec2::ZERO;
        if mouse_button_input.pressed(options.mouse_key_enable_mouse) || *move_toggled {
            for mouse_event in mouse_events.iter() {
                mouse_delta += mouse_event.delta;
            }
        } else {
            mouse_events.clear();
        }

        if mouse_delta != Vec2::ZERO {
            let sensitivity = options.sensitivity;
            let (pitch, yaw) = (
                (options.pitch - mouse_delta.y * 0.5 * sensitivity * dt).clamp(
                    -0.99 * std::f32::consts::FRAC_PI_2,
                    0.99 * std::f32::consts::FRAC_PI_2,
                ),
                options.yaw - mouse_delta.x * sensitivity * dt,
            );

            // Apply look update
            transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
            options.pitch = pitch;
            options.yaw = yaw;
        }
        if let Ok(mut player_velocity) = velocity_query.get_single_mut() {
            player_velocity.linvel = translation_delta;
        }
    }
}
