use std::time::Duration;

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_easings::{Ease, EaseMethod, EasingType};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use common::{
    game::bundles::{ColliderBundle, PlayerBundleBuilder},
    networking::components::{
        ClientChannel, EntityBuffer, NetworkedEntities, Player, PlayerPos, ServerChannel,
        ServerMessages,
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
                pitch
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
