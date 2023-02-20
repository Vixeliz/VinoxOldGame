use std::time::Duration;

use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_easings::{Ease, EaseMethod, EasingType};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{
    greedy_quads, visible_block_faces, GreedyQuadsBuffer, MergeVoxel, UnitQuadBuffer,
    Voxel as MeshableVoxel, VoxelVisibility, RIGHT_HANDED_Y_UP_CONFIG,
};
use common::{
    game::{
        bundles::{ColliderBundle, PlayerBundleBuilder},
        world::chunk::{ChunkShape, Voxel, CHUNK_SIZE},
    },
    networking::components::{
        ClientChannel, EntityBuffer, LevelData, NetworkedEntities, Player, PlayerPos,
        ServerChannel, ServerMessages,
    },
};

use crate::states::game::networking::components::ControlledPlayer;

use super::components::{ClientLobby, NetworkMapping, PlayerInfo};

pub fn client_sync_players(
    mut cmd1: Commands,
    mut cmd2: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut entity_buffer: ResMut<EntityBuffer>,
    asset_server: Res<AssetServer>,
    player_builder: Res<PlayerBundleBuilder>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let client_id = client.client_id();
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                translation,
                entity,
                yaw,
                pitch,
            } => {
                let mut client_entity = cmd1.spawn_empty();
                if client_id == id {
                    println!("You connected.");
                    client_entity
                        .insert(player_builder.build(translation.into(), id))
                        .insert(ControlledPlayer);
                } else {
                    println!("Player {} connected.", id);
                    client_entity.insert(player_builder.build(translation.into(), id));
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity.id());
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {} disconnected.", id);
                if let Some(PlayerInfo {
                    server_entity,
                    client_entity,
                }) = lobby.players.remove(&id)
                {
                    cmd1.entity(client_entity).despawn();
                    network_mapping.0.remove(&server_entity);
                }
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::NetworkedEntities) {
        let networked_entities: NetworkedEntities = bincode::deserialize(&message).unwrap();
        let arr_len = entity_buffer.entities.len() - 1;
        entity_buffer.entities.rotate_left(1);
        entity_buffer.entities[arr_len] = networked_entities;
    }

    while let Some(message) = client.receive_message(ServerChannel::LevelData) {
        let level_data: LevelData = bincode::deserialize(&message).unwrap();
        match level_data {
            LevelData::ChunkCreate { chunk_data, pos } => {
                println!("Recieved chunk {:?}", pos);
                let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

                // Simple meshing works on web and makes texture atlases easier. However I may look into greedy meshing in future
                let mut buffer = UnitQuadBuffer::new();
                visible_block_faces(
                    &chunk_data.voxels,
                    &ChunkShape {},
                    [0; 3],
                    [21; 3],
                    &faces,
                    &mut buffer,
                );
                let num_indices = buffer.num_quads() * 6;
                let num_vertices = buffer.num_quads() * 4;
                let mut indices = Vec::with_capacity(num_indices);
                let mut positions = Vec::with_capacity(num_vertices);
                let mut normals = Vec::with_capacity(num_vertices);
                let mut tex_coords = Vec::with_capacity(num_vertices);
                let mut ao = Vec::with_capacity(num_vertices);
                for (group, face) in buffer.groups.into_iter().zip(faces.into_iter()) {
                    for quad in group.into_iter() {
                        indices.extend_from_slice(&face.quad_mesh_indices(positions.len() as u32));
                        positions.extend_from_slice(&face.quad_mesh_positions(&quad.into(), 1.0));
                        normals.extend_from_slice(&face.quad_mesh_normals());
                        ao.extend_from_slice(&face.quad_mesh_ao(&quad.into()));
                        let mut face_tex = face.tex_coords(
                            RIGHT_HANDED_Y_UP_CONFIG.u_flip_face,
                            true,
                            &quad.into(),
                        );
                        let [x, y, z] = quad.minimum;
                        let i = ChunkShape::linearize([x, y, z]);
                        let voxel_type = chunk_data.voxels[i as usize];
                        let tile_size = 64.0;
                        let texture_size = 1024.0;
                        match voxel_type {
                            Voxel((1, true)) => {
                                let tile_offset = 10.0;
                                face_tex[0][0] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[0][1] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[1][0] = (tile_offset * tile_size) / texture_size;
                                face_tex[1][1] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[2][0] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[2][1] = (tile_offset * tile_size) / texture_size;
                                face_tex[3][0] = (tile_offset * tile_size) / texture_size;
                                face_tex[3][1] = (tile_offset * tile_size) / texture_size;
                            }
                            Voxel((2, true)) => {
                                let tile_offset = 16.0;
                                face_tex[0][0] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[0][1] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[1][0] = (tile_offset * tile_size) / texture_size;
                                face_tex[1][1] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[2][0] = ((tile_offset - 1.0) * tile_size) / texture_size;
                                face_tex[2][1] = (tile_offset * tile_size) / texture_size;
                                face_tex[3][0] = (tile_offset * tile_size) / texture_size;
                                face_tex[3][1] = (tile_offset * tile_size) / texture_size;
                            }
                            _ => {
                                println!("What");
                            }
                        }
                        tex_coords.extend_from_slice(&face_tex);
                    }
                }

                let finalao = ao_convert(ao, num_vertices);
                let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords);
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, finalao);
                render_mesh.set_indices(Some(Indices::U32(indices)));

                cmd1.spawn(PbrBundle {
                    mesh: meshes.add(render_mesh.clone()),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        // base_color_texture: Some(texture_handle.0.clone()),
                        alpha_mode: AlphaMode::Mask((1.0)),
                        perceptual_roughness: 1.0,
                        ..default()
                    }),
                    transform: Transform::from_translation(Vec3::new(
                        (pos[0] * (CHUNK_SIZE) as i32) as f32,
                        (pos[1] * (CHUNK_SIZE) as i32) as f32,
                        (pos[2] * (CHUNK_SIZE) as i32) as f32,
                    )),
                    ..Default::default()
                });
                // This is stupid and awful so ill come back to semi transparent objects
                // cmd2.spawn(PbrBundle {
                //     mesh: meshes.add(render_mesh),
                //     material: materials.add(StandardMaterial {
                //         base_color: Color::WHITE,
                //         // base_color_texture: Some(texture_handle.0.clone()),
                //         alpha_mode: AlphaMode::Blend,
                //         perceptual_roughness: 1.0,
                //         ..default()
                //     }),
                //     transform: Transform::from_translation(Vec3::new(
                //         (pos[0] * (CHUNK_SIZE / 2) as i32) as f32,
                //         (pos[1] * (CHUNK_SIZE / 2) as i32) as f32,
                //         (pos[2] * (CHUNK_SIZE / 2) as i32) as f32,
                //     )),
                //     ..Default::default()
                // });
            }
        }
    }
}

