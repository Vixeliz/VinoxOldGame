use bevy::prelude::*;

#[derive(Resource)]
pub struct NetworkIP(pub String);

use std::time::Duration;

use bevy_renet::renet::{
    ChannelConfig, ChunkChannelConfig, ReliableChannelConfig, RenetConnectionConfig,
    UnreliableChannelConfig,
};
use serde::{Deserialize, Serialize};

use crate::game::world::chunk::RawChunk;

pub const PROTOCOL_ID: u64 = 7;
pub const RELIABLE_CHANNEL_MAX_LENGTH: u64 = 10240;

#[derive(Component)]
pub struct NetworkedEntity;

#[derive(Debug, Component, Default)]
pub struct Player {
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PlayerPos {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
}

// Networking related
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NetworkedEntities {
    pub entities: Vec<Entity>,
    pub translations: Vec<[f32; 3]>,
    pub rotations: Vec<[f32; 4]>,
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
    LevelDataSmall,
    LevelDataLarge,
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum LevelData {
    ChunkCreate { chunk_data: RawChunk, pos: [i32; 3] },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum Commands {
    Interact {
        entity: Entity,
        attack: bool,
    },

    SentBlock {
        chunk_pos: [i32; 3],
        voxel_pos: [u8; 3],
        block_type: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerMessages {
    PlayerCreate {
        entity: Entity,
        id: u64,
        translation: [f32; 3],
        rotation: [f32; 4],
    },
    PlayerRemove {
        id: u64,
    },
    SentBlock {
        chunk_pos: [i32; 3],
        voxel_pos: [u8; 3],
        block_type: String,
    },
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            UnreliableChannelConfig {
                channel_id: Self::Position.into(),
                sequenced: true,
                message_send_queue_size: 2048,
                message_receive_queue_size: 2048,
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::Commands.into(),
                message_resend_time: Duration::ZERO,
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
                message_send_queue_size: 2048,
                message_receive_queue_size: 2048,
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
                channel_id: Self::LevelDataLarge.into(),
                message_send_queue_size: 700,
                ..Default::default()
            }
            .into(),
            ReliableChannelConfig {
                channel_id: Self::LevelDataSmall.into(),
                message_resend_time: Duration::ZERO,
                max_message_size: RELIABLE_CHANNEL_MAX_LENGTH,
                packet_budget: RELIABLE_CHANNEL_MAX_LENGTH * 2,
                message_send_queue_size: (RELIABLE_CHANNEL_MAX_LENGTH * 3) as usize,
                message_receive_queue_size: (RELIABLE_CHANNEL_MAX_LENGTH * 3) as usize,
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
            ServerChannel::LevelDataLarge => 2,
            ServerChannel::LevelDataSmall => 3,
        }
    }
}

pub fn client_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ClientChannel::channels_config(),
        receive_channels_config: ServerChannel::channels_config(),
        max_packet_size: RELIABLE_CHANNEL_MAX_LENGTH * 4,
        ..Default::default()
    }
}

pub fn server_connection_config() -> RenetConnectionConfig {
    RenetConnectionConfig {
        send_channels_config: ServerChannel::channels_config(),
        receive_channels_config: ClientChannel::channels_config(),
        max_packet_size: RELIABLE_CHANNEL_MAX_LENGTH * 4,
        ..Default::default()
    }
}
