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
