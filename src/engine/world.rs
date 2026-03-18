use crate::engine::mca::reader::McLoader;

pub struct World {
    faces: Vec<Face>,
}
impl World {
    pub fn new() -> Self {
        let mut loader = McLoader::new();

        let mut blocks = vec![vec![vec![0u8; 64 * 2]; 320]; 64 * 2];
        for x in 0..128 {
            for z in 0..128 {
                for y in 0..320 {
                    let name = loader.get_block_name([x as i32 - 64, y as i32, z as i32 - 64]);
                    let Some(name) = name else {
                        continue;
                    };

                    if name == "minecraft:air" {
                        continue;
                    }

                    blocks[x][y][z] = 1;
                }
            }
        }

        let faces = vec![
            Face {
                position: [0, 0, 0, 0],
                size: [10, 50, 100],
            },
            Face {
                position: [0, 0, 0, 1],
                size: [10, 50, 100],
            },
            Face {
                position: [0, 0, 0, 2],
                size: [10, 50, 100],
            },
            Face {
                position: [0, 0, 0, 3],
                size: [10, 50, 100],
            },
            Face {
                position: [0, 0, 0, 4],
                size: [10, 50, 100],
            },
            Face {
                position: [0, 0, 0, 5],
                size: [10, 50, 100],
            },
        ];

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
}

impl Face {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Sint32x4, // position
        1 => Sint32x3, // size
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
