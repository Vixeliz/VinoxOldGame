use std::{io::Cursor, time::Duration};

use belly::prelude::*;
use bevy::prelude::*;

use bevy_tweening::{
    lens::{TransformPositionLens, TransformRotationLens},
    *,
};

use bevy_renet::renet::RenetClient;
use common::{
    game::bundles::PlayerBundleBuilder,
    networking::components::{
        ClientChannel, EntityBuffer, LevelData, NetworkedEntities, PlayerPos, ServerChannel,
        ServerMessages,
    },
};
use iyes_loopless::state::NextState;
use zstd::stream::copy_decode;

use crate::{
    components::GameState,
    states::game::{
        networking::components::ControlledPlayer,
        world::chunk::{CreateChunkEvent, PlayerChunk, SetBlockEvent},
    },
};

use super::components::{ClientLobby, NetworkMapping, PlayerInfo};

#[derive(Component)]
pub struct JustSpawned {
    timer: Timer,
}

//TODO: Refactor this is a lot in one function
#[allow(clippy::clone_on_copy)]
#[allow(clippy::too_many_arguments)]
pub fn client_sync_players(
    mut cmd1: Commands,
    mut cmd2: Commands,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut entity_buffer: ResMut<EntityBuffer>,
    player_builder: Res<PlayerBundleBuilder>,
    mut chunk_event: EventWriter<CreateChunkEvent>,
    mut block_event: EventWriter<SetBlockEvent>,
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
                    // let camera_id = cmd2
                    //     .spawn((
                    //         Game,
                    //         Camera3dBundle {
                    //             camera: Camera {
                    //                 hdr: true,
                    //                 ..default()
                    //             },
                    //             transform: Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                    //             ..default()
                    //         },
                    //         FPSCamera::default(),
                    //         BloomSettings::default(),
                    //         AtmosphereCamera::default(),
                    //     ))
                    //     .id();
                    // client_entity.push_children(&[camera_id]);
                    client_entity
                        .insert(player_builder.build(translation.into(), id, true))
                        .insert(ControlledPlayer)
                        // .insert(KinematicCharacterController {
                        //     // snap_to_ground: Some(
                        //     //     bevy_rapier3d::prelude::CharacterLength::Relative(0.3),
                        //     // ),
                        //     autostep: Some(CharacterAutostep {
                        //         max_height: CharacterLength::Absolute(1.1),
                        //         min_width: CharacterLength::Absolute(0.1),
                        //         include_dynamic_bodies: false,
                        //     }),
                        //     offset: CharacterLength::Absolute(0.04),
                        //     ..default()
                        // })
                        .insert(JustSpawned {
                            timer: Timer::new(Duration::from_secs(10), TimerMode::Once),
                        });
                    cmd2.add(eml! {
                        <body s:padding="50px" s:margin-left="5px" s:justify-content="flex-start" s:align-items="flex-start">
                            "ChunkPos: "{from!(PlayerChunk:chunk_pos | fmt.c("{c}"))}
                        </body>
                    });
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
            ServerMessages::SentBlock {
                chunk_pos,
                voxel_pos,
                block_type,
            } => block_event.send(SetBlockEvent {
                chunk_pos: chunk_pos.into(),
                voxel_pos: UVec3::new(
                    voxel_pos[0] as u32,
                    voxel_pos[1] as u32,
                    voxel_pos[2] as u32,
                ),
                block_type,
            }),
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
                // println!("Recieved chunk {pos:?}");
                chunk_event.send(CreateChunkEvent {
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
                // println!("Recieved chunk {pos:?}");
                chunk_event.send(CreateChunkEvent {
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
                        let tween = Tween::new(
                            EaseFunction::QuadraticInOut,
                            Duration::from_millis(75),
                            TransformPositionLens {
                                start: old_transform.translation,
                                end: transform.translation,
                            },
                        )
                        .with_repeat_count(RepeatCount::Finite(1));
                        let tween_rot = Tween::new(
                            EaseFunction::QuadraticInOut,
                            Duration::from_millis(75),
                            TransformRotationLens {
                                start: old_transform.rotation,
                                end: transform.rotation,
                            },
                        )
                        .with_repeat_count(RepeatCount::Finite(1));
                        let track = Tracks::new([tween, tween_rot]);
                        commands
                            .get_entity(*entity)
                            .unwrap()
                            .insert(Animator::new(track));
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
pub fn client_disconect(mut commands: Commands, client: Res<RenetClient>) {
    if client.disconnected().is_some() {
        println!("{}", client.disconnected().unwrap());
        commands.insert_resource(NextState(GameState::Menu));
    }
}

// TODO: Have a more elegant way to wait on loading section or by actually waiting till all the intial chunks are loaded
pub fn wait_for_chunks(
    mut just_spawned_query: Query<(&mut JustSpawned, Entity, &mut Transform)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if let Ok((mut just_spawned, entity, mut player_transform)) =
        just_spawned_query.get_single_mut()
    {
        just_spawned.timer.tick(time.delta());
        if just_spawned.timer.finished() {
            commands.entity(entity).remove::<JustSpawned>();
        } else {
            player_transform.translation = Vec3::new(0.0, 100.0, 0.0);
        }
    }
}
