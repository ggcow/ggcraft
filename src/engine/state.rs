#[cfg(feature = "hot-reload")]
use crate::engine::watcher::Watcher;
use crate::{
    engine::{
        atlas::Atlas,
        cam::{Camera, CameraController, CameraUniform},
        cross::Cross,
        pipe::{FragmentStateTemplate, Pipeline, RenderPipelineTemplate, VertexStateTemplate},
        texture::Texture,
        uniform::{
            HighlightedBlock, HighlightedBlockUniform, ScreenSize, ScreenUniform, UniformData,
        },
        world::{Face, World},
    },
    shader_config, shader_path,
};
use instant::Instant;
use std::{iter, sync::Arc};
use wgpu::{CompositeAlphaMode, util::DeviceExt};
use wgpu_text::{
    BrushBuilder, TextBrush,
    glyph_brush::{Section as TextSection, Text},
};
use winit::{
    event::{ElementState, MouseButton},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::Window,
};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_texture: Texture,
    instance_buffer: wgpu::Buffer,
    camera: Camera,
    camera_controller: CameraController,
    camera_uniform: CameraUniform,
    screen_uniform: ScreenUniform,
    pipeline: Pipeline,
    cross: Cross,
    world: World,
    atlas: Atlas,
    counting_renders_since: Instant,
    renders_fps: u64,
    is_surface_configured: bool,
    pub window: Arc<Window>,
    brush: TextBrush<wgpu_text::glyph_brush::ab_glyph::FontRef<'static>>,
    debug_text: String,
    highlighted_block_uniform: HighlightedBlockUniform,

    #[cfg(feature = "hot-reload")]
    watcher: Watcher,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let info = adapter.get_info();
        let friendly_name = if !info.name.is_empty() {
            info.name.clone()
        } else {
            format!("{:?} via {:?}", info.device_type, info.backend)
        };
        log::info!("Using adapter: {friendly_name}");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                #[cfg(target_arch = "wasm32")]
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                #[cfg(not(target_arch = "wasm32"))]
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                // required_limits: Limits {
                //     max_texture_array_layers: 2048,
                //     max_buffer_size: 2147483647,
                //     ..Default::default()
                // },
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let alpha_mode = if surface_caps
            .alpha_modes
            .contains(&CompositeAlphaMode::PreMultiplied)
        {
            CompositeAlphaMode::PreMultiplied
        } else if surface_caps
            .alpha_modes
            .contains(&CompositeAlphaMode::PostMultiplied)
        {
            CompositeAlphaMode::PostMultiplied
        } else {
            CompositeAlphaMode::Opaque
        };
        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Fifo)
        {
            // VSync
            wgpu::PresentMode::Fifo
        } else {
            surface_caps.present_modes[0]
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let camera = Camera::new(config.width, config.height);
        let camera_controller = CameraController::new();
        let camera_uniform = camera.create_uniform(&device, 0);
        let world = World::new();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&world.faces()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        #[cfg(target_arch = "wasm32")]
        let atlas = Atlas::new(&device, &queue).await;
        #[cfg(not(target_arch = "wasm32"))]
        let atlas = Atlas::new(&device, &queue);

        #[cfg(feature = "hot-reload")]
        let watcher =
            Watcher::new(&[shader_path!("block.wgsl"), shader_path!("cross.wgsl")]).unwrap();

        let highlighted_block_uniform = HighlightedBlock::from((0, 0, 0, -1)).create_uniform(
            &device,
            0,
            Some("highlighted_block_uniform"),
        );

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&atlas.bind_group_layout),
                    Some(&camera_uniform.layout),
                    Some(&highlighted_block_uniform.layout),
                ],
                immediate_size: 0,
            });

        let pipeline = Pipeline::new(
            &device,
            RenderPipelineTemplate {
                label: Some("Render Pipeline"),
                layout: Some(render_pipeline_layout),
                vertex: VertexStateTemplate {
                    entry_point: Some("vs_main"),
                    buffers: vec![Face::layout().into()],
                },
                fragment: FragmentStateTemplate {
                    entry_point: Some("fs_main"),
                    targets: vec![Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                },
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: Texture::DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            },
            shader_config!("block.wgsl"),
        );

        let screen_uniform = ScreenSize(config.width, config.height).create_uniform(
            &device,
            0,
            Some("screen_size_uniform"),
        );

        let cross = Cross::new(&device, &config, &screen_uniform)?;

        let brush =
            BrushBuilder::using_font_bytes(include_bytes!("../../assets/fonts/TTT-Regular.otf"))
                .unwrap()
                // .initial_cache_size((16_384, 16_384)) // use this to avoid resizing cache texture
                .build(&device, config.width, config.height, config.format);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            depth_texture,
            camera,
            camera_controller,
            camera_uniform,
            screen_uniform,
            instance_buffer,
            pipeline,
            cross,
            world,
            atlas,
            is_surface_configured: false,
            window,
            renders_fps: 0,
            counting_renders_since: instant::Instant::now(),
            brush,
            debug_text: String::new(),
            highlighted_block_uniform,
            #[cfg(feature = "hot-reload")]
            watcher,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let max_size = self.device.limits().max_texture_dimension_2d;

            self.config.width = width.min(max_size);
            self.config.height = height.min(max_size);

            self.camera.resize(width, height);
            self.screen_uniform
                .update(&self.queue, &ScreenSize(width, height));
            self.brush
                .resize_view(width as f32, height as f32, &self.queue);

            self.surface.configure(&self.device, &self.config);

            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

            self.is_surface_configured = true;
            self.window.request_redraw();
            log::info!("Resized to {}x{}", width, height);
        }
    }

    pub fn update(&mut self, dt: instant::Duration) {
        self.renders_fps += 1;
        if self.counting_renders_since.elapsed().as_secs() >= 1 {
            self.debug_text = format!("FPS: {}\npos: {:?}", self.renders_fps, self.camera.eye);
            self.renders_fps = 0;
            self.counting_renders_since = instant::Instant::now();
        }
        #[cfg(feature = "hot-reload")]
        if self.watcher.is_dirty() {
            log::info!("Reloading shaders...");
            for path in self.watcher.take_modified_files() {
                for pipeline in [&mut self.pipeline, &mut self.cross.pipeline] {
                    if *path == *pipeline.shader {
                        log::info!(
                            "Reloading shader '{}'",
                            if let Some(name) = path.file_name() {
                                name.to_string_lossy()
                            } else {
                                path.to_string_lossy()
                            }
                        );
                        pipeline.reload_shader(&self.device);
                    }
                }
            }
        }
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update(&self.queue, &(&self.camera).into());
        if let Some((position, block)) = self.raycast() {
            self.debug_text = format!(
                "FPS: {}\npos: {:?}\nBlock: {}",
                self.renders_fps, self.camera.eye, block
            );
            self.highlighted_block_uniform.update(
                &self.queue,
                &((position.0, position.1, position.2, 0).into()),
            );
        } else {
            self.highlighted_block_uniform
                .update(&self.queue, &((0, 0, 0, -1).into()));
        }
    }

    fn raycast(&self) -> Option<((i32, i32, i32), crate::engine::atlas::Block)> {
        let mut pos = self.camera.eye;

        let step = self.camera.forward().normalize() * 0.1;
        let mut traveled = 0.0;
        let max_distance = 100.0;

        while traveled < max_distance {
            let position = (
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            if let Some(block) = self.world.get(position.0, position.1, position.2) {
                return Some((position, block));
            }
            pos += step;
            traveled += step.norm();
        }

        None
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            render_pass.set_bind_group(0, &self.atlas.bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_uniform.bind_group, &[]);
            render_pass.set_bind_group(2, &self.highlighted_block_uniform.bind_group, &[]);
            render_pass.draw(0..4, 0..self.world.faces().len() as u32);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Crosshair Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            self.cross.render(&mut render_pass);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            let section = TextSection::default().add_text(
                Text::new(&self.debug_text)
                    .with_color([1., 0., 0., 1.])
                    .with_scale(42.),
            );

            self.brush
                .queue(&self.device, &self.queue, &[section])
                .unwrap();

            self.brush.draw(&mut render_pass);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        self.camera_controller.handle_mouse_move(dx, dy);
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, state: ElementState) {
        match (code, state == ElementState::Pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::KeyF, true) => {
                if self.window.fullscreen().is_some() {
                    self.window.set_fullscreen(None);
                } else {
                    self.window
                        .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                }
            }
            _ => self.camera_controller.handle_key(code, state),
        }
    }

    #[allow(unused)]
    pub fn handle_mouse_buttons(&mut self, button: winit::event::MouseButton, state: ElementState) {
        match (button, state == ElementState::Pressed) {
            (MouseButton::Left, true) => {
                let _ = self
                    .window
                    .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                    .or_else(|_| {
                        self.window
                            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                    });

                self.window.set_cursor_visible(false);
            }
            (MouseButton::Right, true) => {
                self.window.set_cursor_visible(true);
                self.window
                    .set_cursor_grab(winit::window::CursorGrabMode::None);
            }
            _ => {}
        }
    }
}
