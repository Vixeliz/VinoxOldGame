use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;
use common::game::world::chunk::RawChunk;
use rusqlite::*;
use zstd::stream::{copy_decode, copy_encode};

#[derive(Resource)]
pub struct WorldDatabase {
    pub name: String,
    pub connection: Arc<Mutex<Connection>>,
}

pub fn create_database(database: &Connection) {
    database
        .execute(
            " create table if not exists blocks (
            posx integer not null,
            posy integer not null,
            posz integer not null,
            data blob,
            PRIMARY KEY (posx, posy, posz)
        )",
            [],
        )
        .unwrap();
}

pub fn insert_chunk(chunk_pos: IVec3, raw_chunk: &RawChunk, database: &Connection) {
    if let Ok(raw_chunk_bin) = bincode::serialize(raw_chunk) {
        let mut final_chunk = Cursor::new(raw_chunk_bin);
        let mut output = Cursor::new(Vec::new());
        copy_encode(&mut final_chunk, &mut output, 0).unwrap();
        database
            .execute(
                "REPLACE INTO blocks (posx, posy, posz, data) values (?1, ?2, ?3, ?4)",
                params![
                    &chunk_pos.x,
                    &chunk_pos.y,
                    &chunk_pos.z,
                    &output.get_ref().clone(),
                ],
            )
            .unwrap();
    }
}

pub fn load_chunk(chunk_pos: IVec3, database: &Connection) -> Option<RawChunk> {
    let stmt = database.prepare(
        "SELECT posx, posy, posz, data FROM blocks WHERE posx=:posx AND posy=:posy AND posz=:posz;",
    );
    if let Ok(mut stmt) = stmt {
        let chunk_result: Result<Vec<u8>, _> = stmt.query_row(
            &[
                (":posx", &chunk_pos.x),
                (":posy", &chunk_pos.y),
                (":posz", &chunk_pos.z),
            ],
            |row| Ok(row.get(3).unwrap()),
        );
        if let Ok(chunk_row) = chunk_result {
            let mut temp_output = Cursor::new(Vec::new());
            copy_decode(&chunk_row[..], &mut temp_output).unwrap();
            let final_chunk = bincode::deserialize(temp_output.get_ref()).unwrap();
            return Some(final_chunk);
        }
    }

    None
}
