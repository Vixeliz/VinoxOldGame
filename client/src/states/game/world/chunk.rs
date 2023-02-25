use bevy::prelude::*;
use bevy_rapier3d::prelude::Collider;
use common::game::world::chunk::ChunkComp;

#[derive(Bundle)]
pub struct RenderedChunk {
    pub chunk: ChunkComp,
    #[bundle]
    pub mesh: PbrBundle,
    pub collider: Collider,
}
