use bevy::prelude::*;

#[derive(Resource)]
pub struct NetworkIP(pub String);

use std::time::Duration;

use bevy::prelude::*;
use bevy_renet::renet::{
    ChannelConfig, ChunkChannelConfig, ReliableChannelConfig, RenetConnectionConfig,
    UnreliableChannelConfig,
};
use serde::{Deserialize, Serialize};

use crate::game::world::chunk::RawChunk;

pub const PROTOCOL_ID: u64 = 7;

#[derive(Component)]
pub struct NetworkedEntity;

#[derive(Debug, Component, Default)]
pub struct Player {
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PlayerPos {
    pub translation: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

// Networking related
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
    pub yaw: Vec<f32>,
    pub pitch: Vec<f32>,
}

#[derive(Default, Resource)]
pub struct EntityBuffer {
    pub entities: [NetworkedEntities; 30],
}

pub enum ClientChannel {
    Position,
    Commands,
}

pub enum ServerChannel {
    ServerMessages,
    NetworkedEntities,
    LevelData,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum LevelData {
    ChunkCreate { chunk_data: RawChunk, pos: [i32; 3] },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum Commands {
    Interact { entity: Entity, attack: bool },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate {
        entity: Entity,
        id: u64,
        translation: [f32; 3],
        yaw: f32,
        pitch: f32,
    },
    PlayerRemove {
        id: u64,
    },
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            UnreliableChannelConfig {
                channel_id: Self::Position.into(),
                sequenced: true,
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::Commands.into(),
                message_resend_time: Duration::from_millis(16),
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Position => 0,
            ClientChannel::Commands => 1,
        }
    }
}

impl ServerChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            UnreliableChannelConfig {
                channel_id: Self::NetworkedEntities.into(),
                sequenced: true, // We don't care about old positions
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::ServerMessages.into(),
                message_resend_time: Duration::from_millis(100),
                ..Default::default()
            }
            .into(),
            ChunkChannelConfig {
                channel_id: Self::LevelData.into(),
                message_send_queue_size: 700,
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::NetworkedEntities => 0,
            ServerChannel::ServerMessages => 1,
            ServerChannel::LevelData => 2,
        }
    }
}

pub fn client_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ClientChannel::channels_config(),
        receive_channels_config: ServerChannel::channels_config(),
        ..Default::default()
    }
}

pub fn server_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ServerChannel::channels_config(),
        receive_channels_config: ClientChannel::channels_config(),
        ..Default::default()
    }
}
