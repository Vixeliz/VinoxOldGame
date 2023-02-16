use std::fs::{File, self};
use std::io::prelude::*;
use walkdir::WalkDir;
use directories::ProjectDirs;

use super::block_descriptor::BlockDescriptor;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn show_blocks() {
        println!("{:?}", load_all_blocks());
    }
}

pub fn load_all_blocks() -> Vec<BlockDescriptor> {
    let mut result = Vec::new();
    if let Some(proj_dirs) = ProjectDirs::from("com", "vinox",  "vinox") { 
    for entry in WalkDir::new(proj_dirs.data_dir().join("assets")).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().unwrap_or_default() == "ron" {
            if let Ok(ron_string) = fs::read_to_string(entry.path()) {
                if let Ok(block) = ron::from_str(ron_string.as_str()) {
                    result.push(block);
                }
            }
        }    
    }
    }
    result
}