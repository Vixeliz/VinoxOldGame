mod components;
mod states;
mod systems;
use std::time::Duration;

use belly::prelude::*;
use bevy::prelude::*;
use bevy_easings::EasingsPlugin;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_renet::RenetClientPlugin;
use components::GameState;
use iyes_loopless::prelude::*;
use states::game::setup::GamePlugin;
use states::loading::LoadingPlugin;
use states::menu::MenuPlugin;
use states::splashscreen::SplashscreenPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
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