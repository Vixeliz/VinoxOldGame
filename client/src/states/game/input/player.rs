use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::{camera::CameraProjection, primitives::Frustum},
    window::CursorGrabMode,
};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, SolverGroups, Vect};
use common::game::world::chunk::{world_to_chunk, ChunkComp, CurrentChunks, LoadableTypes};

use crate::states::game::networking::components::ControlledPlayer;

#[derive(Component)]
pub struct FPSCamera {
    pub phi: f32,
    pub theta: f32,
    pub velocity: Vect,
}

impl Default for FPSCamera {
    fn default() -> Self {
        FPSCamera {
            phi: 0.0,
            theta: FRAC_PI_2,
            velocity: Vect::ZERO,
        }
    }
}

pub fn spawn_camera(
    mut commands: Commands,
    player_entity: Query<Entity, With<ControlledPlayer>>,
    mut local: Local<bool>,
    mut windows: ResMut<Windows>,
) {
    if *local {
        return;
    }
    if let Ok(player_entity) = player_entity.get_single() {
        let window = windows.get_primary_mut().unwrap();
        window.set_cursor_grab_mode(CursorGrabMode::Locked);
        window.set_cursor_visibility(false);

        *local = true;
        let camera = {
            let perspective_projection = PerspectiveProjection {
                fov: std::f32::consts::PI / 1.8,
                near: 0.001,
                far: 1000.0,
                aspect_ratio: 1.0,
            };
            let view_projection = perspective_projection.get_projection_matrix();
            let frustum = Frustum::from_view_projection(
                &view_projection,
                &Vec3::ZERO,
                &Vec3::Z,
                perspective_projection.far(),
            );
            Camera3dBundle {
                projection: Projection::Perspective(perspective_projection),
                frustum,
                ..default()
            }
        };
        commands
            .entity(player_entity)
            .insert(GlobalTransform::default())
            .with_children(|c| {
                c.spawn((
                    GlobalTransform::default(),
                    Transform::from_xyz(0.0, 1.0, 0.0),
                    Collider::cylinder(0.8, 0.2),
                    SolverGroups::new(Group::GROUP_1, Group::GROUP_2),
                    CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
                ));
                c.spawn((FPSCamera::default(), camera, AtmosphereCamera::default()));
            });
    }
}

#[derive(Resource)]
pub struct MouseSensitivity(pub f32);

#[allow(clippy::too_many_arguments)]
pub fn movement_input_system(
    mut player: Query<&mut FPSCamera>,
    player_position: Query<&Transform, With<ControlledPlayer>>,
    camera_transform: Query<&Transform, With<Camera>>,
    mut mouse_events: EventReader<MouseMotion>,
    mouse_sensitivity: Res<MouseSensitivity>,
    key_events: Res<Input<KeyCode>>,
    mut windows: ResMut<Windows>,
    time: Res<Time>,
    mut stationary_frames: Local<i32>,
    _chunks: Query<&mut ChunkComp>,
    current_chunks: Res<CurrentChunks>,
    _loadable_types: Res<LoadableTypes>,
) {
    if let Ok(translation) = player_position.get_single() {
        let translation = translation.translation;
        if current_chunks
            .get_entity(world_to_chunk(translation))
            .is_none()
        {
            return;
        }

        let window = windows.get_primary_mut().unwrap();
        let mut movement = Vec3::default();
        if let Ok(mut fps_camera) = player.get_single_mut() {
            let transform = camera_transform.single();

            if window.cursor_grab_mode() == CursorGrabMode::Locked {
                for MouseMotion { delta } in mouse_events.iter() {
                    fps_camera.phi += delta.x * mouse_sensitivity.0 * 0.003;
                    fps_camera.theta = (fps_camera.theta + delta.y * mouse_sensitivity.0 * 0.003)
                        .clamp(0.00005, PI - 0.00005);
                }

                if key_events.pressed(KeyCode::W) {
                    let mut fwd = transform.forward();
                    fwd.y = 0.0;
                    let fwd = fwd.normalize();
                    movement += fwd;
                }
                if key_events.pressed(KeyCode::A) {
                    movement += transform.left()
                }
                if key_events.pressed(KeyCode::D) {
                    movement += transform.right()
                }
                if key_events.pressed(KeyCode::S) {
                    let mut back = transform.back();
                    back.y = 0.0;
                    let back = back.normalize();
                    movement += back;
                }

                if key_events.pressed(KeyCode::Space) && *stationary_frames > 2 {
                    *stationary_frames = 0;
                    fps_camera.velocity.y = 12.0;
                }
            }

            movement = movement.normalize_or_zero();

            if fps_camera.velocity.y.abs() < 0.001 && *stationary_frames < 10 {
                *stationary_frames += 4;
            } else if *stationary_frames >= 0 {
                *stationary_frames -= 1;
            }

            let y = fps_camera.velocity.y;
            fps_camera.velocity.y = 0.0;
            fps_camera.velocity = movement;
            if key_events.pressed(KeyCode::LShift) {
                fps_camera.velocity *= 10.0;
            } else {
                fps_camera.velocity *= 5.0;
            }
            fps_camera.velocity.y = y;
            let chunk_pos = world_to_chunk(translation);

            if current_chunks.get_entity(chunk_pos).is_none() {
                return;
            }

            fps_camera.velocity.y -= 35.0 * time.delta().as_secs_f32().clamp(0.0, 0.1);
        }
    }
}
