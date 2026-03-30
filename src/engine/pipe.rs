use crate::SHADER_PATH;
use derive_more::Deref;

#[derive(Deref)]
pub struct Pipeline {
    #[deref]
    render_pipeline: wgpu::RenderPipeline,
    #[cfg(feature = "hot-reload")]
    pipeline_info: PipelineInfo,
}

pub struct PipelineInfo {
    pub label: String,
    pub layout: wgpu::PipelineLayout,
    pub vertex_layout: wgpu::VertexBufferLayout<'static>,
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
}

impl Pipeline {
    fn build_pipeline(
        device: &wgpu::Device,
        info: &PipelineInfo,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        #[cfg(feature = "hot-reload")]
        let shader = {
            use wgpu::naga::{
                front::wgsl,
                valid::{Capabilities, ValidationFlags, Validator},
            };
            let source = std::fs::read_to_string(SHADER_PATH!()).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read shader {} ({}): {e}",
                    info.label,
                    SHADER_PATH!()
                )
            })?;

            // validate that shader compiles
            let module = wgsl::parse_str(&source)?;
            let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
            validator.validate(&module)?;

            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&info.label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        #[cfg(not(feature = "hot-reload"))]
        let shader = {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&info.label),
                source: wgpu::ShaderSource::Wgsl(include_str!(SHADER_PATH!()).into()),
            })
        };

        let vertex_layouts = &[info.vertex_layout.clone()];

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&info.label),
                layout: Some(&info.layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: vertex_layouts,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: info.color_format,
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
                depth_stencil: info.depth_format.map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    #[cfg(feature = "msaa")]
                    count: 4,
                    #[cfg(not(feature = "msaa"))]
                    count: 1,
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
        label: impl Into<String>,
        layout: wgpu::PipelineLayout,
        vertex_layout: wgpu::VertexBufferLayout<'static>,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        let label = label.into();

        let pipeline_info = PipelineInfo {
            label,
            layout,
            vertex_layout,
            color_format,
            depth_format,
        };

        let render_pipeline =
            Self::build_pipeline(device, &pipeline_info).expect("Failed to create render pipeline");

        Self {
            render_pipeline,
            #[cfg(feature = "hot-reload")]
            pipeline_info,
        }
    }

    #[cfg(feature = "hot-reload")]
    pub fn reload_shader(&mut self, device: &wgpu::Device) {
        match Self::build_pipeline(device, &self.pipeline_info) {
            Ok(render_pipeline) => self.render_pipeline = render_pipeline,
            Err(e) => {
                eprintln!("Failed to reload shader {}: {e}", self.pipeline_info.label);
            }
        }
    }
}
