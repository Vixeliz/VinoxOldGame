use bevy::{app::ScheduleRunnerSettings, prelude::*};
use bevy_renet::RenetServerPlugin;
use common::networking::components::NetworkIP;
use game::setup::GamePlugin;
use iyes_loopless::prelude::*;
use std::{env, time::Duration};
mod game;
mod networking;

// Server should always keep spawn chunks loaded and any chunks near players
fn main() {
    let args: Vec<String> = env::args().collect();

    let mut ip = "127.0.0.1".to_string();
    match args.len() {
        1 => {}
        2 => {
            ip = args[1].to_string();
        }
        _ => {}
    }

    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(NetworkIP(ip))
        .add_plugins(MinimalPlugins)
        .add_plugin(RenetServerPlugin::default())
        .add_fixed_timestep(
            Duration::from_millis(16),
            // give it a label
            "fixed_update",
        )
        .add_fixed_timestep(Duration::from_millis(16), "network_update") // We may play with this value higher it is less delay and easier some things are to implement. Downside is bandwidth so look for ways to compress packets sizes. 60hz as a max goal 30hz as least
        .add_fixed_timestep_child_stage("network_update")
        .add_fixed_timestep_child_stage("fixed_update") // Send packets at simulation speed
        .add_plugin(GamePlugin)
        .run();
}
