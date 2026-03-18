generate_enum::generate_enum_from_files!("Block", "assets/textures/block");

pub struct Atlas {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Atlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Charger toutes les textures
        let mut textures_bytes = vec![];
        for block in Block::ALL {
            let img = image::open(block.path()).unwrap().to_rgba8();
            textures_bytes.push(img);
        }

        let tile_size = textures_bytes[0].width();
        let layer_count = Block::ALL.len() as u32;

        let size = wgpu::Extent3d {
            width: tile_size,
            height: tile_size,
            depth_or_array_layers: layer_count,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("block_array"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[wgpu::TextureFormat::Rgba8UnormSrgb],
        });

        for (i, tex) in textures_bytes.iter().enumerate() {
            let bytes = tex.as_raw();

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32, // <-- layer index
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * tile_size),
                    rows_per_image: Some(tile_size),
                },
                wgpu::Extent3d {
                    width: tile_size,
                    height: tile_size,
                    depth_or_array_layers: 1,
                },
            );
        }

        // View
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        // Sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // Layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("array_texture_layout"),
        });

        // Bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("array_texture_bind_group"),
        });

        Self {
            bind_group_layout,
            bind_group,
        }
    }
}
