use instant::Instant;
use std::{iter, sync::Arc};
use wgpu::util::DeviceExt;
use wgpu_text::{
    glyph_brush::{Section as TextSection, Text},
    BrushBuilder, TextBrush,
};
use winit::{
    event::{ElementState, MouseButton},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::Window,
};

use crate::engine::{
    atlas::Atlas,
    cam::{Camera, CameraController},
    pipe::Pipeline,
    texture::Texture,
    world::{Face, World},
};
#[cfg(feature = "hot-reload")]
use crate::{engine::watcher::Watcher, SHADER_PATH};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_texture: Texture,
    instance_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera: Camera,
    camera_controller: CameraController,
    pipeline: Pipeline,
    world: World,
    atlas: Atlas,
    counting_renders_since: Instant,
    renders_fps: u64,
    is_surface_configured: bool,
    pub window: Arc<Window>,
    brush: TextBrush<wgpu_text::glyph_brush::ab_glyph::FontRef<'static>>,
    debug_text: String,

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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: if surface_caps
                .present_modes
                .contains(&wgpu::PresentMode::Fifo)
            {
                // VSync
                wgpu::PresentMode::Fifo
            } else {
                surface_caps.present_modes[0]
            },
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

        let world = World::new();

        let camera = Camera::new(config.width, config.height);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(camera.build_view_projection_matrix().as_slice()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        let camera_controller = CameraController::new();

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&world.faces()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        #[cfg(target_arch = "wasm32")]
        let atlas = Atlas::new(&device, &queue).await;
        #[cfg(not(target_arch = "wasm32"))]
        let atlas = Atlas::new(&device, &queue);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&atlas.bind_group_layout),
                    Some(&camera_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let pipeline = Pipeline::new(
            &device,
            "Render Pipeline",
            render_pipeline_layout.clone(),
            Face::layout(),
            config.format,
            Some(Texture::DEPTH_FORMAT),
        );

        #[cfg(feature = "hot-reload")]
        let watcher = Watcher::new(&[SHADER_PATH!()]).unwrap();

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
            camera_bind_group,
            camera_buffer,
            camera_controller,
            instance_buffer,
            pipeline,
            world,
            atlas,
            is_surface_configured: false,
            window,
            renders_fps: 0,
            counting_renders_since: instant::Instant::now(),
            brush,
            debug_text: String::new(),
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
            self.brush
                .resize_view(width as f32, height as f32, &self.queue);

            self.surface.configure(&self.device, &self.config);

            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, "depth_texture");

            self.is_surface_configured = true;
            self.window.request_redraw();
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
            self.pipeline.reload_shader(&self.device);
            self.watcher.take_modified_files();
        }
        self.camera_controller.update_camera(&mut self.camera, dt);

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&self.camera.build_view_projection_matrix().as_slice()),
        );
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
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.draw(0..4, 0..self.world.faces().len() as u32);
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
