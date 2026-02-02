use tint::Srgb;

/// Matches `palette` on [`colorgrad`] preset functions.
pub fn parse_palette(palette: &str) -> Vec<Srgb> {
    macro_rules! match_colorgrad {
        (palette, $($arm:ident,)*) => {
            match palette {
                $(stringify!($arm) => generate_gradient(&colorgrad::preset::$arm()),)*
                _ => {
                    println!("Unknown palette: {}", palette);
                    std::process::exit(1);
                }
            }
        };
    }

    match_colorgrad!(
        palette,
        blues,
        br_bg,
        bu_gn,
        bu_pu,
        cividis,
        cool,
        cubehelix_default,
        gn_bu,
        greens,
        greys,
        inferno,
        magma,
        or_rd,
        oranges,
        pi_yg,
        plasma,
        pr_gn,
        pu_bu,
        pu_bu_gn,
        pu_or,
        pu_rd,
        purples,
        rainbow,
        rd_bu,
        rd_gy,
        rd_pu,
        rd_yl_bu,
        rd_yl_gn,
        reds,
        sinebow,
        spectral,
        turbo,
        viridis,
        warm,
        yl_gn,
        yl_gn_bu,
        yl_or_br,
        yl_or_rd,
    )
}

fn generate_gradient(grad: &impl colorgrad::Gradient) -> Vec<Srgb> {
    let mut palette = Vec::new();
    let samples = 16;
    for x in 0..=samples {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette.push(Srgb::new(r, g, b, 255));
    }
    for x in (1..samples).rev() {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette.push(Srgb::new(r, g, b, 255));
    }
    palette
}

/// Stores a palette in a texture.
pub struct Palette {
    pub bind_group: wgpu::BindGroup,
    pub len: usize,
}

impl Palette {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, palette: &[Srgb]) -> Self {
        assert_eq!(palette.len(), 32);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: palette.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            crate::byte_slice(palette),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(palette.len() as u32 * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: palette.len() as u32,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &Self::bind_group_layout(device),
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
        });

        Self {
            bind_group,
            len: palette.len(),
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}
