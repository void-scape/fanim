use super::ssaa::SsaaPipeline;
use crate::{encoder::EncodingTarget, params::*, render::*};

#[allow(unused)]
#[derive(Component)]
pub struct ComputePipeline {
    render_mandelbrot: wgpu::ComputePipeline,
    compute_buddha: wgpu::ComputePipeline,
    min_max_buddha: wgpu::ComputePipeline,
    render_buddha: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    palette: wgpu::Texture,
    buddha_iterations: wgpu::Buffer,
    buddha_norm: wgpu::Buffer,
    buddha_bytes: u64,
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>, ssaa: Single<&SsaaPipeline>) {
    commands.spawn(ComputePipeline::new(
        &renderer.device,
        ssaa.into_inner(),
        renderer.width,
        renderer.height,
    ));
}

pub fn compute_pass(
    renderer: Single<&Renderer>,
    pipeline: Single<&ComputePipeline>,
    ssaa: Single<&SsaaPipeline>,
    // opacity: Single<(Ref<Mandelbrot>, Ref<Buddha>, &BuddhaSamples)>,
    params: Single<(&Params, &Palette), With<EncodingTarget>>,
) {
    // TODO: only write these if they change, but a simple Ref check won't do
    let (params, palette) = params.into_inner();
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
    cpass.set_pipeline(&pipeline.render_mandelbrot);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    let ssaa_factor = ssaa.ssaa_factor();
    let x = (renderer.width * ssaa_factor).div_ceil(16) as u32;
    let y = (renderer.height * ssaa_factor).div_ceil(16) as u32;
    cpass.dispatch_workgroups(x, y, 1);
    drop(cpass);

    renderer.queue.submit([encoder.finish()]);

    //     renderer
    //         .queue
    //         .write_buffer_with(
    //             &pipeline.buddha_iterations,
    //             0,
    //             NonZeroU64::new(pipeline.buddha_bytes).unwrap(),
    //         )
    //         .unwrap()
    //         .fill(0);
    //     renderer.queue.write_buffer(
    //         &pipeline.buddha_norm,
    //         0,
    //         crate::byte_slice(&[f32::MAX, 0.0, f32::MAX, 0.0, f32::MAX, 0.0]),
    //     );
    //
    //     let mut encoder = renderer
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    //     let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //         label: None,
    //         timestamp_writes: None,
    //     });
    //     cpass.set_pipeline(&pipeline.compute_buddha);
    //     cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    //     let x = (renderer.width as u32 * buddha_samples.0).div_ceil(8);
    //     let y = (renderer.height as u32 * buddha_samples.0).div_ceil(8);
    //     cpass.dispatch_workgroups(x, y, 1);
    //     drop(cpass);
    //     renderer.queue.submit([encoder.finish()]);
    //
    //     let mut encoder = renderer
    //         .device
    //         .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    //
    //     let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //         label: None,
    //         timestamp_writes: None,
    //     });
    //     cpass.set_pipeline(&pipeline.min_max_buddha);
    //     cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    //     let x = (renderer.width * renderer.height).div_ceil(256) as u32;
    //     cpass.dispatch_workgroups(x, 1, 1);
    //     drop(cpass);
    //
    //     let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
    //         label: None,
    //         timestamp_writes: None,
    //     });
    //     cpass.set_pipeline(&pipeline.render_buddha);
    //     cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    //     let x = renderer.width.div_ceil(16) as u32;
    //     let y = renderer.height.div_ceil(16) as u32;
    //     cpass.dispatch_workgroups(x, y, 1);
    //     drop(cpass);
    //
    //     renderer.queue.submit([encoder.finish()]);
}

impl ComputePipeline {
    pub fn new(device: &wgpu::Device, ssaa: &SsaaPipeline, width: usize, height: usize) -> Self {
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

        let buddha_bytes = (std::mem::size_of::<u32>() * 3 * width * height) as u64;
        let buddha_iterations = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buddha_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let buddha_norm = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<u32>() as u64 * 2 * 3,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
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
                    resource: buddha_iterations.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(ssaa.buddha_render_target()),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: buddha_norm.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&palette_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&palette_sampler),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::include_wgsl!("shaders/fractal.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let render_mandelbrot = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("render_mandelbrot"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let compute_buddha = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("compute_buddha"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let min_max_buddha = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("buddha_min_max"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let render_buddha = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("render_buddha"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            render_mandelbrot,
            compute_buddha,
            min_max_buddha,
            render_buddha,
            bind_group,
            uniform,
            palette,
            buddha_iterations,
            buddha_norm,
            buddha_bytes,
        }
    }
}
