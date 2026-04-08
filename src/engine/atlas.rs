use rustc_hash::FxHashSet;

generate_enum::generate_enum_from_files!("Block", "assets/textures/block");

pub struct Atlas {
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl Atlas {
    #[cfg(target_arch = "wasm32")]
    pub async fn get_block_image(block: Block) -> image::DynamicImage {
        use wasm_bindgen_futures::JsFuture;
        use web_sys::wasm_bindgen::JsCast as _;
        use web_sys::Response;

        let url = format!("./assets/textures/block/{}", block.name());
        let resp_value = JsFuture::from(web_sys::window().unwrap().fetch_with_str(&url))
            .await
            .expect("fetch failed");
        let resp: Response = resp_value.dyn_into().unwrap();

        let buffer = JsFuture::from(resp.array_buffer().unwrap())
            .await
            .expect("array_buffer failed");
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();

        image::load_from_memory(&bytes).expect("failed to load block image")
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        // Charger toutes les textures
        // let textures_bytes = Block::ALL
        //     .iter()
        //     .copied()
        //     .map(Self::get_block_image) // fn(Block) -> DynamicImage
        //     .map(image::DynamicImage::into_rgba8) // fn(DynamicImage) -> RgbaImage
        //     .collect::<Vec<_>>();

        let mut sizes = FxHashSet::default();

        let mut textures_bytes = Vec::new();
        use futures::future::join_all;
        for img in join_all(Block::ALL.iter().copied().map(Self::get_block_image))
            .await
            .into_iter()
            .map(image::DynamicImage::into_rgba8)
        {
            sizes.insert(img.dimensions());
            textures_bytes.push(img);
        }
        for (width, height) in &sizes {
            log::info!("Texture size: {width}x{height}");
        }
        let tile_size = sizes.iter().max_by_key(|(w, _h)| w).unwrap().0;

        Self::create_texture(device, queue, tile_size, textures_bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut sizes = FxHashSet::default();

        let mut textures_bytes = Vec::new();
        for img in Block::ALL
            .iter()
            .copied()
            .map(|block| image::open(block.path()).unwrap())
            .map(image::DynamicImage::into_rgba8)
        {
            sizes.insert(img.dimensions());
            textures_bytes.push(img);
        }
        for (width, height) in &sizes {
            log::info!("Texture size: {width}x{height}");
        }
        let tile_size = sizes.iter().max_by_key(|(w, _h)| w).unwrap().0;

        Self::create_texture(device, queue, tile_size, textures_bytes)
    }

    fn create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        tile_size: u32,
        textures_bytes: Vec<image::RgbaImage>,
    ) -> Self {
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
