#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CubeFace {
    pub position: [i32; 4],
}

impl CubeFace {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Sint32x4];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<CubeFace>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

const fn _srgb_to_linear_approx(c: f32) -> f32 {
    // polynôme degré 4 pour [0.0,1.0], erreur max <0.001
    c * c * (0.305306 + c * (0.682171 + c * 0.012523))
}

macro_rules! _color {
    [$r:expr, $g:expr, $b:expr] => {
        [
            srgb_to_linear_approx($r as f32),
            srgb_to_linear_approx($g as f32),
            srgb_to_linear_approx($b as f32),
        ]
    };
}
