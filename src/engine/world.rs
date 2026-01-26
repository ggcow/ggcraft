const CHUNK_SIZE: usize = 16;

#[derive(Copy, Clone, PartialEq)]
enum BlockType {
    Air = 0,
    Stone = 1,
}

#[derive(Clone)]
pub struct Chunk {
    blocks: [BlockType; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
    position: (i32, i32, i32),
    pub mesh: Vec<Face>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            blocks: [BlockType::Stone; CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE],
            position: (0, 0, 0),
            mesh: Vec::new(),
        }
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        x + CHUNK_SIZE * (y + CHUNK_SIZE * z)
    }

    fn get(&self, x: usize, y: usize, z: usize) -> BlockType {
        self.blocks[Self::index(x, y, z)]
    }

    fn set(&mut self, x: usize, y: usize, z: usize, value: BlockType) {
        self.blocks[Self::index(x, y, z)] = value;
    }

    pub fn greedy_mesh(&self) -> Vec<Face> {
        let mut faces = Vec::new();
        for axis in 0..3 {
            // 0=X, 1=Y, 2=Z
            for dir in 0..2 {
                // 0=Neg, 1=Pos
                faces.extend(self.mesh_axis(axis, dir));
            }
        }
        for face in &faces {
            println!(
                "Face at position {:?} with size {:?}",
                face.position, face.size
            );
        }

        faces
    }

    fn mesh_axis(&self, axis: usize, dir: usize) -> Vec<Face> {
        let mut faces = Vec::new();
        let u_size = CHUNK_SIZE;
        let v_size = CHUNK_SIZE;
        let w_size = CHUNK_SIZE;

        for w in 0..w_size {
            let mut mask = vec![BlockType::Air; u_size * v_size];

            // construire le mask
            for v in 0..v_size {
                for u in 0..u_size {
                    let (x, y, z) = match axis {
                        0 => (w, u, v),
                        1 => (u, w, v),
                        2 => (u, v, w),
                        _ => unreachable!(),
                    };

                    let neighbor = match (axis, dir) {
                        (0, 0) => {
                            if w > 0 {
                                self.get(w - 1, u, v)
                            } else {
                                BlockType::Air
                            }
                        }
                        (0, 1) => {
                            if w + 1 < CHUNK_SIZE {
                                self.get(w + 1, u, v)
                            } else {
                                BlockType::Air
                            }
                        }
                        (1, 0) => {
                            if w > 0 {
                                self.get(u, w - 1, v)
                            } else {
                                BlockType::Air
                            }
                        }
                        (1, 1) => {
                            if w + 1 < CHUNK_SIZE {
                                self.get(u, w + 1, v)
                            } else {
                                BlockType::Air
                            }
                        }
                        (2, 0) => {
                            if w > 0 {
                                self.get(u, v, w - 1)
                            } else {
                                BlockType::Air
                            }
                        }
                        (2, 1) => {
                            if w + 1 < CHUNK_SIZE {
                                self.get(u, v, w + 1)
                            } else {
                                BlockType::Air
                            }
                        }
                        _ => unreachable!(),
                    };

                    let current = self.get(x, y, z);
                    mask[u + v * u_size] = if current != BlockType::Air && current != neighbor {
                        current
                    } else {
                        BlockType::Air
                    };
                }
            }

            // Greedy rectangle
            let mut visited = vec![false; u_size * v_size];
            for v in 0..v_size {
                for u in 0..u_size {
                    let idx = u + v * u_size;
                    if mask[idx] == BlockType::Air || visited[idx] {
                        continue;
                    }

                    let block_type = mask[idx];
                    let mut width = 1;
                    while u + width < u_size
                        && mask[u + width + v * u_size] == block_type
                        && !visited[u + width + v * u_size]
                    {
                        width += 1;
                    }

                    let mut height = 1;
                    'outer: while v + height < v_size {
                        for du in 0..width {
                            if mask[u + du + (v + height) * u_size] != block_type
                                || visited[u + du + (v + height) * u_size]
                            {
                                break 'outer;
                            }
                        }
                        height += 1;
                    }

                    // Marquer visité
                    for dv in 0..height {
                        for du in 0..width {
                            visited[u + du + (v + dv) * u_size] = true;
                        }
                    }

                    // Calculer coords selon axis
                    let (x, y, z) = match axis {
                        0 => (w, u, v),
                        1 => (u, w, v),
                        2 => (u, v, w),
                        _ => unreachable!(),
                    };

                    let direction = match (axis, dir) {
                        (0, 0) => 0, // -X
                        (0, 1) => 1, // +X
                        (1, 0) => 2, // -Y
                        (1, 1) => 3, // +Y
                        (2, 0) => 4, // -Z
                        (2, 1) => 5, // +Z
                        _ => unreachable!(),
                    };

                    faces.push(Face {
                        position: [x as i32, y as i32, z as i32, direction],
                        size: [width as i32, height as i32],
                    });
                }
            }
        }

        faces
    }
}

#[derive(Debug, Clone, Copy)]
enum Axis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy)]
enum DirectionSign {
    Pos,
    Neg,
}

enum Direction {
    NegativeX = 0,
    PositiveX = 1,
    NegativeZ = 2,
    PositiveZ = 3,
    NegativeY = 4,
    PositiveY = 5,
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
