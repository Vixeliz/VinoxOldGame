use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Component)]
pub struct ControlledPlayer;

#[derive(Debug)]
pub struct PlayerInfo {
    pub client_entity: Entity,
    pub server_entity: Entity,
}

#[derive(Debug, Default, Resource)]
pub struct ClientLobby {
    pub players: HashMap<u64, PlayerInfo>,
}

#[derive(Default, Resource)]
pub struct NetworkMapping(pub HashMap<Entity, Entity>);
