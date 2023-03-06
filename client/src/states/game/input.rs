use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::{camera::CameraProjection, primitives::Frustum},
    window::CursorGrabMode,
};
use bevy_atmosphere::prelude::AtmosphereCamera;
use bevy_egui::EguiContext;
use bevy_rapier3d::prelude::{
    Collider, CollisionGroups, Group, QueryFilter, RapierContext, Rot, SolverGroups, Vect,
};
use bevy_renet::renet::RenetClient;
use common::{
    game::world::chunk::{world_to_voxel, ChunkComp, LoadableTypes, CHUNK_SIZE},
    networking::components::{self, ClientChannel},
};
use renet_visualizer::RenetClientVisualizer;

use super::{
    networking::components::ControlledPlayer,
    world::chunk::{world_to_chunk, CurrentChunks, DirtyChunk},
};
use bevy_rapier3d::prelude::TOIStatus::Converged;

// HEAVILY TEMPORARY BOYFRIEND WANTED ITEMS TO BUILD WITH
#[derive(Default, Clone)]
pub enum CurrentItem {
    Grass,
    Dirt,
    #[default]
    Greybrick,
    Moss,
    Wood,
    Concrete,
    Cobblestone,
}

#[allow(clippy::too_many_arguments)]
pub fn interact(
    mut commands: Commands,
    mut chunks: Query<&mut ChunkComp>,
    mouse_button_input: Res<Input<MouseButton>>,
    keys: Res<Input<KeyCode>>,
    current_chunks: Res<CurrentChunks>,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    rapier_context: Res<RapierContext>,
    mut client: ResMut<RenetClient>,
    player_position: Query<&Transform, With<ControlledPlayer>>,
    mut current_item: Local<CurrentItem>,
) {
    let item_string = match current_item.clone() {
        CurrentItem::Grass => "vinoxgrass",
        CurrentItem::Dirt => "vinoxdirt",
        CurrentItem::Concrete => "vinoxconcrete",
        CurrentItem::Cobblestone => "vinoxcobblestone",
        CurrentItem::Moss => "vinoxmoss",
        CurrentItem::Wood => "vinoxwood",
        CurrentItem::Greybrick => "vinoxgreybrick",
    };

    for key in keys.get_just_pressed() {
        match key {
            KeyCode::Key1 => *current_item = CurrentItem::Dirt,
            KeyCode::Key2 => *current_item = CurrentItem::Grass,
            KeyCode::Key3 => *current_item = CurrentItem::Concrete,
            KeyCode::Key4 => *current_item = CurrentItem::Cobblestone,
            KeyCode::Key5 => *current_item = CurrentItem::Moss,
            KeyCode::Key6 => *current_item = CurrentItem::Wood,
            KeyCode::Key7 => *current_item = CurrentItem::Greybrick,
            _ => {}
        }
    }

    let mouse_left = mouse_button_input.just_pressed(MouseButton::Left);
    let mouse_right = mouse_button_input.just_pressed(MouseButton::Right);
    if let Ok(player_transform) = player_position.get_single() {
        if mouse_left || mouse_right {
            if let Ok(camera_transform) = camera_query.get_single() {
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
                                if (point.x <= player_transform.translation.x - 0.5
                                    || point.x >= player_transform.translation.x + 0.5)
                                    || (point.z <= player_transform.translation.z - 0.5
                                        || point.z >= player_transform.translation.z + 0.5)
                                    || (point.y <= player_transform.translation.y - 2.0
                                        || point.y >= player_transform.translation.y + 1.0)
                                {
                                    chunk.chunk_data.add_block_state(&item_string.to_string());
                                    chunk.chunk_data.set_block(pos.1, item_string.to_string());
                                    let send_block = components::Commands::SentBlock {
                                        chunk_pos: pos.0.into(),
                                        voxel_pos: [pos.1.x as u8, pos.1.y as u8, pos.1.z as u8],
                                        block_type: item_string.to_string(),
                                    };
                                    let input_message = bincode::serialize(&send_block).unwrap();

                                    client.send_message(ClientChannel::Commands, input_message);
                                }
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
