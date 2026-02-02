use super::{Fractal, ssaa::SsaaPipeline};
use crate::{
    encoder::Encoder,
    render::{
        BurningShip, CPlane, ColorScale, EscapeRadius, Exponent, Iterations, Julia, Renderer,
        Rotation, View, ZPlane, palette::PaletteBindGroup,
    },
};
use bevy_ecs::prelude::*;

/// Perform iterative mandelbrot computation in a compute shader.
///
/// Every frame will increment the orbit of each pixel up to a certain threshold
/// in order to prevent the device from timing out.
#[derive(Component)]
pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>, ssaa: Single<&SsaaPipeline>) {
    commands.spawn(ComputePipeline::new(&renderer.device, ssaa.into_inner()));
}

pub fn compute_pass(
    renderer: Single<&Renderer>,
    pipeline: Single<&ComputePipeline>,
    ssaa: Single<&SsaaPipeline>,
    palette: Single<&PaletteBindGroup>,
    //
    iterations: Single<&Iterations>,
    escape_radius: Single<&EscapeRadius>,
    color_scale: Single<&ColorScale>,
    exponent: Single<&Exponent>,
    view: Single<&View>,
    rotation: Single<&Rotation>,
    julia: Single<&Julia>,
    burning_ship: Single<&BurningShip>,
    c: Single<&CPlane>,
    z: Single<&ZPlane>,
    //
    _enable: Single<&Encoder>,
) {
    let fractal = Fractal {
        iterations: iterations.0,
        escape_radius: escape_radius.0,
        color_scale: color_scale.0,
        exponent: exponent.0,
        view: **view,
        rotation: rotation.0,
        julia: julia.0,
        burning_ship: burning_ship.0,
        c: **c,
        z: **z,
        pad1: 0,
        pad2: 0,
    };
    renderer
        .queue
        .write_buffer(&pipeline.uniform, 0, crate::byte_slice(&[fractal]));

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(&pipeline.pipeline);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    cpass.set_bind_group(1, &palette.bind_group, &[]);

    let ssaa_factor = ssaa.ssaa_factor();
    let x = (renderer.width * ssaa_factor).div_ceil(16) as u32;
    let y = (renderer.height * ssaa_factor).div_ceil(16) as u32;
    cpass.dispatch_workgroups(x, y, 1);
    drop(cpass);
    renderer.queue.submit([encoder.finish()]);
}

impl ComputePipeline {
    pub fn new(device: &wgpu::Device, ssaa: &SsaaPipeline) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<Fractal>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssaa.render_target()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
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

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform,
        }
    }
}
