use directories::ProjectDirs;
use std::fs::{self, File};
use std::io::prelude::*;
use walkdir::WalkDir;

use super::entity_descriptor::EntityDescriptor;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn show_entities() {
        println!("{:?}", load_all_entities());
    }
}

pub fn load_all_entities() -> Vec<EntityDescriptor> {
    let mut result = Vec::new();
    if let Some(proj_dirs) = ProjectDirs::from("com", "vinox", "vinox") {
        for entry in WalkDir::new(proj_dirs.data_dir().join("assets"))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().unwrap_or_default() == "ron" {
                if let Ok(ron_string) = fs::read_to_string(entry.path()) {
                    if let Ok(entity) = ron::from_str(ron_string.as_str()) {
                        result.push(entity);
                    }
                }
            }
        }
    }
    result
}
