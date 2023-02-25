use crate::networking::{components::ServerLobby, *};
use bevy::prelude::*;
use common::{
    game::{
        bundles::PlayerBundleBuilder,
        scripting::{block::load::load_all_blocks, entity::load::load_all_entities},
        storage::{convert_block, convert_entity, BlockType, EntityType},
    },
    networking::components::{server_connection_config, NetworkIP, PROTOCOL_ID},
};
use iyes_loopless::prelude::AppLooplessFixedTimestepExt;

pub fn setup(_commands: Commands, mut chunk_manager: ChunkManager) {
    chunk_manager.add_point(IVec3 { x: 0, y: 0, z: 0 }, 0);
}

use std::{collections::HashMap, net::UdpSocket, time::SystemTime};

use bevy::app::AppExit;
use bevy_renet::renet::{RenetError, RenetServer, ServerAuthentication, ServerConfig};

use super::world::chunk::{ChunkGenerationPlugin, ChunkManager};

extern crate common;

#[derive(Resource, Default)]
pub struct LoadableTypes {
    pub entities: HashMap<String, EntityType>,
    pub blocks: HashMap<String, BlockType>,
}

pub fn setup_loadables(mut loadable_types: ResMut<LoadableTypes>) {
    loadable_types.blocks = convert_block(load_all_blocks());
    loadable_types.entities = convert_entity(load_all_entities());
}

pub fn new_renet_server(mut commands: Commands, ip_res: Res<NetworkIP>) {
    let port: String = ":25565".to_owned();
    let server_addr = format!("{}{}", ip_res.0, port).parse().unwrap();
    // let server_addr = "127.0.0.1:25565".parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:25565").unwrap();
    let connection_config = server_connection_config();
    let server_config =
        ServerConfig::new(16, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure);
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    commands.insert_resource(
        RenetServer::new(current_time, server_config, connection_config, socket).unwrap(),
    );
}

fn panic_on_error_system(mut renet_error: EventReader<RenetError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}

fn disconnect_clients_on_exit(exit: EventReader<AppExit>, mut server: ResMut<RenetServer>) {
    if !exit.is_empty() {
        server.disconnect_clients();
    }
}

pub fn setup_builders(mut commands: Commands) {
    commands.insert_resource(PlayerBundleBuilder {
        ..Default::default()
    });
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ChunkGenerationPlugin)
            .insert_resource(LoadableTypes::default())
            .add_startup_system(setup_loadables)
            .add_startup_system(new_renet_server)
            .add_startup_system(setup_builders)
            .add_system(panic_on_error_system)
            .add_system(syncing::server_update_system)
            .add_fixed_timestep_system("network_update", 0, syncing::server_network_sync)
            .add_fixed_timestep_system("network_update", 0, syncing::send_chunks)
            .add_startup_system(setup)
            .insert_resource(ServerLobby::default());
    }
}
