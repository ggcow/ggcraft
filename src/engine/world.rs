use crate::engine::atlas;

macro_rules! SIZE {
    () => {
        128
    };
}

pub struct World {
    faces: Vec<Face>,
}

#[derive(Debug, Clone, Copy)]
enum BlockFaces {
    AllSame(atlas::Block),
    Complex {
        top: atlas::Block,
        sides: atlas::Block,
        bottom: atlas::Block,
        color: [u8; 4],
    },
}

impl BlockFaces {
    fn color(&self) -> u32 {
        match self {
            BlockFaces::AllSame(_) => 0xffffffff,
            BlockFaces::Complex { color, .. } => {
                let [r, g, b, a] = *color;
                ((r as u32) << 24) + ((g as u32) << 16) + ((b as u32) << 8) + (a as u32)
            }
        }
    }
    fn top(&self) -> u32 {
        match self {
            BlockFaces::AllSame(x) => *x as u32,
            BlockFaces::Complex { top, .. } => *top as u32,
        }
    }
    fn side(&self) -> u32 {
        match self {
            BlockFaces::AllSame(x) => *x as u32,
            BlockFaces::Complex { sides, .. } => *sides as u32,
        }
    }
    fn bottom(&self) -> u32 {
        match self {
            BlockFaces::AllSame(x) => *x as u32,
            BlockFaces::Complex { bottom, .. } => *bottom as u32,
        }
    }
}

struct Blocks {
    blocks: rustc_hash::FxHashMap<(i32, i32), [Option<BlockFaces>; 320]>,
}

impl Blocks {
    fn new() -> Self {
        Self {
            blocks: Default::default(),
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: BlockFaces) {
        let column = self.blocks.entry((x, z)).or_insert_with(|| [None; 320]);

        column[y as usize] = Some(block);
    }
    fn get(&self, x: i32, y: i32, z: i32) -> Option<BlockFaces> {
        let idx = self.blocks.get(&(x, z))?.get(y as usize)?;
        if let Some(idx) = idx {
            Some(idx.clone())
        } else {
            None
        }
    }
}

impl World {
    #[cfg(feature = "hermit")]
    pub fn new() -> Self {
        let mut loader = crate::engine::mca::reader::McLoader::new();
        let mut textures = rustc_hash::FxHashMap::<String, i32>::default();

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

                    if matches!(n.as_str(), "air" | "cave_air" | "void_air") {
                        continue;
                    }
                    let t = match atlas::Block::from_stem(&n) {
                        Some(face) => BlockFaces::AllSame(face),

                        None => match n.as_str() {
                            // "infested_stone" => BlockFaces::AllSame(atlas::Block::Stone),
                            // "snow_block" => BlockFaces::AllSame(atlas::Block::Snow),
                            // "grass_block" => BlockFaces::Complex {
                            //     color: [0, 255, 0, 255],
                            //     bottom: atlas::Block::Dirt,
                            //     top: atlas::Block::GrassBlockTop,
                            //     sides: atlas::Block::GrassBlockSide,
                            // },
                            // "mycelium" => BlockFaces::Complex {
                            //     color: [255; 4],
                            //     top: atlas::Block::MyceliumTop,
                            //     sides: atlas::Block::MyceliumSide,
                            //     bottom: atlas::Block::Dirt,
                            // },
                            // "dirt_path" => BlockFaces::Complex {
                            //     color: [255; 4],
                            //     top: atlas::Block::DirtPathTop,
                            //     sides: atlas::Block::DirtPathSide,
                            //     bottom: atlas::Block::Dirt,
                            // },
                            // "honey_block" => BlockFaces::AllSame(atlas::Block::HoneyBlockSide),
                            // "scaffolding" => BlockFaces::Complex {
                            //     color: [255; 4],
                            //     bottom: atlas::Block::ScaffoldingBottom,
                            //     top: atlas::Block::ScaffoldingTop,
                            //     sides: atlas::Block::ScaffoldingSide,
                            // },
                            _ => {
                                *textures.entry(n).or_default() += 1;
                                BlockFaces::AllSame(atlas::Block::Debug)
                            }
                        },
                    };
                    blocks.set(x, y, z, t);
                    // blocks.insert((x, z), )
                    // blocks[x as usize][y as usize][z as usize] = Some(
                    //     atlas::Block::from_name(&name).unwrap_or(atlas::Block::DiamondBlock) as u32,
                    // );
                }
            }
            print!("\r{:.02}%", (x as f32 / SIZE!() as f32) * 100.0);
        }
        println!("");

        for (texture, count) in textures {
            println!("missing texture x{count}: {texture}");
        }

        Self::make_faces(&blocks)
    }

    #[cfg(not(feature = "hermit"))]
    pub fn new() -> Self {
        let mut blocks = Blocks::new();

        fn height(x: i32, y: i32) -> i32 {
            (((x as f32 + y as f32) / 10.).cos() * x as f32 + SIZE!() as f32 + 10.0) as i32
        }

        for x in -SIZE!()..SIZE!() {
            for z in -SIZE!()..SIZE!() {
                for y in 0..height(x, z) {
                    blocks.set(x, y, z, BlockFaces::AllSame(atlas::Block::Dirt));
                }
            }
        }

        Self::make_faces(&blocks)
    }

    fn make_faces(blocks: &Blocks) -> Self {
        let mut faces = Vec::new();

        for x in -SIZE!()..SIZE!() {
            for z in -SIZE!()..SIZE!() {
                for y in 0..320 {
                    let Some(tex_index) = blocks.get(x, y, z) else {
                        continue;
                    };

                    if x == 0 || blocks.get(x - 1, y, z).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 0],
                            size: [1i32; 3],
                            tex_index: tex_index.side(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                    if x == SIZE!() - 1 || blocks.get(x + 1, y, z).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 1],
                            size: [1i32; 3],
                            tex_index: tex_index.side(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                    if y == 0 || blocks.get(x, y - 1, z).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 2],
                            size: [1i32; 3],
                            tex_index: tex_index.bottom(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                    if y == 319 || blocks.get(x, y + 1, z).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 3],
                            size: [1i32; 3],
                            tex_index: tex_index.top(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                    if z == 0 || blocks.get(x, y, z - 1).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 4],
                            size: [1i32; 3],
                            tex_index: tex_index.side(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                    if z == SIZE!() - 1 || blocks.get(x, y, z + 1).is_none() {
                        faces.push(Face {
                            position: [x, y, z, 5],
                            size: [1i32; 3],
                            tex_index: tex_index.side(),
                            color_multiplier: tex_index.color(),
                        });
                    }
                }
            }
        }

        Self { faces }
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
    pub color_multiplier: u32,
}

impl Face {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Sint32x4, // position
        1 => Sint32x3, // size
        2 => Uint32,   // tex_index
        3 => Uint32,  // rgb color multiplier + 6 bits for which sides are affected (with 2 unused bits just before)
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
