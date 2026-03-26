use std::sync::Arc;
use wgpu::{Limits, util::DeviceExt as _};
use wgpu_text::{
    BrushBuilder, TextBrush,
    glyph_brush::{Section as TextSection, Text},
};
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::engine::{
    atlas,
    cam::{Camera, CameraController},
    pipe, texture, watcher, world,
};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    pub window: Arc<Window>,
    pipeline: pipe::Pipeline,
    atlas: atlas::Atlas,
    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    instance_buffer: wgpu::Buffer,
    world: world::World,
    depth_texture: texture::Texture,
    watcher: watcher::Watcher,
    msaa_texture: texture::Texture,
    brush: TextBrush<wgpu_text::glyph_brush::ab_glyph::FontRef<'static>>,
    counting_renders_since: std::time::Instant,
    renders_fps: u64,
    debug_text: String,
}

impl State {
    // We don't need this to be async right now,
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: Limits {
                    max_texture_array_layers: 2048,
                    ..Default::default()
                },
                ..Default::default()
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let msaa_texture = texture::Texture::create_msaa_texture(&device, &config, "msaa_texture");

        let world = world::World::new();

        let watcher_handle = watcher::Watcher::new(&["src/shaders/block.wgsl"]).unwrap();

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

        let atlas = atlas::Atlas::new(&device, &queue);
        let texture_bind_group_layout = &atlas.bind_group_layout;
        let diffuse_bind_group = &atlas.bind_group;

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                immediate_size: 0,
            });

        let pipeline = pipe::Pipeline::new(
            &device,
            "src/shaders/block.wgsl",
            "Render Pipeline",
            render_pipeline_layout.clone(),
            world::Face::layout(),
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
        );

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
            is_surface_configured: false,
            window,
            pipeline,
            atlas,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            instance_buffer,
            world,
            depth_texture,
            watcher: watcher_handle,
            msaa_texture,
            brush,
            debug_text: String::new(),
            counting_renders_since: std::time::Instant::now(),
            renders_fps: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.brush
                .resize_view(width as f32, height as f32, &self.queue);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
            // Update camera aspect ratio
            self.camera.aspect = self.config.width as f32 / self.config.height as f32;
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.msaa_texture =
                texture::Texture::create_msaa_texture(&self.device, &self.config, "msaa_texture");

            self.window.request_redraw();
        }
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::KeyF, true) => {
                if self.window.fullscreen().is_some() {
                    self.window.set_fullscreen(None);
                } else {
                    self.window
                        .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                }
            }
            _ => self.camera_controller.handle_key(code, is_pressed),
        }
    }

    pub fn handle_mouse_buttons(&mut self, button: winit::event::MouseButton, is_pressed: bool) {
        match (button, is_pressed) {
            (winit::event::MouseButton::Left, true) => {}
            (winit::event::MouseButton::Left, false) => {}
            _ => {}
        }
    }

    pub fn handle_mouse_move(&mut self, dx: f32, dy: f32) {
        self.camera_controller.handle_mouse_move(dx, dy);
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.renders_fps += 1;
        if self.counting_renders_since.elapsed().as_secs() >= 1 {
            self.debug_text = format!("FPS: {}\npos: {:?}", self.renders_fps, self.camera.eye);
            self.renders_fps = 0;
            self.counting_renders_since += std::time::Duration::from_secs(1);
        }
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

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();
        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let output_view = output
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
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.msaa_texture.view,
                        resolve_target: Some(&output_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
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
                    view: &output_view,
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

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
