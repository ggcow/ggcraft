use crate::{
    engine::{
        pipe::{FragmentStateTemplate, Pipeline, RenderPipelineTemplate, VertexStateTemplate},
        uniform::ScreenUniform,
    },
    shader_config, shader_path,
};

pub struct Cross {
    screen_uniform_group: wgpu::BindGroup,
    pub pipeline: Pipeline,
}

impl Cross {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        screen_uniform: &ScreenUniform,
    ) -> anyhow::Result<Self> {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&screen_uniform.layout)],
                immediate_size: 0,
            });
        let pipeline = Pipeline::new(
            device,
            RenderPipelineTemplate {
                label: Some("Cross Render Pipeline"),
                layout: Some(render_pipeline_layout),
                vertex: VertexStateTemplate {
                    entry_point: Some("vs_main"),
                    buffers: vec![],
                },
                fragment: FragmentStateTemplate {
                    entry_point: Some("fs_main"),
                    targets: vec![Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::Zero,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            },
            shader_config!("cross.wgsl"),
        );

        Ok(Cross {
            screen_uniform_group: screen_uniform.bind_group.clone(),
            pipeline,
        })
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.screen_uniform_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
