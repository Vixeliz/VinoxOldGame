use std::{io::Cursor, time::Duration};

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_easings::{Ease, EaseMethod, EasingType};

use bevy_rapier3d::prelude::{Collider, ComputedColliderShape};
use bevy_renet::renet::RenetClient;
use block_mesh::ndshape::ConstShape;
use block_mesh::{visible_block_faces, UnitQuadBuffer, RIGHT_HANDED_Y_UP_CONFIG};
use common::{
    game::{
        bundles::PlayerBundleBuilder,
        world::chunk::{Chunk, ChunkShape, CHUNK_SIZE},
    },
    networking::components::{
        ClientChannel, EntityBuffer, LevelData, NetworkedEntities, PlayerPos, ServerChannel,
        ServerMessages,
    },
};
use zstd::stream::copy_decode;

use crate::{
    components::Game,
    states::{
        game::{
            input::CameraController, networking::components::ControlledPlayer,
            world::chunk::RenderedChunk,
        },
        loading::LoadableAssets,
    },
};

use super::components::{ClientLobby, NetworkMapping, PlayerInfo};

//TODO: Refactor this is a lot in one function
pub fn client_sync_players(
    mut cmd1: Commands,
    mut cmd2: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut entity_buffer: ResMut<EntityBuffer>,
    _asset_server: Res<AssetServer>,
    player_builder: Res<PlayerBundleBuilder>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut loadable_assets: ResMut<LoadableAssets>,
    texture_atlas: Res<Assets<TextureAtlas>>,
) {
    let client_id = client.client_id();
    while let Some(message) = client.receive_message(ServerChannel::ServerMessages) {
        let server_message = bincode::deserialize(&message).unwrap();
        match server_message {
            ServerMessages::PlayerCreate {
                id,
                translation,
                entity,
                rotation,
            } => {
                let mut client_entity = cmd1.spawn_empty();
                if client_id == id {
                    println!("You connected.");
                    let camera_id = cmd2
                        .spawn((
                            Game,
                            Camera3dBundle {
                                transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                                ..default()
                            },
                            CameraController::default(),
                        ))
                        .id();
                    client_entity.push_children(&[camera_id]);
                    client_entity
                        .insert(player_builder.build(translation.into(), id, true))
                        .insert(ControlledPlayer);
                } else {
                    println!("Player {id} connected.");
                    client_entity.insert(player_builder.build(translation.into(), id, false));
                    client_entity.insert(
                        Transform::from_translation(translation.into())
                            .with_rotation(Quat::from_vec4(rotation.into())),
                    );
                }

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };
                lobby.players.insert(id, player_info);
                network_mapping.0.insert(entity, client_entity.id());
            }
            ServerMessages::PlayerRemove { id } => {
                println!("Player {id} disconnected.");
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

    while let Some(message) = client.receive_message(ServerChannel::LevelDataSmall) {
        let mut temp_output = Cursor::new(Vec::new());
        copy_decode(&message[..], &mut temp_output).unwrap();
        let level_data: LevelData = bincode::deserialize(temp_output.get_ref()).unwrap();
        match level_data {
            LevelData::ChunkCreate { chunk_data, pos } => {
                println!("Recieved chunk {pos:?}");
                let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;
                // Simple meshing works on web and makes texture atlases easier. However I may look into greedy meshing in future
                let mut buffer = UnitQuadBuffer::new();
                visible_block_faces(
                    &chunk_data.voxels,
                    &ChunkShape {},
                    [0; 3],
                    [CHUNK_SIZE as u32; 3],
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
                        let block_atlas = texture_atlas.get(&loadable_assets.block_atlas).unwrap();
                        let texture_index = block_atlas.get_texture_index(
                            &loadable_assets
                                .block_textures
                                .get(
                                    &chunk_data
                                        .get_state_for_index(voxel_type.value as usize)
                                        .unwrap(),
                                )
                                .unwrap()[0],
                        );
                        calculate_coords(
                            &mut face_tex,
                            texture_index.unwrap(),
                            Vec2::new(16.0, 16.0),
                            block_atlas.size,
                        );
                        tex_coords.extend_from_slice(&face_tex);
                    }
                }

                let finalao = ao_convert(ao, num_vertices);
                let mut render_mesh = Mesh::new(PrimitiveTopology::TriangleList);

                render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, tex_coords);
                render_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, finalao);
                render_mesh.set_indices(Some(Indices::U32(indices.clone())));
                let collider = if positions.len() >= 4 {
                    Collider::from_bevy_mesh(&render_mesh.clone(), &ComputedColliderShape::TriMesh)
                        .unwrap_or_default()
                } else {
                    Collider::cuboid(0.0, 0.0, 0.0)
                };
                cmd1.spawn(RenderedChunk {
                    collider,
                    chunk: Chunk {
                        chunk_data,
                        pos: pos.into(),
                        dirty: true,
                        entities: Vec::new(),
                        saved_entities: Vec::new(),
                    },
                    mesh: PbrBundle {
                        mesh: meshes.add(render_mesh.clone()),
                        material: materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            base_color_texture: Some(
                                texture_atlas
                                    .get(&loadable_assets.block_atlas)
                                    .unwrap()
                                    .texture
                                    .clone(),
                            ),
                            alpha_mode: AlphaMode::Mask(1.0),
                            perceptual_roughness: 1.0,
                            ..default()
                        }),
                        transform: Transform::from_translation(Vec3::new(
                            (pos[0] * (CHUNK_SIZE - 2) as i32) as f32,
                            (pos[1] * (CHUNK_SIZE - 2) as i32) as f32,
                            (pos[2] * (CHUNK_SIZE - 2) as i32) as f32,
                        )),
                        ..Default::default()
                    },
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
            0 => res.extend_from_slice(&[[0.3, 0.3, 0.3, 1.0]]),
            1 => res.extend_from_slice(&[[0.5, 0.5, 0.5, 1.0]]),
            2 => res.extend_from_slice(&[[0.75, 0.75, 0.75, 1.0]]),
            _ => res.extend_from_slice(&[[1.0, 1.0, 1.0, 1.0]]),
        }
    }
    res
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
            let rotation = Quat::from_vec4(entity_buffer.entities[0].rotations[i].into());
            let transform = Transform {
                translation,
                ..Default::default()
            }
            .with_rotation(rotation);
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

pub fn calculate_coords(
    face_tex: &mut [[f32; 2]; 4],
    index: usize,
    tile_size: Vec2,
    tilesheet_size: Vec2,
) {
    let mut index = index as f32;
    // We need to start at 1.0 for calculations
    index += 1.0;
    let max_y = (tile_size.y) / tilesheet_size.y;
    face_tex[0][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[0][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[0][1] = 0.0;
    face_tex[1][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[1][1] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    face_tex[1][1] = 0.0;
    face_tex[2][0] = ((index - 1.0) * tile_size.x) / tilesheet_size.x;
    // face_tex[2][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[2][1] = max_y;
    face_tex[3][0] = (index * tile_size.x) / tilesheet_size.x;
    // face_tex[3][1] = (index * tile_size.x) / tilesheet_size.x;
    face_tex[3][1] = max_y;
}

pub fn client_send_naive_position(
    mut transform_query: Query<&mut Transform, With<ControlledPlayer>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<ControlledPlayer>)>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(transform) = transform_query.get_single_mut() {
        if let Ok(camera_transform) = camera_query.get_single_mut() {
            let player_pos = PlayerPos {
                translation: transform.translation.into(),
                rotation: camera_transform.rotation.into(),
            };
            let input_message = bincode::serialize(&player_pos).unwrap();

            client.send_message(ClientChannel::Position, input_message);
        }
    }
}
