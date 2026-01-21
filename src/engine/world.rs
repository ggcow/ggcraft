use crate::engine::model::CubeFace;

const WORLD_SIZE: usize = 64;

pub struct World {
    pub faces: Vec<CubeFace>,
}

fn sphere(x: f32, y: f32, z: f32, radius: f32) -> bool {
    x * x + y * y + z * z <= radius * radius
}

impl World {
    pub fn new() -> Self {
        let mut faces = Vec::new();
        for x in 0..WORLD_SIZE {
            for y in 0..WORLD_SIZE {
                for z in 0..WORLD_SIZE {
                    if sphere(
                        x as f32 - WORLD_SIZE as f32 / 2.0,
                        y as f32 - WORLD_SIZE as f32 / 2.0,
                        z as f32 - WORLD_SIZE as f32 / 2.0,
                        WORLD_SIZE as f32 / 2.0,
                    ) {
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 0],
                        });
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 1],
                        });
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 2],
                        });
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 3],
                        });
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 4],
                        });
                        faces.push(CubeFace {
                            position: [x as i32, y as i32, z as i32, 5],
                        });
                    }
                }
            }
        }

        Self { faces }
    }
}
