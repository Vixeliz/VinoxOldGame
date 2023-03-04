use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::{camera::CameraProjection, primitives::Frustum},
    window::CursorGrabMode,
};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_egui::EguiContext;
use bevy_rapier3d::{
    prelude::{
        Collider, CollisionGroups, Group, KinematicCharacterController,
        KinematicCharacterControllerOutput, QueryFilter, RapierConfiguration, RapierContext, Rot,
        SolverGroups, Vect, Velocity,
    },
    rapier::prelude::InteractionGroups,
};
use bevy_renet::renet::RenetClient;
use common::{
    game::world::chunk::{
        world_to_voxel, Chunk, ChunkComp, LoadableTypes, RawChunk, Voxel, VoxelVisibility,
        CHUNK_SIZE,
    },
    networking::components::{self, ClientChannel},
};
use renet_visualizer::{RenetClientVisualizer, RenetVisualizerStyle};

use super::{
    networking::components::ControlledPlayer,
    world::chunk::{world_to_chunk, CurrentChunks, DirtyChunk, PlayerChunk, ViewDistance},
};
use bevy_rapier3d::prelude::TOIStatus::Converged;

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
// #[derive(Component, Clone)]
// pub struct CameraController {
//     pub enabled: bool,
//     pub initialized: bool,
//     pub sensitivity: f32,
//     pub key_forward: KeyCode,
//     pub key_back: KeyCode,
//     pub key_left: KeyCode,
//     pub key_right: KeyCode,
//     pub key_up: KeyCode,
//     pub key_down: KeyCode,
//     pub key_run: KeyCode,
//     pub keyboard_key_enable_mouse: KeyCode,
//     pub walk_speed: f32,
//     pub run_speed: f32,
//     pub friction: f32,
//     pub pitch: f32,
//     pub yaw: f32,
//     pub velocity: Vec3,
// }

// impl CameraController {
//     pub fn print_controls(self) -> Self {
//         println!(
//             "
// ===============================
// ======= Camera Controls =======
// ===============================
//     {:?} - Forward
//     {:?} - Backward
//     {:?} - Left
//     {:?} - Right
//     {:?} - Up
//     {:?} - Down
//     {:?} - Run
//     {:?} - EnableMouse
// ",
//             self.key_forward,
//             self.key_back,
//             self.key_left,
//             self.key_right,
//             self.key_up,
//             self.key_down,
//             self.key_run,
//             self.keyboard_key_enable_mouse,
//         );
//         self
//     }
// }

// impl Default for CameraController {
//     fn default() -> Self {
//         Self {
//             enabled: true,
//             initialized: false,
//             sensitivity: 0.25,
//             key_forward: KeyCode::W,
//             key_back: KeyCode::S,
//             key_left: KeyCode::A,
//             key_right: KeyCode::D,
//             key_up: KeyCode::Space,
//             key_down: KeyCode::C,
//             key_run: KeyCode::LShift,
//             keyboard_key_enable_mouse: KeyCode::M,
//             walk_speed: 10.0,
//             run_speed: 25.0,
//             friction: 0.5,
//             pitch: 0.0,
//             yaw: 0.0,
//             velocity: Vec3::ZERO,
//         }
//     }
// }

// pub fn camera_controller(
//     time: Res<Time>,
//     mut mouse_events: EventReader<MouseMotion>,
//     mouse_button_input: Res<Input<MouseButton>>,
//     key_input: Res<Input<KeyCode>>,
//     mut move_toggled: Local<bool>,
//     mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
//     mut player_query: Query<
//         (
//             &mut KinematicCharacterController,
//             Option<&KinematicCharacterControllerOutput>,
//         ),
//         With<ControlledPlayer>,
//     >,
//     player_chunk: Res<PlayerChunk>,
//     current_chunks: Res<CurrentChunks>,
//     mut commands: Commands,
//     mut windows: ResMut<Windows>,
//     rapier_config: Res<RapierConfiguration>,
// ) {
//     let dt = time.delta_seconds();

//     if let Ok((mut transform, mut options)) = query.get_single_mut() {
//         if !options.initialized {
//             let (_roll, yaw, pitch) = transform.rotation.to_euler(EulerRot::ZYX);
//             options.yaw = yaw;
//             options.pitch = pitch;
//             options.initialized = true;
//             *move_toggled = !*move_toggled;
//             let window = windows.get_primary_mut().unwrap();
//             window.set_cursor_grab_mode(CursorGrabMode::Locked);
//             window.set_cursor_visibility(false);
//         }
//         if !options.enabled {
//             return;
//         }

