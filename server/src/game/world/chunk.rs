use bevy::prelude::*;
use std::collections::*;

#[derive(Resource)]
pub struct CurrentChunks {
    pub chunks: HashMap<IVec3, Entity>,
}

impl CurrentChunks {
    pub fn insert_entity(&mut self, pos: IVec3, entity: Entity) {
        self.0.insert(pos, entity);
    }

    pub fn remove_entity(&mut self, pos: IVec3) -> Option<Entity> {
        self.0.remove(&pos)
    }

    pub fn get_entity(&self, pos: IVec3) -> Option<Entity> {
        self.0.get(&pos).map(|x| x.clone())
    }
}

#[derive(Resource)]
pub struct CurrentLoadPoints {
    pub points: Vec<IVec3>,
}

#[derive(Default, Resource)]
pub struct ViewDistance {
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    commands: Commands<'w, 's>,
}

impl<'w, 's> ChunkManager<'w, 's> {}
