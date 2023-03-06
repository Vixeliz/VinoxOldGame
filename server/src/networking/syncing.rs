use std::{collections::HashSet, io::Cursor, mem::size_of_val};

use bevy::prelude::*;
use bevy_egui::egui::epaint::ahash::HashSetExt;
use bevy_renet::renet::{RenetServer, ServerEvent};
use common::{
    game::{bundles::PlayerBundleBuilder, world::chunk::ChunkComp},
    networking::components::{
        self, ClientChannel, LevelData, NetworkedEntities, Player, PlayerPos, ServerChannel,
        ServerMessages, RELIABLE_CHANNEL_MAX_LENGTH,
    },
};
use rand::seq::{IteratorRandom, SliceRandom};
use rustc_data_structures::stable_set::FxHashSet;
use zstd::stream::copy_encode;

use crate::game::world::{
    chunk::{world_to_chunk, ChunkManager, CurrentChunks, LoadPoint, ViewDistance},
    storage::{create_database, insert_chunk, WorldDatabase},
};

use super::components::ServerLobby;

#[derive(Component, Clone)]
pub struct SentChunks {
    pub chunks: FxHashSet<IVec3>,
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
                println!("Player {id} connected.");

                // Initialize other players for this new client
                for (entity, player, transform, _sent_chunks) in players.iter_mut() {
                    let translation: [f32; 3] = transform.translation.into();
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
                let transform = Transform::from_xyz(0.0, 200.0, -10.0);
                // let player_entity = commands.spawn((transform, Player { id: *id })).id();
                let player_entity = commands
                    .spawn(player_builder.build(transform.translation, *id, false))
                    .insert(SentChunks {
                        chunks: FxHashSet::new(),
                    })
                    .insert(LoadPoint(world_to_chunk(transform.translation)))
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
            }
            ServerEvent::ClientDisconnected(id) => {
                println!("Player {id} disconnected.");
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
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    lobby: ResMut<ServerLobby>,
    mut players: Query<(&Transform, &mut SentChunks), With<Player>>,
    mut chunk_manager: ChunkManager,
) {
    for client_id in server.clients_id().into_iter() {
        if let Some(player_entity) = lobby.players.get(&client_id) {
            if let Ok((player_transform, mut sent_chunks)) = players.get_mut(*player_entity) {
                let chunk_pos = world_to_chunk(player_transform.translation);
                let load_point = LoadPoint(chunk_pos);
                commands.entity(*player_entity).insert(load_point.clone());
                for chunk in chunk_manager
                    .get_chunks_around_chunk(chunk_pos, &sent_chunks)
                    .choose_multiple(&mut rand::thread_rng(), 10)
                {
                    let raw_chunk = chunk.chunk_data.clone();
                    if let Ok(raw_chunk_bin) = bincode::serialize(&LevelData::ChunkCreate {
                        chunk_data: raw_chunk,
                        pos: chunk.pos.0.into(),
                    }) {
                        let mut final_chunk = Cursor::new(raw_chunk_bin);
                        let mut output = Cursor::new(Vec::new());
                        copy_encode(&mut final_chunk, &mut output, 0).unwrap();
                        if size_of_val(output.get_ref().as_slice())
                            <= RELIABLE_CHANNEL_MAX_LENGTH as usize
                        {
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
                        sent_chunks.chunks.insert(chunk.pos.0);
                    }
                }
            }
        }
    }
}

pub fn block_sync(
    mut server: ResMut<RenetServer>,
    mut chunks: Query<&mut ChunkComp>,
    current_chunks: Res<CurrentChunks>,
    database: Res<WorldDatabase>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Commands) {
            if let Ok(sent_block) = bincode::deserialize::<components::Commands>(&message) {
                match sent_block {
                    components::Commands::SentBlock {
                        chunk_pos,
                        voxel_pos,
                        block_type,
                    } => {
                        if let Some(chunk_entity) = current_chunks.get_entity(chunk_pos.into()) {
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
                                if let Ok(send_message) =
                                    bincode::serialize(&ServerMessages::SentBlock {
                                        chunk_pos,
                                        voxel_pos,
                                        block_type,
                                    })
                                {
                                    server.broadcast_message(
                                        ServerChannel::ServerMessages,
                                        send_message,
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
