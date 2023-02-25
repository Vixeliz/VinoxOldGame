use std::{io::Cursor, time::Duration};

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_easings::{Ease, EaseMethod, EasingType};

use bevy_rapier3d::prelude::{Collider, ComputedColliderShape};
use bevy_renet::renet::RenetClient;
use common::{
    game::{
        bundles::PlayerBundleBuilder,
        world::chunk::{Chunk, CHUNK_SIZE},
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
            input::CameraController,
            networking::components::ControlledPlayer,
            rendering::meshing::{build_mesh, MeshChunkEvent},
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
    mut chunk_mesh_event: EventWriter<MeshChunkEvent>,
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
                chunk_mesh_event.send(MeshChunkEvent {
                    raw_chunk: chunk_data,
                    pos: pos.into(),
                });
            }
        }
    }

    while let Some(message) = client.receive_message(ServerChannel::LevelDataLarge) {
        let mut temp_output = Cursor::new(Vec::new());
        copy_decode(&message[..], &mut temp_output).unwrap();
        let level_data: LevelData = bincode::deserialize(temp_output.get_ref()).unwrap();
        match level_data {
            LevelData::ChunkCreate { chunk_data, pos } => {
                println!("Recieved chunk {pos:?}");
                chunk_mesh_event.send(MeshChunkEvent {
                    raw_chunk: chunk_data,
                    pos: pos.into(),
                });
            }
        }
    }
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
