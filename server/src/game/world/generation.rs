use bevy::prelude::*;
use common::game::world::chunk::*;
use noise::*;
use rand::Rng;

pub fn generate_chunk(pos: IVec3, seed: u32) -> RawChunk {
    //TODO: Switch to using ron files to determine biomes and what blocks they should use. For now hardcoding a simplex noise
    let simplex_noise = OpenSimplex::new(seed);

    let mut raw_chunk = RawChunk::new();
    for z in 1..(CHUNK_SIZE - 1) {
        for y in 1..(CHUNK_SIZE - 1) {
            for x in 1..(CHUNK_SIZE - 1) {
                let full_x = x as i32 + ((CHUNK_SIZE as i32) * pos.x);
                let full_z = z as i32 + ((CHUNK_SIZE as i32) * pos.z);
                let full_y = y as i32 + ((CHUNK_SIZE as i32) * pos.y);
                let noise_val =
                    (simplex_noise.get([full_x as f64 / 15.0, full_z as f64 / 15.0]) * 10.0) + 5.0;
                if full_y as f64 <= noise_val && full_y as f64 >= (noise_val - 2.0) {
                    raw_chunk.add_block_state(&"vinoxgrass".to_string());
                    raw_chunk.set_block(
                        UVec3::new(x as u32, y as u32, z as u32),
                        "vinoxgrass".to_string(),
                    );
                } else if full_y as f64 <= noise_val {
                    raw_chunk.add_block_state(&"vinoxdirt".to_string());
                    raw_chunk.set_block(
                        UVec3::new(x as u32, y as u32, z as u32),
                        "vinoxdirt".to_string(),
                    );
                } else {
                    raw_chunk
                        .set_block(UVec3::new(x as u32, y as u32, z as u32), "air".to_string());
                }
            }
        }
    }
    raw_chunk
}
