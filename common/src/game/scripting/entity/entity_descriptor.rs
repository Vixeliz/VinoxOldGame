use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct EntityDescriptor {
    pub namespace: String,
    pub entity_name: String,
    pub model: String, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    pub friction: f32,
    pub attack: u32,
    pub entity_script: Option<String>,
    pub interactable: bool,
    pub ai_type: String
}