//         if key_input.just_pressed(KeyCode::E) {
//             if let Some(chunk_entity) = current_chunks.get_entity(player_chunk.chunk_pos) {
//                 commands.entity(chunk_entity).insert(DirtyChunk);
//             }
//         }
//         // Handle key input
//         let mut axis_input = Vec3::ZERO;
//         if key_input.pressed(options.key_forward) {
//             axis_input.z += 1.0;
//         }
//         if key_input.pressed(options.key_back) {
//             axis_input.z -= 1.0;
//         }
//         if key_input.pressed(options.key_right) {
//             axis_input.x += 1.0;
//         }
//         if key_input.pressed(options.key_left) {
//             axis_input.x -= 1.0;
//         }
//         // if key_input.pressed(options.key_up) {
//         //     axis_input.y += 1.0;
//         // }
//         // if key_input.pressed(options.key_down) {
//         //     axis_input.y -= 1.0;
//         // }

//         if key_input.just_pressed(options.keyboard_key_enable_mouse) {
//             *move_toggled = !*move_toggled;
//             let window = windows.get_primary_mut().unwrap();
//             if *move_toggled {
//                 window.set_cursor_grab_mode(CursorGrabMode::Locked);
//                 window.set_cursor_visibility(false);
//             } else {
//                 window.set_cursor_grab_mode(CursorGrabMode::None);
//                 window.set_cursor_visibility(true);
//             }
//         }

//         // Apply movement update
//         if axis_input != Vec3::ZERO {
//             let max_speed = if key_input.pressed(options.key_run) {
//                 options.run_speed
//             } else {
//                 options.walk_speed
//             };
//             let pre_vel = options.velocity.y;
//             options.velocity = axis_input.normalize() * max_speed;
//             options.velocity.y = pre_vel;
//         } else {
//             let friction = options.friction.clamp(0.0, 1.0);
//             options.velocity *= 1.0 - friction;
//             if options.velocity.length_squared() < 1e-6 {
//                 options.velocity = Vec3::ZERO;
//             }
//         }
//         let mut forward = transform.forward();
//         forward.y = 0.0;
//         let right = transform.right();

//         // Handle mouse input
//         let mut mouse_delta = Vec2::ZERO;
//         if *move_toggled {
//             for mouse_event in mouse_events.iter() {
//                 mouse_delta += mouse_event.delta;
//             }
//         } else {
//             mouse_events.clear();
//         }

//         if mouse_delta != Vec2::ZERO {
//             let sensitivity = options.sensitivity;
//             let (pitch, yaw) = (
//                 (options.pitch - mouse_delta.y * 0.5 * sensitivity * dt).clamp(
//                     -0.99 * std::f32::consts::FRAC_PI_2,
//                     0.99 * std::f32::consts::FRAC_PI_2,
//                 ),
//                 options.yaw - mouse_delta.x * sensitivity * dt,
//             );

//             // Apply look update
//             transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, yaw, pitch);
//             options.pitch = pitch;
//             options.yaw = yaw;
//         }
//         if let Ok((mut player_controller, player_info)) = player_query.get_single_mut() {
//             let grounded = match player_info {
//                 Some(output) => output.grounded,
//                 None => false,
//             };
//             if grounded {
//             } else {
//                 options.velocity.y += rapier_config.gravity.y;
//             }

//             if key_input.just_pressed(KeyCode::Space) && grounded {
//                 options.velocity.y = 150.0;
//             }

//             let translation_delta = options.velocity.x * dt * right
//                 + options.velocity.z * dt * forward
//                 + options.velocity.y * dt * Vec3::Y;

//             player_controller.translation = Some(translation_delta);
//         }
//     }
// }

