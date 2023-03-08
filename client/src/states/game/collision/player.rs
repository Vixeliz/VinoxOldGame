use bevy::prelude::*;

use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, QueryFilter, RapierContext, Rot};
use bevy_renet::renet::RenetClient;
use common::{
    game::world::chunk::{voxel_to_world, world_to_voxel, ChunkComp, CurrentChunks, CHUNK_SIZE},
    networking::components::{self, ClientChannel},
};

use crate::states::game::{
    input::player::FPSCamera,
    networking::{components::ControlledPlayer, syncing::HighLightCube},
    world::chunk::DirtyChunk,
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
    Glass,
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
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
    mut cube_position: Query<
        (&mut Transform, &mut Visibility),
        (With<HighLightCube>, Without<ControlledPlayer>),
    >,
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
        CurrentItem::Glass => "vinoxglass",
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
            KeyCode::Key8 => *current_item = CurrentItem::Glass,
            _ => {}
        }
    }

    let mouse_left = mouse_button_input.just_pressed(MouseButton::Left);
    let mouse_right = mouse_button_input.just_pressed(MouseButton::Right);
    if let Ok(player_transform) = player_position.get_single() {
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
                    if let Ok((mut block_transform, mut block_visibility)) =
                        cube_position.get_single_mut()
                    {
                        if !block_visibility.is_visible {
                            block_visibility.toggle();
                        }
                        block_transform.translation =
                            voxel_to_world(pos.1, pos.0) - Vec3::splat(0.5);
                    }
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
                        } else if mouse_left {
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
            } else if let Ok((_, mut block_visibility)) = cube_position.get_single_mut() {
                if block_visibility.is_visible {
                    block_visibility.toggle();
                }
            }
        }
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
