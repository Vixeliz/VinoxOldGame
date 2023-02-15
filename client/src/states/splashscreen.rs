use std::time::Duration;

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::components::{GameState, Splashscreen};
use crate::systems::*;

#[derive(Resource)]
pub struct SplashTimer {
    timer: Timer,
}

pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
 
    commands.spawn((Splashscreen, Camera2dBundle::default()));
    commands.spawn((
        Splashscreen,
        SpriteBundle {
            texture: asset_server.load("Title.png"),
            transform: Transform::from_scale(Vec3::splat(0.0)),
            ..default()
        }
    ));
    commands.insert_resource(SplashTimer {
        // create the repeating timer
        timer: Timer::new(Duration::from_secs(5), TimerMode::Once),
    });
}

pub fn update(mut commands: Commands, time: Res<Time>, mut timer: ResMut<SplashTimer>) {
    timer.timer.tick(time.delta());
    if timer.timer.finished() {
        commands.insert_resource(NextState(GameState::Menu));
    }
}
pub fn input(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    _mouse_buttons: Res<Input<MouseButton>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        commands.insert_resource(NextState(GameState::Menu));
    }
}

pub fn gamepad_input_events(mut commands: Commands, mut gamepad_evr: EventReader<GamepadEvent>) {
    for ev in gamepad_evr.iter() {
        match ev.event_type {
            GamepadEventType::Disconnected => {}
            GamepadEventType::Connected(_) => {}
            _ => {
                commands.insert_resource(NextState(GameState::Menu));
            }
        }
    }
}

pub struct SplashscreenPlugin;

impl Plugin for SplashscreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::Splashscreen, setup)
            .add_exit_system(GameState::Splashscreen, despawn_with::<Splashscreen>)
            .add_system(input.run_in_state(GameState::Splashscreen))
            .add_system(gamepad_input_events.run_in_state(GameState::Splashscreen))
            .add_system(update.run_in_state(GameState::Splashscreen));
    }
}
