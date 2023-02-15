use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct EntityDescriptor {
    namespace: String,
    entity_name: String,
    model: String, // We will allow someone to specify a gltf however i want to build in a few types such as slabs
    friction: f32,
    attack: u32,
    entity_script: Option<String>,
    interactable: bool,
    ai_type: String
}