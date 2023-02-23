use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;
use common::game::world::chunk::Chunk;

#[derive(Bundle)]
pub struct RenderedChunk {
    pub chunk: Chunk,
    #[bundle]
    pub mesh: PbrBundle,
    pub collider: Collider,
}
