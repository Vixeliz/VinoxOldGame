use bevy::prelude::*;
use common::game::world::chunk::*;
use noise::{BasicMulti, MultiFractal, NoiseFn, OpenSimplex, RidgedMulti};

// Just some interesting stuff to look at while testing
pub fn add_grass(raw_chunk: &mut RawChunk) {
    for x in 1..=CHUNK_SIZE {
        for z in 1..=CHUNK_SIZE {
            for y in 1..=CHUNK_SIZE {
                if raw_chunk.get_block(UVec3::new(x, y + 1, z)).unwrap() == "air"
                    && raw_chunk.get_block(UVec3::new(x, y, z)).unwrap() == "vinoxcobblestone"
                {
                    raw_chunk.add_block_state(&"vinoxgrass".to_string());
                    raw_chunk.set_block(UVec3::new(x, y, z), "vinoxgrass".to_string());
                    raw_chunk.add_block_state(&"vinoxdirt".to_string());
                    raw_chunk.set_block(UVec3::new(x, y - 1, z), "vinoxdirt".to_string());
                }
            }
        }
    }
}

pub fn generate_chunk(pos: IVec3, seed: u32) -> RawChunk {
    //TODO: Switch to using ron files to determine biomes and what blocks they should use. For now hardcoding a simplex noise
    let ridged_noise: RidgedMulti<OpenSimplex> =
        RidgedMulti::new(seed).set_octaves(8).set_frequency(0.25);
    let basic_noise: BasicMulti<OpenSimplex> =
        BasicMulti::new(seed).set_octaves(2).set_frequency(0.5);
    let mut raw_chunk = RawChunk::new();
    for x in 1..=CHUNK_SIZE {
        for z in 1..=CHUNK_SIZE {
            for y in 1..=CHUNK_SIZE {
                let full_x = x as i32 + ((CHUNK_SIZE as i32) * pos.x);
                let full_z = z as i32 + ((CHUNK_SIZE as i32) * pos.z);
                let full_y = y as i32 + ((CHUNK_SIZE as i32) * pos.y);
                let noise_val =
                    ridged_noise.get([(full_x as f64 / 100.0), (full_z as f64 / 100.0)]) * 100.0;
                if full_y as f64 <= noise_val {
                    raw_chunk.add_block_state(&"vinoxcobblestone".to_string());
                    raw_chunk.set_block(UVec3::new(x, y, z), "vinoxcobblestone".to_string());
                } else {
                    raw_chunk.set_block(UVec3::new(x, y, z), "air".to_string());
                }

                let density = basic_noise.get([
                    (full_x as f64 / 5.0),
                    (full_y as f64 / 5.0),
                    (full_z as f64 / 5.0),
                ]) / 16.0;

                if (density * (get_value_at_height(full_y) * 2.0)) >= 0.01 {
                    raw_chunk.set_block(UVec3::new(x, y, z), "air".to_string());
                }
            }
        }
    }
    add_grass(&mut raw_chunk);
    raw_chunk
}

fn get_value_at_height(pos: i32) -> f64 {
    let max_height = 96;
    let min_height = -128;
    let mut val = pos;
    if pos > max_height {
        val = max_height;
    } else if pos < min_height {
        val = min_height;
    }

    ((val as f64 - min_height as f64) / (max_height as f64 - min_height as f64)) * (1.0 - 0.0) + 0.0
}
