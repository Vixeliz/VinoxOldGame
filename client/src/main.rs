mod components;
mod states;
mod systems;
use std::path::PathBuf;
use std::time::Duration;

use belly::prelude::*;
use bevy::prelude::*;
use bevy_easings::EasingsPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_renet::RenetClientPlugin;
use components::GameState;
use directories::ProjectDirs;
use iyes_loopless::prelude::*;
use states::game::setup::GamePlugin;
use states::loading::LoadingPlugin;
use states::menu::MenuPlugin;
use states::splashscreen::SplashscreenPlugin;

fn main() {
    let mut asset_path = if let Some(proj_dirs) = ProjectDirs::from("com", "vinox", "vinox") {
        proj_dirs.data_dir().join("assets")
    } else {
        let mut path = PathBuf::new();
        path.push("assets");
        path
    };
    //TODO: make directory for assets if it doesn't exist and also copy over the game assets to it
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    asset_folder: asset_path.to_string_lossy().to_string(),
                    watch_for_changes: false,
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugin(BellyPlugin)
        .add_plugin(EasingsPlugin)
        .add_plugin(WorldInspectorPlugin)
        .add_plugin(RenetClientPlugin::default())
        .add_fixed_timestep(
            Duration::from_millis(16),
            // give it a label
            "fixed_update",
        )
        .add_fixed_timestep(Duration::from_millis(16), "network_update") // We may play with this value higher it is less delay and easier some things are to implement. Downside is bandwidth so look for ways to compress packets sizes. 60hz as a max goal 30hz as least
        .add_fixed_timestep_child_stage("network_update")
        .add_fixed_timestep_child_stage("fixed_update") // Send packets at simulation speed
        .add_loopless_state(GameState::Splashscreen)
        .add_plugin(SplashscreenPlugin)
        .add_plugin(MenuPlugin)
        .add_plugin(LoadingPlugin)
        .add_plugin(GamePlugin)
        .add_startup_system(systems::start)
        .run();
}