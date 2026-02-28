use crate::{
    encoder::EncodingTarget,
    params::{Buddha, Params},
    render::{Renderer, ssaa::SsaaPipeline},
};
use bevy_ecs::prelude::*;
use std::num::NonZeroU64;

#[derive(Component)]
pub struct BuddhaPipeline {
    compute: wgpu::ComputePipeline,
    min_max: wgpu::ComputePipeline,
    render: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    iterations: wgpu::Buffer,
    norm: wgpu::Buffer,
    buddha_bytes: u64,
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>, ssaa: Single<&SsaaPipeline>) {
    commands.spawn(BuddhaPipeline::new(
        &renderer.device,
        ssaa.into_inner(),
        renderer.width,
        renderer.height,
    ));
}

pub fn compute_pass(
    renderer: Single<&Renderer>,
    pipeline: Single<&BuddhaPipeline>,
    params: Single<(&Params, &Buddha), With<EncodingTarget>>,
) {
    let (params, buddha) = params.into_inner();
    if **buddha <= 0.0 {
        return;
    }
    // TODO: only write these if they change, but a simple Ref check won't do
    renderer
        .queue
        .write_buffer(&pipeline.uniform, 0, crate::byte_slice(&[*params]));

    renderer
        .queue
        .write_buffer_with(
            &pipeline.iterations,
            0,
            NonZeroU64::new(pipeline.buddha_bytes).unwrap(),
        )
        .unwrap()
        .fill(0);
    renderer.queue.write_buffer(
        &pipeline.norm,
        0,
        crate::byte_slice(&[f32::MAX, 0.0, f32::MAX, 0.0, f32::MAX, 0.0]),
    );

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(&pipeline.compute);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    let x = (renderer.width as u32 * params.buddha_samples.0).div_ceil(8);
    let y = (renderer.height as u32 * params.buddha_samples.0).div_ceil(8);
    cpass.dispatch_workgroups(x, y, 1);
    drop(cpass);
    renderer.queue.submit([encoder.finish()]);

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(&pipeline.min_max);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    let x = (renderer.width * renderer.height).div_ceil(256) as u32;
    cpass.dispatch_workgroups(x, 1, 1);
    drop(cpass);

    let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    cpass.set_pipeline(&pipeline.render);
    cpass.set_bind_group(0, &pipeline.bind_group, &[]);
    let x = renderer.width.div_ceil(16) as u32;
    let y = renderer.height.div_ceil(16) as u32;
    cpass.dispatch_workgroups(x, y, 1);
    drop(cpass);

    renderer.queue.submit([encoder.finish()]);
}

impl BuddhaPipeline {
    pub fn new(device: &wgpu::Device, ssaa: &SsaaPipeline, width: usize, height: usize) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let buddha_bytes = (std::mem::size_of::<u32>() * 3 * width * height) as u64;
        let iterations = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buddha_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let norm = device.create_buffer(&wgpu::BufferDescriptor {
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
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
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: iterations.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(ssaa.buddha_render_target()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: norm.as_entire_binding(),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shaders/buddha.wgsl"),
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

        let compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("compute_buddha"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let min_max = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("buddha_min_max"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let render = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("render_buddha"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Self {
            compute,
            min_max,
            render,
            bind_group,
            uniform,
            iterations,
            norm,
            buddha_bytes,
        }
    }
}
