use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_renet::renet::{RenetServer, ServerEvent};
use common::{
    game::{bundles::PlayerBundleBuilder, world::chunk::Chunk},
    networking::components::{
        ClientChannel, LevelData, NetworkedEntities, Player, PlayerPos, ServerChannel,
        ServerMessages,
    },
};

use crate::game::world::{
    chunk::{ChunkManager, CurrentChunks},
    generation::generate_chunk,
};

use super::components::ServerLobby;

// So i dont forget this is actually fine this is just receiving we are just sending out response packets which dont need to be limited since they only happen once per receive
#[allow(clippy::too_many_arguments)]
pub fn server_update_system(
    mut server_events: EventReader<ServerEvent>,
    mut commands: Commands,
    mut lobby: ResMut<ServerLobby>,
    mut server: ResMut<RenetServer>,
    players: Query<(Entity, &Player, &Transform)>,
    chunks: Query<&Chunk>,
    player_builder: Res<PlayerBundleBuilder>,
    mut chunk_manager: ChunkManager,
) {
    for event in server_events.iter() {
        match event {
            ServerEvent::ClientConnected(id, _) => {
                println!("Player {} connected.", id);

                // Initialize other players for this new client
                for (entity, player, transform) in players.iter() {
                    let translation: [f32; 3] = Vec3::from(transform.translation).into();
                    let message = bincode::serialize(&ServerMessages::PlayerCreate {
                        id: player.id,
                        entity,
                        translation,
                        yaw: 0.0,
                        pitch: 0.0,
                    })
                    .unwrap();
                    server.send_message(*id, ServerChannel::ServerMessages, message);
                }

                // Spawn new player
                let transform = Transform::from_xyz(0.0, 0.0, -10.0);
                // let player_entity = commands.spawn((transform, Player { id: *id })).id();
                let player_entity = commands
                    .spawn(player_builder.build(transform.translation, *id, false))
                    .id();
                lobby.players.insert(*id, player_entity);

                let translation: [f32; 3] = transform.translation.into();
                let message = bincode::serialize(&ServerMessages::PlayerCreate {
                    id: *id,
                    entity: player_entity,
                    translation,
                    yaw: 0.0,
                    pitch: 0.0,
                })
                .unwrap();
                server.broadcast_message(ServerChannel::ServerMessages, message);
                let chunk_pos = chunk_manager.world_to_chunk(transform.translation);
                chunk_manager.add_point(chunk_pos);
                for chunk in chunk_manager.get_chunks_around_chunk(chunk_pos).iter() {
                    let raw_chunk = chunk.chunk_data.clone();
                    let chunk_message = bincode::serialize(&LevelData::ChunkCreate {
                        chunk_data: raw_chunk,
                        pos: chunk.pos.into(),
                    })
                    .unwrap();
                    server.send_message(*id, ServerChannel::LevelData, chunk_message);
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
                    Transform::from_translation(transform.translation.into()).with_rotation(
                        Quat::from_euler(EulerRot::ZYX, 0.0, transform.yaw, transform.pitch),
                    ),
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
        networked_entities.yaw.push(0.0);
        networked_entities.pitch.push(0.0);
    }

    let sync_message = bincode::serialize(&networked_entities).unwrap();
    server.broadcast_message(ServerChannel::NetworkedEntities, sync_message);
}
