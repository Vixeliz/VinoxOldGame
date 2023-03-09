use bevy::{asset::LoadState, math::Vec3A, prelude::*, render::primitives::Aabb};

use bevy_quinnet::client::{
    certificate::CertificateVerificationMode,
    connection::{ConnectionConfiguration, ConnectionEvent},
    Client,
};
use common::{
    game::{
        bundles::{AssetsLoading, PlayerBundleBuilder},
        scripting::{block::load::load_all_blocks, entity::load::load_all_entities},
        storage::{convert_block, convert_entity},
        world::chunk::LoadableTypes,
    },
    networking::components::NetworkIP,
};
use iyes_loopless::{prelude::AppLooplessStateExt, state::NextState};

use crate::{
    components::{GameState, Loading},
    systems::despawn_with,
};

use std::collections::HashMap;

use iyes_loopless::prelude::*;

use super::game::networking::components::ClientData;
extern crate common;

//TODO: Right now we are building the client only as a multiplayer client. This is fine but eventually we need to have singleplayer.
// To achieve this we will just have the client start up a server. But for now I am just going to use a dedicated one for testing
pub fn new_client(ip_res: Res<NetworkIP>, mut client: ResMut<Client>) {
    client
        .open_connection(
            ConnectionConfiguration::new(ip_res.0.clone(), 25565, "0.0.0.0".to_string(), 0),
            CertificateVerificationMode::SkipVerification,
        )
        .unwrap();
}

pub fn switch(
    mut commands: Commands,
    mut client: ResMut<Client>,
    loading: Res<AssetsLoading>,
    asset_server: Res<AssetServer>,
    mut loadable_assets: ResMut<LoadableAssets>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut textures: ResMut<Assets<Image>>,
    mut connected_event: EventReader<ConnectionEvent>,
) {
    match asset_server.get_group_load_state(loading.0.iter().map(|h| h.id)) {
        LoadState::Failed => {
            commands.insert_resource(NextState(GameState::Menu));
        }
        LoadState::Loaded => {
            for _ in connected_event.iter() {
                client.connection_mut().set_default_channel(
                    bevy_quinnet::shared::channel::ChannelId::UnorderedReliable,
                );
                let mut texture_atlas_builder = TextureAtlasBuilder::default();
                for handle in loadable_assets.block_textures.values() {
                    for item in handle {
                        let Some(texture) = textures.get(item) else {
            warn!("{:?} did not resolve to an `Image` asset.", asset_server.get_handle_path(item));
            continue;
        };

                        texture_atlas_builder.add_texture(item.clone(), texture);
                    }
                }

                let texture_atlas = texture_atlas_builder.finish(&mut textures).unwrap();
                let atlas_handle = texture_atlases.add(texture_atlas);
                loadable_assets.block_atlas = atlas_handle;
                commands.insert_resource(NextState(GameState::Game));
                // remove the resource to drop the tracking handles
                // commands.remove_resource::<AssetsLoading>();
                // (note: if you don't have any other handles to the assets
                // elsewhere, they will get unloaded after this)
            }
        }
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

// Move to game state
// fn disconnect_on_exit(exit: EventReader<AppExit>, mut client: ResMut<RenetClient>) {
//     if !exit.is_empty() && client.is_connected() {
//         client.disconnect();
//     }
// }

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
            let mut texture_array: Vec<Handle<Image>> = Vec::with_capacity(6);
            texture_array.resize(6, Handle::default());
            let mut block_identifier = String::new();
            for texture_path_and_type in block.textures.iter() {
                let mut path = "blocks/".to_string();
                path.push_str(block.block_name.as_str());
                path.push('/');
                path.push_str(texture_path_and_type.1);
                let texture_handle: Handle<Image> = asset_server.load(path.as_str());
                loading.0.push(texture_handle.clone_untyped());
                block_identifier = block.namespace.to_owned();
                block_identifier.push_str(&block.block_name.to_owned());
                match texture_path_and_type.0.as_str() {
                    "up" => {
                        texture_array[0] = texture_handle;
                    }
                    "down" => {
                        texture_array[1] = texture_handle;
                    }
                    "left" => {
                        texture_array[2] = texture_handle;
                    }
                    "right" => {
                        texture_array[3] = texture_handle;
                    }
                    "front" => {
                        texture_array[4] = texture_handle;
                    }
                    "back" => {
                        texture_array[5] = texture_handle;
                    }
                    _ => {}
                }
            }
            let texture_array: [Handle<Image>; 6] =
                texture_array
                    .try_into()
                    .unwrap_or_else(|texture_array: Vec<Handle<Image>>| {
                        panic!(
                            "Expected a Vec of length {} but it was {}",
                            6,
                            texture_array.len()
                        )
                    });
            loadable_assets
                .block_textures
                .insert(block_identifier, texture_array);
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
        app.insert_resource(ClientData::default())
            .insert_resource(AssetsLoading::default())
            .insert_resource(LoadableTypes::default())
            .insert_resource(LoadableAssets::default())
            .add_system(switch.run_in_state(GameState::Loading))
            .add_enter_system(GameState::Loading, setup_resources)
            .add_system(load_blocks.run_in_state(GameState::Loading))
            .add_enter_system(GameState::Loading, load_entities)
            .add_enter_system(GameState::Loading, load_sounds)
            .add_enter_system(GameState::Loading, new_client)
            .add_exit_system(GameState::Loading, despawn_with::<Loading>);
        // .add_system(panic_on_error_system.run_in_state(GameState::Loading))
        // .add_system(panic_on_error_system.run_in_state(GameState::Game));
        // .add_system_to_stage(
        //     CoreStage::PostUpdate,
        //     disconnect_on_exit.after(exit_on_all_closed),
        // );
    }
}
