use bevy::prelude::*;

#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum GameState {
    Splashscreen,
    Menu,
    Game,
    Loading,
}

// Tags for all the different states
#[derive(Default, Component, Clone)]
pub struct Splashscreen;
#[derive(Default, Component, Clone)]
pub struct Menu;
#[derive(Default, Component, Clone)]
pub struct Game;

#[derive(Default, Component, Clone)]
pub struct Loading;

pub const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
pub const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
pub const PRESSED_BUTTON: Color = Color::rgb(0.50, 0.50, 0.50);
