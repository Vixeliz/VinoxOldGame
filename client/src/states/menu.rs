use std::env;

use crate::components::*;

use crate::systems::despawn_with;
use belly::prelude::*;
use bevy::app::AppExit;
use bevy::prelude::*;
use common::networking::components::{NetworkIP};
use iyes_loopless::prelude::*;

pub struct StartEvent;
pub struct QuitEvent;

pub fn setup(mut commands: Commands, _asset_server: Res<AssetServer>) {
    commands.spawn((Menu, Camera2dBundle::default()));
    commands.add(StyleSheet::parse(
        r#"
        .text-input-value {
            color: #2f2f2f;
        }
        .text-input-border:focus {
            background-color: #2f2f2f;
        }
        span {
            width: 100px;
        }
        body: {
            padding: 20px;
            justify-content: center;
            align-content: center;
            align-items: center;
        }
        div: {
            justify-content: center;
        }
        button: {
            width: 200px;
            height: 50px;
        }
        button .content {
            width: 100%;
            height: 100%;
            justify-content: center;
            align-items: center;
        }
    "#,
    ));
    let input = commands.spawn_empty().insert(Menu).id();
    let label = commands.spawn_empty().insert(Menu).id();
    commands.add(eml! {
        <body s:padding="5px" with=Menu>
        <div>
            <span> "Type Input" </span>
            <textinput {input} bind:value=to!(label, Label:value | fmt.val("I'm bound to label, {val}!")) s:width="150px"/>
            <brl/>
        </div>
        <div>
             <button on:press=connect!(|ctx| {
                ctx.send_event(StartEvent{})
                })>
                    "Play"
            </button>
        </div>
        <div>
             <button on:press=connect!(|ctx| {
                ctx.send_event(QuitEvent{})
                })>
                    "Quit"
            </button>
        </div>
        </body>
    });
}

pub fn input(
    mut commands: Commands,
    keyboard_input: Res<Input<KeyCode>>,
    _text_query: Query<&mut Text>,
    label: Query<&Label>,
    mut ip_res: ResMut<NetworkIP>,
) {
    if let Ok(label_val) = label.get_single() {
        if !label_val.value.is_empty() {
            ip_res.0 = (*label_val.value).to_string();
        }
    }
    if keyboard_input.just_pressed(KeyCode::Escape) {
        commands.insert_resource(NextState(GameState::Loading));
    }
}

pub fn start_event(mut commands: Commands, mut events: EventReader<StartEvent>) {
    for _event in events.iter() {
        commands.insert_resource(NextState(GameState::Loading));
    }
}

pub fn quit_event(mut events: EventReader<QuitEvent>, mut exit: EventWriter<AppExit>) {
    for _event in events.iter() {
        exit.send(AppExit);
    }
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        let args: Vec<String> = env::args().collect();

        let mut ip = "127.0.0.1".to_string();
        match args.len() {
            1 => {}
            2 => {
                ip = args[1].to_string();
            }
            _ => {}
        }

        app.add_system(input.run_in_state(GameState::Menu))
            .add_enter_system(GameState::Menu, setup)
            .add_exit_system(GameState::Menu, despawn_with::<Menu>)
            .add_system(start_event.run_in_state(GameState::Menu))
            .add_system(quit_event.run_in_state(GameState::Menu))
            .add_event::<StartEvent>()
            .add_event::<QuitEvent>()
            .insert_resource(NetworkIP(ip));
    }
}
