use std::fs::{File, self};
use std::io::prelude::*;
use walkdir::WalkDir;

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
    for entry in WalkDir::new("assets/blocks/").into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().unwrap_or_default() == "ron" {
            if let Ok(ron_string) = fs::read_to_string(entry.path()) {
                if let Ok(block) = ron::from_str(ron_string.as_str()) {
                    result.push(block);
                }
            }
        }    
    }
    result
}