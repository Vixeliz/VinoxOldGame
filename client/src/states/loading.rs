use bevy::{asset::LoadState, math::Vec3A, prelude::*, render::primitives::Aabb};
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use common::{
    game::{
        bundles::{AssetsLoading, PlayerBundleBuilder},
        scripting::{block::load::load_all_blocks, entity::load::load_all_entities},
        storage::{convert_block, convert_entity},
        world::chunk::LoadableTypes,
    },
    networking::components::{client_connection_config, NetworkIP, PROTOCOL_ID},
};
use iyes_loopless::{prelude::AppLooplessStateExt, state::NextState};

use crate::{
    components::{GameState, Loading},
    systems::despawn_with,
};

use std::{collections::HashMap, net::UdpSocket, time::SystemTime};

use bevy::app::AppExit;
use bevy_renet::renet::{ClientAuthentication, RenetClient, RenetError};
use iyes_loopless::prelude::*;
extern crate common;

//TODO: Right now we are building the client only as a multiplayer client. This is fine but eventually we need to have singleplayer.
// To achieve this we will just have the client start up a server. But for now I am just going to use a dedicated one for testing
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
        client_id,
        server_addr,
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
    mut loadable_assets: ResMut<LoadableAssets>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
) {
    match asset_server.get_group_load_state(loading.0.iter().map(|h| h.id)) {
        LoadState::Failed => {
            commands.insert_resource(NextState(GameState::Menu));
        }
        LoadState::Loaded => {
            if client.is_connected() {
                let mut texture_atlas_builder = TextureAtlasBuilder::default();
                for handle in loadable_assets.block_textures.values() {
                    let Some(texture) = textures.get(&handle[0]) else {
            warn!("{:?} did not resolve to an `Image` asset.", asset_server.get_handle_path(&handle[0]));
            continue;
        };

                    texture_atlas_builder.add_texture(handle[0].clone(), texture);
                }

                let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
                let atlas_handle = texture_atlases.add(texture_atlas);
                loadable_assets.block_atlas = atlas_handle;
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
    _client: ResMut<RenetClient>,
) {
    for _e in renet_error.iter() {
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
    let player_handle = asset_server.load("base_player.gltf#Scene0");
    loading.0.push(player_handle.clone_untyped());
    let player_hands_handle = asset_server.load("hands.gltf#Scene0");
    loading.0.push(player_hands_handle.clone_untyped());
    commands.insert_resource(PlayerBundleBuilder {
        default_model: player_handle,
        local_model: player_hands_handle,
        model_aabb: Aabb {
            half_extents: Vec3A::new(0.25, 1.0, 0.2),
            ..default()
        },
    });
    loadable_types.blocks = convert_block(load_all_blocks());
    loadable_types.entities = convert_entity(load_all_entities());
}

#[derive(Resource, Default, Clone)]
pub struct LoadableAssets {
    pub block_models: HashMap<String, Handle<Scene>>,
    pub block_textures: HashMap<String, [Handle<Image>; 6]>,
    pub entity_models: HashMap<String, Handle<Scene>>,
    pub block_atlas: Handle<TextureAtlas>,
}

pub fn load_blocks(
    _commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    loadable_types: Res<LoadableTypes>,
    mut loadable_assets: ResMut<LoadableAssets>,
    mut has_ran: Local<bool>,
) {
    if !(*has_ran) && loadable_types.is_changed() {
        for block_pair in &loadable_types.blocks {
            let block = block_pair.1;
            for texture_path_and_type in block.textures.iter() {
                let mut path = "blocks/".to_string();
                path.push_str(block.block_name.as_str());
                path.push('/');
                path.push_str(texture_path_and_type.1);
                let texture_handle: Handle<Image> = asset_server.load(path.as_str());
                loading.0.push(texture_handle.clone_untyped());
                let mut block_identifier = block.namespace.to_owned();
                block_identifier.push_str(&block.block_name.to_owned());
                let texture_array = [
                    texture_handle.clone(),
                    texture_handle.clone(),
                    texture_handle.clone(),
                    texture_handle.clone(),
                    texture_handle.clone(),
                    texture_handle.clone(),
                ];
                loadable_assets
                    .block_textures
                    .insert(block_identifier, texture_array);
            }
        }
        *has_ran = true;
    }
}

pub fn load_sounds() {}

pub fn load_entities(
    _commands: Commands,
    _asset_server: Res<AssetServer>,
    _loading: ResMut<AssetsLoading>,
    _loadable_types: Res<LoadableTypes>,
) {
}

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AssetsLoading::default())
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
