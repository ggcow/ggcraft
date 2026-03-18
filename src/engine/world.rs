use std::collections::HashSet;

use crate::engine::{atlas, mca::reader::McLoader};

macro_rules! SIZE {
    () => {
        512
    };
}

pub struct World {
    faces: Vec<Face>,
}
impl World {
    pub fn new() -> Self {
        let mut loader = McLoader::new();
        let mut textures = HashSet::new();

        let mut blocks = vec![vec![vec![None; SIZE!()]; 320]; SIZE!()];

        for x in 0..SIZE!() {
            for z in 0..SIZE!() {
                for y in 0..320 {
                    let name = loader.get_block_name([x as i32, y as i32, z as i32]);
                    let Some(name) = name else {
                        continue;
                    };

                    let name = name.replace("minecraft:", "");

                    if name == "air" {
                        continue;
                    }

                    blocks[x][y][z] = Some(
                        atlas::Block::from_name(&name).unwrap_or(atlas::Block::DiamondBlock) as u32,
                    );
                    textures.insert(name);
                }
            }
            println!("{}%", (x as f32 / SIZE!() as f32) * 100.0);
        }

        for texture in textures {
            println!("Found block: {texture}");
        }
        let mut faces = Vec::new();

        for x in 0..SIZE!() {
            for z in 0..SIZE!() {
                for y in 0..320 {
                    let Some(tex_index) = blocks[x][y][z] else {
                        continue;
                    };

                    if x == 0 || blocks[x - 1][y][z].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 0],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if x == SIZE!() - 1 || blocks[x + 1][y][z].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 1],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if y == 0 || blocks[x][y - 1][z].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 2],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if y == 319 || blocks[x][y + 1][z].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 3],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if z == 0 || blocks[x][y][z - 1].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 4],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if z == SIZE!() - 1 || blocks[x][y][z + 1].is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 5],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                }
            }
        }

        Self { faces: faces }
    }

    pub fn faces(&self) -> &[Face] {
        self.faces.as_slice()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Face {
    pub position: [i32; 4],
    pub size: [i32; 3],
    pub tex_index: u32,
}

impl Face {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Sint32x4, // position
        1 => Sint32x3, // size
        2 => Uint32,   // tex_index
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
