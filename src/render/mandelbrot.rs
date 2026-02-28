use super::ssaa::SsaaPipeline;
use crate::{encoder::EncodingTarget, params::*, render::*};

#[derive(Component)]
pub struct MandelbrotPipeline {
    render: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    palette: wgpu::Texture,
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>, ssaa: Single<&SsaaPipeline>) {
    commands.spawn(MandelbrotPipeline::new(&renderer.device, ssaa.into_inner()));
}

pub fn compute_pass(
    renderer: Single<&Renderer>,
    pipeline: Single<&MandelbrotPipeline>,
    ssaa: Single<&SsaaPipeline>,
    params: Single<(&Params, &Palette, &Mandelbrot), With<EncodingTarget>>,
) {
    // TODO: only write these if they change, but a simple Ref check won't do
    let (params, palette, mandelbrot) = params.into_inner();
    if **mandelbrot <= 0.0 {
        return;
    }
    renderer.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &pipeline.palette,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        crate::byte_slice(&palette.0),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(PALETTE_LEN as u32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: PALETTE_LEN as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    renderer
        .queue
        .write_buffer(&pipeline.uniform, 0, crate::byte_slice(&[*params]));

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(&pipeline.render);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    let ssaa_factor = ssaa.ssaa_factor();
    let x = (renderer.width * ssaa_factor).div_ceil(16) as u32;
    let y = (renderer.height * ssaa_factor).div_ceil(16) as u32;
    cpass.dispatch_workgroups(x, y, 1);
    drop(cpass);

    renderer.queue.submit([encoder.finish()]);
}

impl MandelbrotPipeline {
    pub fn new(device: &wgpu::Device, ssaa: &SsaaPipeline) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let palette = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: PALETTE_LEN as u32,
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
        let palette_view = palette.create_view(&Default::default());
        let palette_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssaa.mandelbrot_render_target()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&palette_sampler),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/mandelbrot.wgsl"),
                    include_str!("shaders/shared.wgsl"),
                )
                .into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let render = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("render_mandelbrot"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            render,
            bind_group,
            uniform,
            palette,
        }
    }
}
