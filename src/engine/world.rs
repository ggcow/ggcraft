use std::collections::HashSet;

use crate::engine::{atlas, mca::reader::McLoader};

macro_rules! SIZE {
    () => {
        256
    };
}

pub struct World {
    faces: Vec<Face>,
}

struct Blocks {
    blocks: rustc_hash::FxHashMap<(i32, i32), [Option<u32>; 320]>,
}

impl Blocks {
    fn new() -> Self {
        Self {
            blocks: Default::default(),
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: u32) {
        let column = self.blocks.entry((x, z)).or_insert_with(|| [None; 320]);

        column[y as usize] = Some(block);
    }
    fn get(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        let idx = self.blocks.get(&(x, z))?.get(y as usize)?;
        if let Some(idx) = idx {
            Some(*idx)
        } else {
            None
        }
    }
}

impl World {
    pub fn new() -> Self {
        let mut loader = McLoader::new();
        let mut textures = HashSet::new();

        let mut blocks = Blocks::new();

        println!("");
        for x in -SIZE!()..SIZE!() {
            for z in -SIZE!()..SIZE!() {
                for y in 0..320 {
                    let name = loader.get_block_name([x as i32, y as i32, z as i32]);
                    let Some(name) = name else {
                        continue;
                    };

                    let n = match name.strip_prefix("minecraft:") {
                        Some(name) => name.to_string(),
                        None => name,
                    };
                    // let name = name.replace("minecraft:", "");

                    if n == "air" {
                        continue;
                    }
                    blocks.set(
                        x,
                        y,
                        z,
                        atlas::Block::from_name(&n).unwrap_or(atlas::Block::DiamondBlock) as u32,
                    );
                    // blocks.insert((x, z), )
                    // blocks[x as usize][y as usize][z as usize] = Some(
                    //     atlas::Block::from_name(&name).unwrap_or(atlas::Block::DiamondBlock) as u32,
                    // );
                    textures.insert(n);
                }
            }
            print!("\r{:.02}%", (x as f32 / SIZE!() as f32) * 100.0);
        }
        println!("");

        for texture in textures {
            println!("Found block: {texture}");
        }
        let mut faces = Vec::new();

        for x in -SIZE!()..SIZE!() {
            for z in -SIZE!()..SIZE!() {
                for y in 0..320 {
                    let Some(tex_index) = blocks.get(x, y, z) else {
                        continue;
                    };

                    if x == 0 || blocks.get(x - 1, y, z).is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 0],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if x == SIZE!() - 1 || blocks.get(x + 1, y, z).is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 1],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if y == 0 || blocks.get(x, y - 1, z).is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 2],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if y == 319 || blocks.get(x, y + 1, z).is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 3],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if z == 0 || blocks.get(x, y, z - 1).is_none() {
                        faces.push(Face {
                            position: [x as i32, y as i32, z as i32, 4],
                            size: [1, 1, 1],
                            tex_index,
                        });
                    }
                    if z == SIZE!() - 1 || blocks.get(x, y, z + 1).is_none() {
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
