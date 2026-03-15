use crate::engine::mca::reader::McLoader;

pub struct World {
    faces: Vec<Face>,
}
impl World {
    pub fn new() -> Self {
        let mut loader = McLoader::new();

        // const SPACE_BETWEEN: f32 = 2.0;
        // const NUM_INSTANCES_PER_ROW: u32 = 16;

        let mut ret = Vec::new();
        let mut blocks = vec![vec![vec![0u8; 128]; 320]; 128];
        for x in -64..64 {
            for z in -64..64 {
                for y in 0..320 {
                    let name = loader.get_block_name([x, y, z]);
                    // let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    // let y = SPACE_BETWEEN * (y as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    // let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let Some(name) = name else {
                        continue;
                    };

                    if name == "minecraft:air" {
                        continue;
                    }

                    blocks[(x + 64) as usize][(y) as usize][(z + 64) as usize] = 1;
                }
            }
        }

        for x in 0..128 {
            for z in 0..128 {
                for y in 0..320 {
                    if blocks[x][y][z] == 0 {
                        continue;
                    }

                    // left
                    if x == 0 || blocks[x - 1][y][z] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 0],
                            size: [1, 1],
                        });
                    }

                    // right
                    if x == 127 || blocks[x + 1][y][z] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 1],
                            size: [1, 1],
                        });
                    }

                    // down
                    if y == 0 || blocks[x][y - 1][z] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 2],
                            size: [1, 1],
                        });
                    }

                    // up
                    if y == 319 || blocks[x][y + 1][z] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 3],
                            size: [1, 1],
                        });
                    }

                    // back
                    if z == 0 || blocks[x][y][z - 1] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 4],
                            size: [1, 1],
                        });
                    }

                    // front
                    if z == 127 || blocks[x][y][z + 1] == 0 {
                        ret.push(Face {
                            position: [x as i32, y as i32, z as i32, 5],
                            size: [1, 1],
                        });
                    }
                }
            }
        }

        Self { faces: ret }
    }

    pub fn faces(&self) -> &[Face] {
        self.faces.as_slice()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Face {
    pub position: [i32; 4],
    pub size: [i32; 2],
}

impl Face {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Sint32x4, // position
        1 => Sint32x2, // size
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Face>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}
