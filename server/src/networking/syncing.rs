use bevy_quinnet::server::{ConnectionEvent, Server};
use iyes_loopless::prelude::*;
use std::io::Cursor;

use bevy::prelude::*;
use common::{
    game::{
        bundles::PlayerBundleBuilder,
        world::chunk::{world_to_chunk, ChunkComp, CurrentChunks},
    },
    networking::components::{ClientMessage, NetworkedEntities, Player, ServerMessage},
};
use rustc_data_structures::stable_set::FxHashSet;
use zstd::stream::copy_encode;

use crate::game::world::{
    chunk::{ChunkManager, LoadPoint},
    storage::{insert_chunk, WorldDatabase},
};

use super::components::ServerLobby;

#[derive(Component, Clone)]
pub struct SentChunks {
    pub chunks: FxHashSet<IVec3>,
}

pub fn connections(
    mut server: ResMut<Server>,
    lobby: Res<ServerLobby>,
    mut connection_events: EventReader<ConnectionEvent>,
) {
    for client in connection_events.iter() {
        // Refuse connection once we already have two players
        if lobby.players.len() >= 8 {
            server.endpoint_mut().disconnect_client(client.id).unwrap();
        } else {
            server
                .endpoint_mut()
                .try_send_message(client.id, ServerMessage::ClientId { id: client.id });
        }
    }
}

// So i dont forget this is actually fine this is just receiving we are just sending out response packets which dont need to be limited since they only happen once per receive
#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server: ResMut<Server>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut players: Query<(Entity, &Player, &Transform, &mut SentChunks)>,
    player_builder: Res<PlayerBundleBuilder>,
    mut chunks: Query<&mut ChunkComp>,
    current_chunks: Res<CurrentChunks>,
    database: Res<WorldDatabase>,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some(message) = endpoint.try_receive_message_from::<ClientMessage>(client_id) {
            match message {
                ClientMessage::Join { id, user_name: _ } => {
                    println!("Player {id} connected.");

                    // Initialize other players for this new client
                    for (entity, player, transform, _sent_chunks) in players.iter_mut() {
                        endpoint.try_send_message(
                            id,
                            ServerMessage::PlayerCreate {
                                id: player.id,
                                entity,
                                translation: transform.translation,
                                rotation: Vec4::from(transform.rotation),
                            },
                        );
                    }

                    // Spawn new player
                    let transform = Transform::from_xyz(0.0, 130.0, 0.0);
                    // let player_entity = commands.spawn((transform, Player { id: *id })).id();
                    let player_entity = commands
                        .spawn(player_builder.build(transform.translation, id, false))
                        .insert(SentChunks {
                            chunks: FxHashSet::default(),
                        })
                        .insert(LoadPoint(world_to_chunk(transform.translation)))
                        .id();
                    lobby.players.insert(id, player_entity);

                    endpoint.try_broadcast_message(&ServerMessage::PlayerCreate {
                        id,
                        entity: player_entity,
                        translation: transform.translation,
                        rotation: Vec4::from(transform.rotation),
                    });
                }
                ClientMessage::Leave { id } => {
                    println!("Player {id} disconnected.");
                    if let Some(player_entity) = lobby.players.remove(&id) {
                        commands.entity(player_entity).despawn();
                    }

                    endpoint.try_broadcast_message(&ServerMessage::PlayerRemove { id });
                }
                ClientMessage::Position {
                    player_pos,
                    player_rot,
                } => {
                    if let Some(player_entity) = lobby.players.get(&client_id) {
                        commands.entity(*player_entity).insert(
                            Transform::from_translation(player_pos)
                                .with_rotation(Quat::from_vec4(player_rot)),
                        );
                    }
                }

                ClientMessage::SentBlock {
                    chunk_pos,
                    voxel_pos,
                    block_type,
                } => {
                    if let Some(chunk_entity) = current_chunks.get_entity(chunk_pos) {
                        if let Ok(mut chunk) = chunks.get_mut(chunk_entity) {
                            chunk.chunk_data.add_block_state(&block_type);
                            chunk.chunk_data.set_block(
                                UVec3::new(
                                    voxel_pos[0] as u32,
                                    voxel_pos[1] as u32,
                                    voxel_pos[2] as u32,
                                ),
                                block_type.clone(),
                            );
                            let data = database.connection.lock().unwrap();
                            insert_chunk(chunk.pos.0, &chunk.chunk_data, &data);
                            endpoint.try_broadcast_message(ServerMessage::SentBlock {
                                chunk_pos,
                                voxel_pos,
                                block_type,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::type_complexity)]
//This would eventually take in any networkedentity for now just player
pub fn server_network_sync(mut server: ResMut<Server>, query: Query<(Entity, &Transform)>) {
    let mut networked_entities = NetworkedEntities::default();
    for (entity, transform) in query.iter() {
        networked_entities.entities.push(entity);
        networked_entities.translations.push(transform.translation);
        networked_entities
            .rotations
            .push(Vec4::from(transform.rotation));
    }
    server.endpoint_mut().try_broadcast_message_on(
        bevy_quinnet::shared::channel::ChannelId::Unreliable,
        ServerMessage::NetworkedEntities { networked_entities },
    );
}

pub fn send_chunks(
    mut commands: Commands,
    mut server: ResMut<Server>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&Transform, &mut SentChunks), With<Player>>,
    mut chunk_manager: ChunkManager,
) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        if let Some(player_entity) = lobby.players.get(&client_id) {
            if let Ok((player_transform, mut sent_chunks)) = players.get_mut(*player_entity) {
                let chunk_pos = world_to_chunk(player_transform.translation);
                let load_point = LoadPoint(chunk_pos);
                commands.entity(*player_entity).insert(load_point.clone());
                for chunk in chunk_manager.get_chunks_around_chunk(chunk_pos, &sent_chunks) {
                    let raw_chunk = chunk.chunk_data.clone();
                    if let Ok(raw_chunk_bin) = bincode::serialize(&raw_chunk) {
                        let mut final_chunk = Cursor::new(raw_chunk_bin);
                        let mut output = Cursor::new(Vec::new());
                        copy_encode(&mut final_chunk, &mut output, 0).unwrap();
                        server.endpoint_mut().try_send_message(
                            client_id,
                            ServerMessage::LevelData {
                                chunk_data: output.get_ref().clone(),
                                pos: chunk.pos.0,
                            },
                        );
                        sent_chunks.chunks.insert(chunk.pos.0);
                    }
                }
            }
        }
    }
}

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(server_update_system)
            .add_fixed_timestep_system("network_update", 0, server_network_sync)
            .add_fixed_timestep_system("network_update", 0, connections)
            .add_fixed_timestep_system("network_update", 0, send_chunks)
            .insert_resource(ServerLobby::default());
    }
}
