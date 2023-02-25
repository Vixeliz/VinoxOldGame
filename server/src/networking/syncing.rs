use std::{
    collections::HashSet,
    io::{Cursor, Write},
    mem::size_of_val,
};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_renet::renet::{RenetServer, ServerEvent};
use common::{
    game::{bundles::PlayerBundleBuilder, world::chunk::ChunkComp},
    networking::components::{
        ClientChannel, LevelData, NetworkedEntities, Player, PlayerPos, ServerChannel,
        ServerMessages,
    },
};
use zstd::stream::{
    copy_decode, copy_encode,
    write::{Decoder, Encoder},
};

use crate::game::world::{
    chunk::{ChunkManager, CurrentChunks},
    generation::generate_chunk,
};

use super::components::ServerLobby;

#[derive(Component)]
pub struct SentChunks {
    chunks: HashSet<IVec3>,
}

// So i dont forget this is actually fine this is just receiving we are just sending out response packets which dont need to be limited since they only happen once per receive
#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    mut players: Query<(Entity, &Player, &Transform, &mut SentChunks)>,
    player_builder: Res<PlayerBundleBuilder>,
    mut chunk_manager: ChunkManager,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _) => {
                println!("Player {} connected.", id);

                // Initialize other players for this new client
                for (entity, player, transform, sent_chunks) in players.iter_mut() {
                    let translation: [f32; 3] = Vec3::from(transform.translation).into();
                    let rotation: [f32; 4] = Vec4::from(transform.rotation).into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation,
                        rotation,
                    })
                    .unwrap();
                    server.send_message(*id, ServerChannel::ServerMessages, message);
                }

                // Spawn new player
                let transform = Transform::from_xyz(0.0, 16.0, -10.0);
                // let player_entity = commands.spawn((transform, Player { id: *id })).id();
                let player_entity = commands
                    .spawn(player_builder.build(transform.translation, *id, false))
                    .insert(SentChunks {
                        chunks: HashSet::new(),
                    })
                    .id();
                lobby.players.insert(*id, player_entity);

                let translation: [f32; 3] = transform.translation.into();
                let rotation: [f32; 4] = transform.rotation.into();
                let message = bincode::serialize(&ServerMessages::PlayerCreate {
                    id: *id,
                    entity: player_entity,
                    translation,
                    rotation,
                })
                .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
                let chunk_pos = chunk_manager.world_to_chunk(transform.translation);
                for chunk in chunk_manager.get_chunks_around_chunk(chunk_pos).iter() {
                    if let Ok((_, _, _, mut sent_chunks)) = players.get_mut(player_entity) {
                        sent_chunks.chunks.insert(chunk.pos);
                        let raw_chunk = chunk.chunk_data.clone();
                        if let Ok(raw_chunk_bin) = bincode::serialize(&LevelData::ChunkCreate {
                            chunk_data: raw_chunk.clone(),
                            pos: chunk.pos.into(),
                        }) {
                            let mut final_chunk = Cursor::new(raw_chunk_bin);
                            let mut output = Cursor::new(Vec::new());
                            copy_encode(&mut final_chunk, &mut output, 0).unwrap();
                            if size_of_val(output.get_ref().as_slice()) <= 10000 {
                                server.send_message(
                                    *id,
                                    ServerChannel::LevelDataSmall,
                                    output.get_ref().clone(),
                                );
                            } else {
                                server.send_message(
                                    *id,
                                    ServerChannel::LevelDataLarge,
                                    output.get_ref().clone(),
                                );
                            }
                        }
                    }
                }
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Player {} disconnected.", id);
                if let Some(player_entity) = lobby.players.remove(id) {
                    commands.entity(player_entity).despawn();
                }

                let message =
                    bincode::serialize(&ServerMessages::PlayerRemove { id: *id }).unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
            }
        }
    }

    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Position) {
            let transform: PlayerPos = bincode::deserialize(&message).unwrap();
            if let Some(player_entity) = lobby.players.get(&client_id) {
                commands.entity(*player_entity).insert(
                    Transform::from_translation(transform.translation.into())
                        .with_rotation(Quat::from_vec4(transform.rotation.into())),
                );
            }
        }
    }
}

#[allow(clippy::type_complexity)]
//This would eventually take in any networkedentity for now just player
pub fn server_network_sync(mut server: ResMut<RenetServer>, query: Query<(Entity, &Transform)>) {
    let mut networked_entities = NetworkedEntities::default();
    for (entity, transform) in query.iter() {
        networked_entities.entities.push(entity);
        networked_entities
            .translations
            .push(transform.translation.into());
        networked_entities.rotations.push(transform.rotation.into());
    }

    let sync_message = bincode::serialize(&networked_entities).unwrap();
    server.broadcast_message(ServerChannel::NetworkedEntities, sync_message);
}

pub fn send_chunks(
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<ServerLobby>,
    mut players: Query<(&Transform, &mut SentChunks), With<Player>>,
    mut chunk_manager: ChunkManager,
) {
    for client_id in server.clients_id().into_iter() {
        if let Some(player_entity) = lobby.players.get(&client_id) {
            if let Ok((player_transform, mut sent_chunks)) = players.get_mut(*player_entity) {
                let chunk_pos = chunk_manager.world_to_chunk(player_transform.translation);
                chunk_manager.add_point(chunk_pos, client_id);
                for chunk in chunk_manager.get_chunks_around_chunk(chunk_pos).iter() {
                    if !sent_chunks.chunks.contains(&chunk.pos) {
                        let raw_chunk = chunk.chunk_data.clone();
                        if let Ok(raw_chunk_bin) = bincode::serialize(&LevelData::ChunkCreate {
                            chunk_data: raw_chunk.clone(),
                            pos: chunk.pos.into(),
                        }) {
                            let mut final_chunk = Cursor::new(raw_chunk_bin);
                            let mut output = Cursor::new(Vec::new());
                            copy_encode(&mut final_chunk, &mut output, 0).unwrap();
                            if size_of_val(output.get_ref().as_slice()) <= 10000 {
                                server.send_message(
                                    client_id,
                                    ServerChannel::LevelDataSmall,
                                    output.get_ref().clone(),
                                );
                            } else {
                                server.send_message(
                                    client_id,
                                    ServerChannel::LevelDataLarge,
                                    output.get_ref().clone(),
                                );
                            }
                            sent_chunks.chunks.insert(chunk.pos);
                        }
                    }
                }
            }
        }
    }
}