pub fn interact(
    mut commands: Commands,
    mut chunks: Query<&mut ChunkComp>,
    mouse_button_input: Res<Input<MouseButton>>,
    current_chunks: Res<CurrentChunks>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    view_distance: Res<ViewDistance>,
    loadable_types: Res<LoadableTypes>,
    rapier_context: Res<RapierContext>,
    windows: ResMut<Windows>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut client: ResMut<RenetClient>,
) {
    let mouse_left = mouse_button_input.just_pressed(MouseButton::Left);
    let mouse_right = mouse_button_input.just_pressed(MouseButton::Right);
    if mouse_left || mouse_right {
        if let Ok((camera, camera_transform)) = camera_query.get_single() {
            let ray = camera
                .viewport_to_world(
                    camera_transform,
                    windows
                        .get_primary()
                        .unwrap()
                        .cursor_position()
                        .unwrap_or(Vec2::new(0.0, 0.0)),
                )
                .unwrap();
            // Then cast the ray.
            let hit = rapier_context.cast_ray_and_get_normal(
                camera_transform.translation(),
                camera_transform.forward(),
                100.0,
                true,
                QueryFilter::only_fixed(),
            );
            if let Some((_, toi)) = hit {
                let point = if mouse_right {
                    toi.point + (toi.normal / Vec3::splat(2.0))
                } else {
                    toi.point - (toi.normal / Vec3::splat(2.0))
                };
                let pos = world_to_voxel(point);
                if let Some(chunk_entity) = current_chunks.get_entity(pos.0) {
                    if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
                        if mouse_right {
                            chunk.chunk_data.set_block(pos.1, "vinoxdirt".to_string());
                            let send_block = components::Commands::SentBlock {
                                chunk_pos: pos.0.into(),
                                voxel_pos: [pos.1.x as u8, pos.1.y as u8, pos.1.z as u8],
                                block_type: "vinoxdirt".to_string(),
                            };
                            let input_message = bincode::serialize(&send_block).unwrap();

                            client.send_message(ClientChannel::Commands, input_message);
                        } else {
                            chunk.chunk_data.set_block(pos.1, "air".to_string());
                            let send_block = components::Commands::SentBlock {
                                chunk_pos: pos.0.into(),
                                voxel_pos: [pos.1.x as u8, pos.1.y as u8, pos.1.z as u8],
                                block_type: "air".to_string(),
                            };
                            let input_message = bincode::serialize(&send_block).unwrap();

                            client.send_message(ClientChannel::Commands, input_message);
                        }
                        match pos.1.x {
                            1 => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(-1, 0, 0))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            CHUNK_SIZE => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(1, 0, 0))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            _ => {}
                        }
                        match pos.1.y {
                            1 => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(0, -1, 0))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            CHUNK_SIZE => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(0, 1, 0))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            _ => {}
                        }
                        match pos.1.z {
                            1 => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(0, 0, -1))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            CHUNK_SIZE => {
                                if let Some(neighbor_chunk) =
                                    current_chunks.get_entity(pos.0 + IVec3::new(0, 0, 1))
                                {
                                    commands.entity(neighbor_chunk).insert(DirtyChunk);
                                }
                            }
                            _ => {}
                        }

                        commands.entity(chunk_entity).insert(DirtyChunk);
                    }
                }
            }
        }
    }
}

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
        let mut window = windows.get_primary_mut().unwrap();
        window.set_cursor_grab_mode(CursorGrabMode::Locked);
        window.set_cursor_visibility(false);

        *local = true;
        let camera = {
            let perspective_projection = PerspectiveProjection {
                fov: std::f32::consts::PI / 4.0,
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
            .insert(Transform::from_xyz(10.1, 45.0, 10.0))
            .insert(GlobalTransform::default())
            .with_children(|c| {
                c.spawn((
                    GlobalTransform::default(),
                    Transform::from_xyz(0.0, -0.5, 0.0),
                    Collider::cylinder(0.8, 0.2),
                    SolverGroups::new(Group::GROUP_1, Group::GROUP_2),
                    CollisionGroups::new(Group::GROUP_1, Group::GROUP_2),
                ));
                c.spawn((FPSCamera::default(), camera, AtmosphereCamera::default()));
            });
    }
}

