use bevy::prelude::*;
use common::game::world::chunk::*;
use noise::*;

pub fn generate_chunk(pos: IVec3, seed: u32) -> RawChunk {
    //TODO: Switch to using ron files to determine biomes and what blocks they should use. For now hardcoding a simplex noise
    let simplex_noise = OpenSimplex::new(seed);

    let mut raw_chunk = RawChunk::new();
    for z in 1..CHUNK_SIZE {
        for y in 1..CHUNK_SIZE {
            for x in 1..CHUNK_SIZE {
                let full_x = x as i32 + ((CHUNK_SIZE as i32) * pos.x);
                let full_z = z as i32 + ((CHUNK_SIZE as i32) * pos.z);
                // let full_y = y as i32 + ((CHUNK_SIZE as i32) * pos.y);
                let noise_val =
                    (simplex_noise.get([full_x as f64 / 5.0, full_z as f64 / 5.0]) * 10.0) + 5.0;
                if y as f64 <= noise_val {
                    raw_chunk.add_block_state(&"grass".to_string());
                    raw_chunk.set_block(
                        UVec3::new(x as u32, y as u32, z as u32),
                        "grass".to_string(),
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

#[cfg(test)]
mod tests {
    use crate::game::world::generation::*;
    use bevy::prelude::*;
    #[test]
    fn chunk_type() {
        // println!("{:?}", generate_chunk(IVec3::new(0, 0, 0), 0));
    }
}
