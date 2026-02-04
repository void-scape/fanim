use super::{Fractal, ssaa::SsaaPipeline};
use crate::{
    encoder::Encoder,
    render::{palette::PaletteBindGroup, *},
};
use bevy_ecs::system::SystemChangeTick;
use std::num::NonZeroU64;

#[derive(Component)]
pub struct ComputePipeline {
    render_mandelbrot: wgpu::ComputePipeline,
    compute_buddha: wgpu::ComputePipeline,
    render_buddha: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    buddha: wgpu::Buffer,
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
    palette: Single<&PaletteBindGroup>,
    fractal: Single<(
        EntityRef,
        &Iterations,
        &EscapeRadius,
        &ColorScale,
        &Exponent,
        &View,
        &Rotation,
        &Julia,
        &BurningShip,
        &Buddha,
        &Mandelbrot,
        &CPlane,
        &ZPlane,
        &ColorRotation,
        &BuddhaSamples,
    )>,
    system_ticks: SystemChangeTick,
    _enable: Single<&Encoder>,
) {
    let (
        entity,
        iterations,
        escape_radius,
        color_scale,
        exponent,
        view,
        rotation,
        julia,
        burning_ship,
        buddha,
        mandelbrot,
        c,
        z,
        color_rotation,
        buddha_samples,
    ) = fractal.into_inner();

    if entity.archetype().components().iter().all(|id| {
        entity.get_change_ticks_by_id(*id).is_some_and(|ticks| {
            !ticks.is_changed(system_ticks.last_run(), system_ticks.this_run())
        })
    }) {
        return;
    }

    let fractal = Fractal {
        iterations: iterations.0,
        escape_radius: escape_radius.0,
        color_scale: color_scale.0,
        exponent: exponent.0,
        view: *view,
        rotation: rotation.0,
        julia: julia.0,
        burning_ship: burning_ship.0,
        c: *c,
        z: *z,
        color_rotation: color_rotation.0,
        buddha: buddha.0,
        mandelbrot: mandelbrot.0,
        buddha_samples: buddha_samples.0,
    };
    renderer
        .queue
        .write_buffer(&pipeline.uniform, 0, crate::byte_slice(&[fractal]));

    if **mandelbrot > 0.0 {
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline.render_mandelbrot);
        cpass.set_bind_group(0, &pipeline.bind_group, &[]);
        cpass.set_bind_group(1, &palette.bind_group, &[]);
        let ssaa_factor = ssaa.ssaa_factor();
        let x = (renderer.width * ssaa_factor).div_ceil(16) as u32;
        let y = (renderer.height * ssaa_factor).div_ceil(16) as u32;
        cpass.dispatch_workgroups(x, y, 1);
        drop(cpass);

        renderer.queue.submit([encoder.finish()]);
    }

    if **buddha > 0.0 {
        renderer
            .queue
            .write_buffer_with(
                &pipeline.buddha,
                0,
                NonZeroU64::new(pipeline.buddha_bytes).unwrap(),
            )
            .unwrap()
            .fill(0);

        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline.compute_buddha);
        cpass.set_bind_group(0, &pipeline.bind_group, &[]);
        cpass.set_bind_group(1, &palette.bind_group, &[]);
        let x = (renderer.width as u32 * buddha_samples.0).div_ceil(16);
        let y = (renderer.height as u32 * buddha_samples.0).div_ceil(16);
        cpass.dispatch_workgroups(x, y, 1);
        drop(cpass);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline.render_buddha);
        cpass.set_bind_group(0, &pipeline.bind_group, &[]);
        cpass.set_bind_group(1, &palette.bind_group, &[]);
        let x = renderer.width.div_ceil(16) as u32;
        let y = renderer.height.div_ceil(16) as u32;
        cpass.dispatch_workgroups(x, y, 1);
        drop(cpass);

        renderer.queue.submit([encoder.finish()]);
    }
}

impl ComputePipeline {
    pub fn new(device: &wgpu::Device, ssaa: &SsaaPipeline, width: usize, height: usize) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<Fractal>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let buddha_bytes = (std::mem::size_of::<u32>() * width * height) as u64;
        let buddha = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buddha_bytes,
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
                    resource: buddha.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(ssaa.buddha_render_target()),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::include_wgsl!("shaders/fractal.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &bind_group_layout,
                &PaletteBindGroup::bind_group_layout(device),
            ],
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
            render_buddha,
            bind_group,
            uniform,
            buddha,
            buddha_bytes,
        }
    }
}