pub fn collision_movement_system(
    mut camera: Query<(Entity, &mut FPSCamera)>,
    player: Query<Entity, With<ControlledPlayer>>,
    mut transforms: Query<&mut Transform>,
    time: Res<Time>,
    rapier_context: Res<RapierContext>,
) {
    if let Ok((entity_camera, mut fps_camera)) = camera.get_single_mut() {
        let entity_player = player.single();

        let looking_at = Vec3::new(
            10.0 * fps_camera.phi.cos() * fps_camera.theta.sin(),
            10.0 * fps_camera.theta.cos(),
            10.0 * fps_camera.phi.sin() * fps_camera.theta.sin(),
        );

        let mut camera_t = transforms.get_mut(entity_camera).unwrap();
        camera_t.look_at(looking_at, Vec3::new(0.0, 1.0, 0.0));

        let shape = Collider::cylinder(0.745, 0.2);
        let feet_shape = Collider::cylinder(0.05, 0.2);

        let mut movement_left = fps_camera.velocity * time.delta().as_secs_f32();
        let leg_height = 0.26;

        let filter = QueryFilter {
            flags: Default::default(),
            groups: Some(CollisionGroups::new(Group::GROUP_1, Group::GROUP_2)),
            exclude_collider: None,
            exclude_rigid_body: None,
            predicate: None,
            ..default()
        };

        loop {
            if movement_left.length() <= 0.0 {
                break;
            }
            let mut player_transform = transforms.get_mut(entity_player).unwrap();
            let position = player_transform.translation - Vec3::new(0.0, 0.495, 0.0);

            match rapier_context.cast_shape(
                position,
                Rot::default(),
                movement_left,
                &shape,
                1.0,
                filter,
            ) {
                None => {
                    player_transform.translation =
                        position + movement_left + Vec3::new(0.0, 0.495, 0.0);
                    break;
                }
                Some((collision_entity, toi)) => {
                    if toi.status != Converged {
                        let unstuck_vector =
                            transforms.get(collision_entity).unwrap().translation - position;
                        transforms.get_mut(entity_player).unwrap().translation -=
                            unstuck_vector.normalize() * 0.01;
                        fps_camera.velocity = Vec3::new(0.0, 0.0, 0.0);
                        break;
                    }
                    movement_left -= movement_left.dot(toi.normal1) * toi.normal1;
                    fps_camera.velocity = movement_left / time.delta().as_secs_f32();
                }
            }
        }

        if fps_camera.velocity.y <= 0.0 {
            let position =
                transforms.get(entity_player).unwrap().translation - Vec3::new(0.0, 1.19, 0.0);

            if let Some((_, toi)) = rapier_context.cast_shape(
                position,
                Rot::default(),
                Vec3::new(0.0, -1.0, 0.0),
                &feet_shape,
                leg_height,
                filter,
            ) {
                transforms.get_mut(entity_player).unwrap().translation -=
                    Vec3::new(0.0, toi.toi - leg_height, 0.0);
                fps_camera.velocity.y = 0.0;
            }
        }
    }
}

#[derive(Resource)]
pub struct MouseSensitivity(pub f32);

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
    mut chunks: Query<&mut ChunkComp>,
    current_chunks: Res<CurrentChunks>,
    loadable_types: Res<LoadableTypes>,
) {
    if let Ok(translation) = player_position.get_single() {
        // if block_accessor.get_chunk_entity_or_queue(to_ddd(translation)).is_none() {
        //   return;
        // }
        let translation = translation.translation;

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
                    fps_camera.velocity.y = 7.0;
                }
            }

            movement = movement.normalize_or_zero();

            if fps_camera.velocity.y.abs() < 0.001 {
                *stationary_frames += 1;
            } else {
                *stationary_frames = 0;
            }

            let y = fps_camera.velocity.y;
            fps_camera.velocity.y = 0.0;
            fps_camera.velocity = movement;
            fps_camera.velocity *= 5.0;
            fps_camera.velocity.y = y;
            let chunk_pos = world_to_chunk(translation);

            if current_chunks.get_entity(chunk_pos).is_none() {
                return;
            }

            fps_camera.velocity.y -= 19.8 * time.delta().as_secs_f32().clamp(0.0, 0.1);
        }
    }
}

pub fn update_visualizer_system(
    mut egui_context: ResMut<EguiContext>,
    mut visualizer: ResMut<RenetClientVisualizer<200>>,
    client: Res<RenetClient>,
    mut show_visualizer: Local<bool>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    visualizer.add_network_info(client.network_info());
    if keyboard_input.just_pressed(KeyCode::F1) {
        *show_visualizer = !*show_visualizer;
    }
    if *show_visualizer {
        visualizer.show_window(egui_context.ctx_mut());
    }
}
