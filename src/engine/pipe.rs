use derive_more::Deref;
use wgpu::*;

#[derive(Deref)]
pub struct Pipeline {
    #[deref]
    render_pipeline: RenderPipeline,
    template: RenderPipelineTemplate,
    pub shader: &'static str,
}

#[derive(Clone)]
pub struct VertexBufferTemplate {
    pub array_stride: BufferAddress,
    pub step_mode: VertexStepMode,
    pub attributes: Vec<VertexAttribute>,
}
#[derive(Clone)]
pub struct VertexStateTemplate {
    pub entry_point: Option<&'static str>,
    pub buffers: Vec<VertexBufferTemplate>,
}

impl<'a> From<&'a VertexBufferTemplate> for VertexBufferLayout<'a> {
    fn from(t: &'a VertexBufferTemplate) -> Self {
        Self {
            array_stride: t.array_stride,
            step_mode: t.step_mode,
            attributes: &t.attributes,
        }
    }
}

impl From<VertexBufferLayout<'_>> for VertexBufferTemplate {
    fn from(t: VertexBufferLayout) -> Self {
        Self {
            array_stride: t.array_stride,
            step_mode: t.step_mode,
            attributes: t.attributes.to_vec(),
        }
    }
}

#[derive(Clone)]
pub struct FragmentStateTemplate {
    pub entry_point: Option<&'static str>,
    pub targets: Vec<Option<ColorTargetState>>,
}

#[derive(Clone)]
pub struct RenderPipelineTemplate {
    pub label: Option<&'static str>,
    pub layout: Option<PipelineLayout>,
    pub vertex: VertexStateTemplate,
    pub fragment: FragmentStateTemplate,
    pub primitive: PrimitiveState,
    pub depth_stencil: Option<DepthStencilState>,
    pub multisample: MultisampleState,
}

#[macro_export]
macro_rules! shader_config {
    ($name:literal) => {{
        #[cfg(feature = "hot-reload")]
        {
            shader_path!($name)
        }
        #[cfg(not(feature = "hot-reload"))]
        {
            include_str!(shader_path!($name))
        }
    }};
}

impl Pipeline {
    fn build_pipeline(
        device: &Device,
        template: &RenderPipelineTemplate,
        shader: &'static str,
    ) -> anyhow::Result<RenderPipeline> {
        use naga::{
            front::wgsl,
            valid::{Capabilities, ValidationFlags, Validator},
        };
        let source = {
            #[cfg(feature = "hot-reload")]
            {
                let source = std::fs::read_to_string(shader)?;

                let module = wgsl::parse_str(&source)
                    .map_err(|e| anyhow::anyhow!(e.emit_to_string_with_path(&source, shader)))?;

                let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
                validator
                    .validate(&module)
                    .map_err(|e| anyhow::anyhow!(e.emit_to_string_with_path(&source, shader)))?;

                source
            }

            #[cfg(not(feature = "hot-reload"))]
            shader
        };

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: template
                .label
                .map(|label| label.to_string() + "_shader")
                .as_deref(),
            source: ShaderSource::Wgsl(source.into()),
        });

        let descriptor = &RenderPipelineDescriptor {
            label: template.label,
            layout: template.layout.as_ref(),
            vertex: VertexState {
                module: &shader,
                entry_point: template.vertex.entry_point,
                buffers: &template
                    .vertex
                    .buffers
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
                compilation_options: Default::default(),
            },
            primitive: template.primitive,
            depth_stencil: template.depth_stencil.clone(),
            multisample: template.multisample,
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: template.fragment.entry_point,
                targets: &template.fragment.targets,
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        };

        Ok(device.create_render_pipeline(&descriptor))
    }

    pub fn new(device: &Device, template: RenderPipelineTemplate, shader: &'static str) -> Self {
        let render_pipeline = Self::build_pipeline(device, &template, &shader)
            .expect("Failed to create render pipeline");

        Self {
            render_pipeline,
            template,
            shader,
        }
    }

    #[cfg(feature = "hot-reload")]
    pub fn reload_shader(&mut self, device: &Device) {
        match Self::build_pipeline(device, &self.template, &self.shader) {
            Ok(render_pipeline) => self.render_pipeline = render_pipeline,
            Err(e) => log::error!("{e}"),
        }
    }
}
