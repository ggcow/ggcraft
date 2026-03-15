use derive_more::Deref;
use std::path::{Path, PathBuf};
use wgpu::naga::{
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

#[derive(Deref)]
pub struct Pipeline {
    #[deref]
    render_pipeline: wgpu::RenderPipeline,

    shader_path: PathBuf,
    label: String,
    layout: wgpu::PipelineLayout,
    vertex_layout: wgpu::VertexBufferLayout<'static>,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
}

impl Pipeline {
    fn build_pipeline(
        device: &wgpu::Device,
        shader_path: &Path,
        label: &str,
        layout: &wgpu::PipelineLayout,
        vertex_layout: wgpu::VertexBufferLayout<'static>,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        let source = std::fs::read_to_string(shader_path).expect("Failed to read shader file");

        // validate that shader compiles
        let module = wgsl::parse_str(&source)?;
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
        validator.validate(&module)?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let vertex_layouts = [vertex_layout];

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 4,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            }),
        )
    }

    pub fn new(
        device: &wgpu::Device,
        shader_path: impl Into<PathBuf>,
        label: impl Into<String>,
        layout: wgpu::PipelineLayout,
        vertex_layout: wgpu::VertexBufferLayout<'static>,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let shader_path = shader_path.into();
        let label = label.into();

        let render_pipeline = Self::build_pipeline(
            device,
            &shader_path,
            &label,
            &layout,
            vertex_layout.clone(),
            color_format,
            depth_format,
        )
        .expect("Failed to create render pipeline");

        Self {
            render_pipeline,
            shader_path,
            label,
            layout,
            vertex_layout,
            color_format,
            depth_format,
        }
    }

    pub fn reload_shader(&mut self, device: &wgpu::Device) {
        match Self::build_pipeline(
            device,
            &self.shader_path,
            &self.label,
            &self.layout,
            self.vertex_layout.clone(),
            self.color_format,
            self.depth_format,
        ) {
            Ok(render_pipeline) => self.render_pipeline = render_pipeline,
            Err(e) => {
                eprintln!(
                    "Failed to reload shader {}: {}",
                    self.shader_path.display(),
                    e
                );
            }
        }
    }
}
