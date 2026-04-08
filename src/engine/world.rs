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
pub enum MaybeColored<T> {
    Colored(T, [u8; 4]),
    NonColored(T),
}

impl<T> MaybeColored<T> {
    pub fn color(&self) -> u32 {
        match self {
            MaybeColored::Colored(_, c) => {
                let [r, g, b, a] = *c;
                ((r as u32) << 24) + ((g as u32) << 16) + ((b as u32) << 8) + (a as u32)
            }
            MaybeColored::NonColored(_) => 0xff_ff_ff_ff,
        }
    }
    pub fn value(&self) -> &T {
        match self {
            MaybeColored::Colored(t, _) => t,
            MaybeColored::NonColored(t) => t,
        }
    }
    pub fn map<V>(&self, f: impl FnOnce(&T) -> V) -> MaybeColored<V> {
        match self {
            Self::Colored(t, color) => MaybeColored::Colored(f(t), *color),
            Self::NonColored(t) => MaybeColored::NonColored(f(t)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BlockFaces {
    AllSame(MaybeColored<atlas::Block>),
    Complex {
        top: MaybeColored<atlas::Block>,
        sides: MaybeColored<atlas::Block>,
        bottom: MaybeColored<atlas::Block>,
        size: [i32; 4],
    },
}

impl BlockFaces {
    pub fn is_solid(&self) -> bool {
        match self {
            Self::AllSame(_) => true,
            Self::Complex { size, .. } if size == &[2, 2, 2, 0] => true,
            _ => false,
        }
    }
    pub fn new(faces: atlas::Block) -> Self {
        Self::AllSame(MaybeColored::NonColored(faces))
    }
    pub fn new_colored(faces: atlas::Block, color: [u8; 4]) -> Self {
        Self::Complex {
            top: MaybeColored::Colored(faces, color),
            sides: MaybeColored::Colored(faces, color),
            bottom: MaybeColored::Colored(faces, color),
            size: [2, 2, 2, 0],
        }
    }
    pub fn top_side_bottom(top: atlas::Block, side: atlas::Block, bottom: atlas::Block) -> Self {
        Self::Complex {
            top: MaybeColored::NonColored(top),
            sides: MaybeColored::NonColored(side),
            bottom: MaybeColored::NonColored(bottom),
            size: [2, 2, 2, 0],
        }
    }
}

impl BlockFaces {
    fn top(&self) -> MaybeColored<u32> {
        match self {
            BlockFaces::AllSame(x) => x.map(|v| *v as u32),
            BlockFaces::Complex { top, .. } => top.map(|v| *v as u32),
        }
    }
    fn side(&self) -> MaybeColored<u32> {
        match self {
            BlockFaces::AllSame(x) => x.map(|v| *v as u32),
            BlockFaces::Complex { sides, .. } => sides.map(|v| *v as u32),
        }
    }
    fn bottom(&self) -> MaybeColored<u32> {
        match self {
            BlockFaces::AllSame(x) => x.map(|v| *v as u32),
            BlockFaces::Complex { bottom, .. } => bottom.map(|v| *v as u32),
        }
    }
    fn size(&self) -> [i32; 4] {
        match self {
            BlockFaces::AllSame(_) => [2, 2, 2, 0],
            BlockFaces::Complex { size, .. } => *size,
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

        // let mut total_blocks = 0;

        let load = &(-SIZE!()..SIZE!(), -SIZE!()..SIZE!());
        // let load = &(-1000..-200, -600..-80);

        #[cfg(not(target_arch = "wasm32"))]
        let i = indicatif::ProgressBar::new((load.0.len() * load.1.len()) as u64 * 320);
        for (x, z) in itertools::iproduct!(load.0.clone(), load.1.clone()) {
            for y in 0..320 {
                #[cfg(not(target_arch = "wasm32"))]
                i.inc(1);

                // TODO: use other block info to get top slabs
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
                // total_blocks += 1;
                let t = match atlas::Block::from_stem(&n) {
                    Some(face) => match face {
                        atlas::Block::OakLeaves
                        | atlas::Block::BirchLeaves
                        | atlas::Block::FloweringAzaleaLeaves
                        | atlas::Block::AcaciaLeaves
                        | atlas::Block::AzaleaLeaves
                        | atlas::Block::CherryLeaves
                        | atlas::Block::JungleLeaves
                        | atlas::Block::SpruceLeaves
                        | atlas::Block::MangroveLeaves => {
                            BlockFaces::new_colored(face, [0, 255, 0, 255])
                        }
                        face => BlockFaces::new(face),
                    },

                    None => match n.as_str() {
                        "infested_stone" => BlockFaces::new(atlas::Block::Stone),
                        // "smooth_quartz" => BlockFaces::top_side_bottom(
                        //     atlas::Block::QuartzBlockTop,
                        //     atlas::Block::QuartzBlockSide,
                        //     atlas::Block::QuartzBlockBottom,
                        // ),
                        // "snow_block" => BlockFaces::new(atlas::Block::Snow),
                        // "grass_block" => BlockFaces::Complex {
                        //     size: [2; 3],
                        //     top: MaybeColored::Colored(
                        //         atlas::Block::GrassBlockTop,
                        //         [0, 255, 0, 255],
                        //     ),
                        //     sides: MaybeColored::Colored(atlas::Block::GrassBlockSide, [255; 4]),
                        //     bottom: MaybeColored::Colored(atlas::Block::Dirt, [255; 4]),
                        // },
                        // "mycelium" => BlockFaces::top_side_bottom(
                        //     atlas::Block::MyceliumTop,
                        //     atlas::Block::MyceliumSide,
                        //     atlas::Block::Dirt,
                        // ),
                        // "dirt_path" => BlockFaces::top_side_bottom(
                        //     atlas::Block::DirtPathTop,
                        //     atlas::Block::DirtPathSide,
                        //     atlas::Block::Dirt,
                        // ),
                        // "honey_block" => BlockFaces::top_side_bottom(
                        //     atlas::Block::HoneyBlockTop,
                        //     atlas::Block::HoneyBlockSide,
                        //     atlas::Block::HoneyBlockBottom,
                        // ),
                        // "scaffolding" => BlockFaces::top_side_bottom(
                        //     atlas::Block::ScaffoldingTop,
                        //     atlas::Block::ScaffoldingSide,
                        //     atlas::Block::ScaffoldingBottom,
                        // ),
                        _ => {
                            if let Some(block) =
                                n.strip_suffix("_slab").and_then(atlas::Block::from_stem)
                            {
                                BlockFaces::Complex {
                                    bottom: MaybeColored::NonColored(block),
                                    sides: MaybeColored::NonColored(block),
                                    top: MaybeColored::NonColored(block),
                                    size: [2, 1, 2, 0],
                                }
                            } else {
                                *textures.entry(n).or_default() += 1;
                                BlockFaces::new(atlas::Block::Debug)
                            }
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

        Self::make_faces(&blocks)
    }
    // i.finish();
    // for (texture, count) in textures {
    //     log::info!("missing texture x{count}: {texture}");
    // }
    // log::info!("loaded {total_blocks} blocks");

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

        for (xz, column) in &blocks.blocks {
            for (y, block) in column.iter().enumerate() {
                let Some(tex_index) = block else {
                    continue;
                };

                let y = y as i32;
                let (x, z) = *xz;

                if x == 0 || blocks.get(x - 1, y, z).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 0],
                        size: tex_index.size(),
                        tex_index: *tex_index.side().value(),
                        color_multiplier: tex_index.side().color(),
                    });
                }
                if x == SIZE!() - 1 || blocks.get(x + 1, y, z).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 1],
                        size: tex_index.size(),
                        tex_index: *tex_index.side().value(),
                        color_multiplier: tex_index.side().color(),
                    });
                }
                if y == 0 || blocks.get(x, y - 1, z).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 2],
                        size: tex_index.size(),
                        tex_index: *tex_index.bottom().value(),
                        color_multiplier: tex_index.bottom().color(),
                    });
                }
                if y == 319 || blocks.get(x, y + 1, z).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 3],
                        size: tex_index.size(),
                        tex_index: *tex_index.top().value(),
                        color_multiplier: tex_index.top().color(),
                    });
                }
                if z == 0 || blocks.get(x, y, z - 1).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 4],
                        size: tex_index.size(),
                        tex_index: *tex_index.side().value(),
                        color_multiplier: tex_index.side().color(),
                    });
                }
                if z == SIZE!() - 1 || blocks.get(x, y, z + 1).is_none_or(|x| !x.is_solid()) {
                    faces.push(Face {
                        position: [x, y, z, 5],
                        size: tex_index.size(),
                        tex_index: *tex_index.side().value(),
                        color_multiplier: tex_index.side().color(),
                    });
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
    // x,y,z, flags
    /// flags:
    /// 0x1: +0.5x
    /// 0x2: +0.5y
    /// 0x4: +0.5z
    pub size: [i32; 4],
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
