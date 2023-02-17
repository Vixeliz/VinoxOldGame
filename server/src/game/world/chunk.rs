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

#[derive(Resource)]
pub struct DirtyChunks {
    pub chunks: HashSet<IVec3>,
}

#[allow(dead_code)]
impl DirtyChunks {
    pub fn mark_dirty(&mut self, pos: IVec3) {
        self.0.insert(pos);
    }

    pub fn iter_dirty(&self) -> impl Iterator<Item = &IVec3> {
        self.0.iter()
    }

    pub fn num_dirty(&self) -> usize {
        self.0.len()
    }
}

#[derive(Default, Resource)]
pub struct ViewDistance {
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

#[derive(Default, Resource)]
pub struct ChunkQueue {
    pub create: Vec<IVec3>,
    pub remove: Vec<IVec3>,
}

#[derive(SystemParam)]
pub struct ChunkManager<'w, 's> {
    commands: Commands<'w, 's>,
    current_chunks: ResMut<'w, CurrentChunks>,
}

impl<'w, 's> ChunkManager<'w, 's> {}
