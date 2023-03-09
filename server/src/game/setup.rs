use crate::networking::syncing::NetworkingPlugin;
use bevy::prelude::*;
use bevy_quinnet::server::*;
use common::{
    game::{
        bundles::PlayerBundleBuilder,
        scripting::{block::load::load_all_blocks, entity::load::load_all_entities},
        storage::{convert_block, convert_entity, BlockType, EntityType},
    },
    networking::components::NetworkIP,
};

pub fn setup(mut commands: Commands, _chunk_manager: ChunkManager) {
    commands.spawn(LoadPoint(IVec3::new(0, 0, 0)));
}

use std::collections::HashMap;

use super::world::chunk::{ChunkGenerationPlugin, ChunkManager, LoadPoint};

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

pub fn new_server(ip_res: Res<NetworkIP>, mut server: ResMut<Server>) {
    server
        .start_endpoint(
            ServerConfigurationData::new(ip_res.0.clone(), 25565, "0.0.0.0".to_string()),
            certificate::CertificateRetrievalMode::GenerateSelfSigned,
        )
        .unwrap();
    server
        .endpoint_mut()
        .set_default_channel(bevy_quinnet::shared::channel::ChannelId::UnorderedReliable);
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
            .add_plugin(QuinnetServerPlugin::default())
            .add_plugin(NetworkingPlugin)
            .insert_resource(LoadableTypes::default())
            .add_startup_system(setup_loadables)
            .add_startup_system(new_server)
            .add_startup_system(setup_builders)
            .add_startup_system(setup);
    }
}
