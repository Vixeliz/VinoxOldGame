use bevy::{
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rand::Rng;

const CHUNK_SIZE: u32 = 16;
const CHUNK_SIZE_PADDED: u32 = CHUNK_SIZE + 1;
const TOTAL_CHUNK_SIZE: u32 = CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED * CHUNK_SIZE_PADDED;

struct Chunk {
    voxels: [bool; TOTAL_CHUNK_SIZE as usize],
    pos: IVec3,
}

struct ChunkMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(WorldInspectorPlugin)
        .add_startup_system(setup_scene)
        .run();
}

fn flatten_coord(coords: UVec3) -> usize {
    (coords.x + CHUNK_SIZE_PADDED * (coords.y + CHUNK_SIZE_PADDED * coords.z)) as usize
}

fn generate_chunk(position: IVec3) -> Chunk {
    //Do as if we were making an actual fully fleshed engine only gen blocks inside of actual chunk let padding be for other chunks
    let mut chunk = Chunk {
        voxels: [false; TOTAL_CHUNK_SIZE as usize],
        pos: position,
    };
    for x in 1..CHUNK_SIZE {
        for y in 1..CHUNK_SIZE {
            for z in 1..CHUNK_SIZE {
                let index = flatten_coord(UVec3::new(x, y, z));
                let (full_x, full_y, full_z) = (
                    x as i32 + (position.x * CHUNK_SIZE as i32),
                    y as i32 + (position.y * CHUNK_SIZE as i32),
                    z as i32 + (position.z * CHUNK_SIZE as i32),
                ); // These are full world coordinates of the voxels for generation
                if full_y == 9 {
                    chunk.voxels[index] = rand::thread_rng().gen_bool(0.5); // Would use simplex noise or similiar for actual game
                } else if full_y <= 8 {
                    chunk.voxels[index] = true;
                }
            }
        }
    }
    chunk
}

fn build_mesh(chunk: &Chunk) -> ChunkMesh {
    // 0 and CHUNK_SIZE_PADDED dont get built into the mesh itself its data for meshing from other chunks this is just one solution
    let mut res = ChunkMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
    };

    for x in 0..CHUNK_SIZE_PADDED {
        for y in 0..CHUNK_SIZE_PADDED {
            for z in 0..CHUNK_SIZE_PADDED {
                if x != 0
                    || y != 0
                    || z != 0
                    || x != CHUNK_SIZE_PADDED
                    || y != CHUNK_SIZE_PADDED
                    || z != CHUNK_SIZE_PADDED
                {
                    let index = flatten_coord(UVec3::new(x, y, z));
                    // Make sure this voxel is solid since we dont need to mesh air
                    if chunk.voxels[index] == true {
                        let neighbors = [
                            chunk.voxels[flatten_coord(UVec3::new(x, y, z + 1))],
                            chunk.voxels[flatten_coord(UVec3::new(x + 1, y, z))],
                            chunk.voxels[flatten_coord(UVec3::new(x, y, z - 1))],
                            chunk.voxels[flatten_coord(UVec3::new(x - 1, y, z))],
                            chunk.voxels[flatten_coord(UVec3::new(x, y + 1, z))],
                            chunk.voxels[flatten_coord(UVec3::new(x, y - 1, z))],
                        ]; // 0 is north, 1 can be west, 2 south, 3 east, 4 up, 5 down
                        if !neighbors[0] {
                            // Front face
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 3);
                            res.indices.push(start + 2);
                            res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                            res.normals.push([0.0, 0.0, -1.0]);
                            res.normals.push([0.0, 0.0, -1.0]);
                            res.normals.push([0.0, 0.0, -1.0]);
                            res.normals.push([0.0, 0.0, -1.0]);
                        }
                        if !neighbors[1] {
                            // West
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 3);
                            res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                            res.normals.push([1.0, 0.0, 0.0]);
                            res.normals.push([1.0, 0.0, 0.0]);
                            res.normals.push([1.0, 0.0, 0.0]);
                            res.normals.push([1.0, 0.0, 0.0]);
                        }
                        if !neighbors[2] {
                            // Back face
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 3);
                            res.positions.push([x as f32, y as f32, z as f32]);
                            res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                            res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                            res.normals.push([0.0, 0.0, 1.0]);
                            res.normals.push([0.0, 0.0, 1.0]);
                            res.normals.push([0.0, 0.0, 1.0]);
                            res.normals.push([0.0, 0.0, 1.0]);
                        }
                        if !neighbors[3] {
                            // East
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 3);
                            res.indices.push(start + 2);
                            res.positions.push([x as f32, y as f32, z as f32]);
                            res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                            res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                            res.positions
                                .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                            res.normals.push([-1.0, 0.0, 0.0]);
                            res.normals.push([-1.0, 0.0, 0.0]);
                            res.normals.push([-1.0, 0.0, 0.0]);
                            res.normals.push([-1.0, 0.0, 0.0]);
                        }
                        if !neighbors[4] {
                            // Top face
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 3);
                            res.positions.push([x as f32, y as f32 + 1.0, z as f32]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32]);
                            res.positions
                                .push([x as f32, y as f32 + 1.0, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0]);
                            res.normals.push([0.0, 1.0, 0.0]);
                            res.normals.push([0.0, 1.0, 0.0]);
                            res.normals.push([0.0, 1.0, 0.0]);
                            res.normals.push([0.0, 1.0, 0.0]);
                        }
                        if !neighbors[5] {
                            // Bottom face
                            let start = res.positions.len() as u32;
                            res.indices.push(start);
                            res.indices.push(start + 1);
                            res.indices.push(start + 2);
                            res.indices.push(start + 1);
                            res.indices.push(start + 3);
                            res.indices.push(start + 2);
                            res.positions.push([x as f32, y as f32, z as f32]);
                            res.positions.push([x as f32 + 1.0, y as f32, z as f32]);
                            res.positions.push([x as f32, y as f32, z as f32 + 1.0]);
                            res.positions
                                .push([x as f32 + 1.0, y as f32, z as f32 + 1.0]);
                            res.normals.push([0.0, -1.0, 0.0]);
                            res.normals.push([0.0, -1.0, 0.0]);
                            res.normals.push([0.0, -1.0, 0.0]);
                            res.normals.push([0.0, -1.0, 0.0]);
                        }
                    }
                } else {
                }
            }
        }
    }
    res
}