// TODO: move this out just testing rn
fn ao_convert(ao: Vec<u8>, num_vertices: usize) -> Vec<[f32; 4]> {
    let mut res = Vec::with_capacity(num_vertices);
    for value in ao {
        match value {
            0 => res.extend_from_slice(&[[0.1, 0.1, 0.1, 1.0]]),
            1 => res.extend_from_slice(&[[0.3, 0.3, 0.3, 1.0]]),
            2 => res.extend_from_slice(&[[0.5, 0.5, 0.5, 1.0]]),
            3 => res.extend_from_slice(&[[0.75, 0.75, 0.75, 1.0]]),
            _ => res.extend_from_slice(&[[1., 1., 1., 1.0]]),
        }
    }
    return res;
}

pub fn lerp_new_location(
    mut commands: Commands,
    entity_buffer: ResMut<EntityBuffer>,
    lobby: ResMut<ClientLobby>,
    network_mapping: ResMut<NetworkMapping>,
    client: ResMut<RenetClient>,
    transform_query: Query<&Transform>,
) {
    for i in 0..entity_buffer.entities[0].entities.len() {
        if let Some(entity) = network_mapping
            .0
            .get(&entity_buffer.entities[0].entities[i])
        {
            let translation = Vec3::from(entity_buffer.entities[0].translations[i]);
            let transform = Transform {
                translation: translation,
                rotation: Quat::from_euler(
                    EulerRot::ZYX,
                    0.0,
                    entity_buffer.entities[0].yaw[i],
                    0.,
                ),
                ..Default::default()
            };
            if let Some(player_entity) = lobby.players.get(&client.client_id()) {
                if player_entity.client_entity != *entity {
                    if let Ok(old_transform) = transform_query.get(*entity) {
                        commands
                            .get_entity(*entity)
                            .unwrap()
                            .insert(old_transform.ease_to(
                                transform,
                                EaseMethod::Linear,
                                EasingType::Once {
                                    duration: Duration::from_millis(150),
                                },
                            ));
                    }
                } else {
                }
            } else {
                //Different entity rather then player.
            }
        }
    }
}

pub fn client_send_naive_position(
    mut transform_query: Query<&mut Transform, With<ControlledPlayer>>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(transform) = transform_query.get_single_mut() {
        let player_pos = PlayerPos {
            translation: transform.translation.into(),
            yaw: transform.rotation.z,
            pitch: transform.rotation.y,
        };
        let input_message = bincode::serialize(&player_pos).unwrap();

        client.send_message(ClientChannel::Position, input_message);
    }
}
