use bevy::{asset::LoadState, math::Vec3A, prelude::*, render::primitives::Aabb};
use bevy_rapier3d::prelude::*;
use common::{
    game::{
        bundles::{AssetsLoading, PlayerBundleBuilder},
        scripting::{block::load::load_all_blocks, entity::load::load_all_entities},
        storage::{convert_block, convert_entity, BlockType, EntityType},
    },
    networking::components::{client_connection_config, NetworkIP, PROTOCOL_ID},
};
use iyes_loopless::{prelude::AppLooplessStateExt, state::NextState};

use crate::{
    components::{GameState, Loading},
    systems::despawn_with,
};

use std::{
    collections::HashMap,
    net::UdpSocket,
    time::{Duration, SystemTime},
};

use bevy::{
    app::AppExit,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::exit_on_all_closed,
};
use bevy_renet::{
    renet::{ClientAuthentication, RenetClient, RenetError},
    RenetClientPlugin,
};
use iyes_loopless::prelude::*;
extern crate common;

pub fn new_client(mut commands: Commands, ip_res: Res<NetworkIP>) {
    let port: String = ":25565".to_owned();
    let server_addr = format!("{}{}", ip_res.0, port).parse().unwrap();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    let connection_config = client_connection_config();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = current_time.as_millis() as u64;
    let authentication = ClientAuthentication::Unsecure {
        protocol_id: PROTOCOL_ID,
        client_id: client_id,
        server_addr: server_addr,
        user_data: None,
    };
    commands.insert_resource(
        RenetClient::new(current_time, socket, connection_config, authentication).unwrap(),
    );
}

pub fn switch(
    mut commands: Commands,
    client: Res<RenetClient>,
    loading: Res<AssetsLoading>,
    asset_server: Res<AssetServer>,
) {
    match asset_server.get_group_load_state(loading.0.iter().map(|h| h.id)) {
        LoadState::Failed => {
            commands.insert_resource(NextState(GameState::Menu));
        }
        LoadState::Loaded => {
            if client.is_connected() {
                commands.insert_resource(NextState(GameState::Game));
            }
            // remove the resource to drop the tracking handles
            // commands.remove_resource::<AssetsLoading>();
            // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)
        }
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

fn panic_on_error_system(
    mut renet_error: EventReader<RenetError>,
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
) {
    for e in renet_error.iter() {
        commands.remove_resource::<RenetClient>();
        commands.insert_resource(NextState(GameState::Menu));
    }
}

// Move to game state
fn disconnect_on_exit(exit: EventReader<AppExit>, mut client: ResMut<RenetClient>) {
    if !exit.is_empty() && client.is_connected() {
        client.disconnect();
    }
}

pub fn setup_resources(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    mut loadable_types: ResMut<LoadableTypes>,
) {
    let player_handle = asset_server.load("player.gltf#Scene0");
    loading.0.push(player_handle.clone_untyped());
    commands.insert_resource(PlayerBundleBuilder {
        default_model: player_handle,
        model_aabb: Aabb {
            half_extents: Vec3A::new(1.0, 2.0, 0.5),
            ..default()
        },
    });
    loadable_types.blocks = convert_block(load_all_blocks());
    loadable_types.entities = convert_entity(load_all_entities());
}

#[derive(Resource, Default)]
pub struct LoadableTypes {
    pub entities: Vec<EntityType>,
    pub blocks: Vec<BlockType>,
}

#[derive(Resource, Default)]
pub struct LoadableAssets {
    pub block_models: HashMap<String, Handle<Scene>>,
    pub block_textures: HashMap<String, Handle<Image>>,
    pub entity_models: HashMap<String, Handle<Scene>>,
}

pub fn load_blocks(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    loadable_types: Res<LoadableTypes>,
    mut loadable_assets: ResMut<LoadableAssets>,
    mut has_ran: Local<bool>,
) {
    if *has_ran != true {
        if loadable_types.is_changed() {
            for block in &loadable_types.blocks {
                for texture_path_and_type in block.textures.iter() {
                    let mut path = "blocks/".to_string();
                    path.push_str(block.block_name.as_str());
                    path.push_str("/");
                    path.push_str(texture_path_and_type.1);
                    let texture_handle: Handle<Image> = asset_server.load(path.as_str());
                    loading.0.push(texture_handle.clone_untyped());
                    let mut block_identifier = block.namespace.to_owned();
                    block_identifier.push_str(&block.block_name.to_owned());
                    loadable_assets
                        .block_textures
                        .insert(block_identifier, texture_handle);
                }
                if let Some(mut model_path) = block.model.clone() {
                    let mut path = "blocks/".to_string();
                    path.push_str(block.block_name.as_str());
                    path.push_str("/");
                    path.push_str(model_path.as_str());
                    path.push_str("#Scene0");
                    let model_handle: Handle<Scene> = asset_server.load(model_path);
                    loading.0.push(model_handle.clone_untyped());
                    let mut block_identifier = block.namespace.to_owned();
                    block_identifier.push_str(&block.block_name.to_owned());
                    loadable_assets
                        .block_models
                        .insert(block_identifier, model_handle);
                }
            }
            *has_ran = true;
        }
    }
}

pub fn load_sounds() {}

pub fn load_entities(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    loadable_types: Res<LoadableTypes>,
) {
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .insert_resource(RapierConfiguration {
                gravity: Vec3::new(0.0, 0.0, 0.0),
                ..Default::default()
            })
            .add_plugin(RapierDebugRenderPlugin::default())
            .insert_resource(AssetsLoading::default())
            .insert_resource(LoadableTypes::default())
            .insert_resource(LoadableAssets::default())
            .add_system(switch.run_in_state(GameState::Loading))
            .add_enter_system(GameState::Loading, setup_resources)
            .add_system(load_blocks.run_in_state(GameState::Loading))
            .add_enter_system(GameState::Loading, load_entities)
            .add_enter_system(GameState::Loading, new_client)
            .add_exit_system(GameState::Loading, despawn_with::<Loading>)
            .add_system(panic_on_error_system.run_in_state(GameState::Loading))
            .add_system(panic_on_error_system.run_in_state(GameState::Game));
        // .add_system_to_stage(
        //     CoreStage::PostUpdate,
        //     disconnect_on_exit.after(exit_on_all_closed),
        // );
    }
}