fn setup_scene(
    mut wireframe_config: ResMut<WireframeConfig>,

    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    wireframe_config.global = true;
    commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });
    commands.insert_resource(AmbientLight {
        brightness: 1.0,
        ..default()
    });
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1000.0,
            ..default()
        },
        ..default()
    });

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let start = vertices.len();
    indices.push(start as u32);
    indices.push((start + 1) as u32);
    indices.push((start + 2) as u32);
    indices.push((start + 1) as u32);
    indices.push((start + 3) as u32);
    indices.push((start + 2) as u32);
    vertices.push([0.0, 0.0, 1.0]);
    vertices.push([1.0, 0.0, 1.0]);
    vertices.push([0.0, 1.0, 1.0]);
    vertices.push([1.0, 1.0, 1.0]);
    normals.push([0.0, 0.0, 1.0]);
    normals.push([0.0, 0.0, 1.0]);
    normals.push([0.0, 0.0, 1.0]);
    normals.push([0.0, 0.0, 1.0]);
    let mut simple_mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
    simple_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    simple_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    simple_mesh.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));
    commands.spawn(PbrBundle {
        mesh: meshes.add(simple_mesh),
        material: materials.add(StandardMaterial {
            base_color: Color::GREEN,
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..default()
    });

    // CHUNK
    let chunk = generate_chunk(IVec3::new(0, 0, 0));
    let render_chunk = build_mesh(&chunk);
    let mut render_mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, render_chunk.positions);
    render_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, render_chunk.normals);
    render_mesh.set_indices(Some(bevy::render::mesh::Indices::U32(render_chunk.indices)));
    commands.spawn(PbrBundle {
        mesh: meshes.add(render_mesh),
        material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        }),
        transform: Transform::from_translation(Vec3::new(
            (chunk.pos.x * (CHUNK_SIZE - 2) as i32) as f32, // -2 is for the padding since that doesnt matter for positioning
            (chunk.pos.x * (CHUNK_SIZE - 2) as i32) as f32,
            (chunk.pos.x * (CHUNK_SIZE - 2) as i32) as f32,
        )),
        ..default()
    });
}
