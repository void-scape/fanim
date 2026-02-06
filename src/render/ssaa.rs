use crate::{
    encoder::EncodingTarget,
    params::*,
    render::{Renderer, Rerender},
};
use bevy_ecs::prelude::*;

#[repr(C)]
struct SsaaUniform {
    buddha: f32,
    mandelbrot: f32,
}

#[derive(Component)]
pub struct SsaaPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    mandelbrot: wgpu::TextureView,
    buddha: wgpu::TextureView,
    dst: wgpu::Texture,
    dst_view: wgpu::TextureView,
    samples: usize,
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>) {
    commands.spawn(SsaaPipeline::new(
        &renderer.device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        renderer.width,
        renderer.height,
        renderer.super_samples,
    ));
}

pub fn render_pass(
    renderer: Single<&Renderer>,
    pipeline: Single<&SsaaPipeline>,
    _target: Single<(), (With<Params>, With<Rerender>, With<EncodingTarget>)>,
    // opacity: Single<(Ref<Buddha>, Ref<Mandelbrot>)>,
) {
    // let (buddha, mandelbrot) = opacity.into_inner();
    // if buddha.is_changed() || mandelbrot.is_changed() {
    renderer.queue.write_buffer(
        &pipeline.uniform,
        0,
        crate::byte_slice(&[SsaaUniform {
            buddha: 0.0,
            mandelbrot: 1.0,
        }]),
    );
    // }

    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &pipeline.dst_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rpass.set_pipeline(&pipeline.pipeline);
    rpass.set_bind_group(0, &pipeline.bind_group, &[]);
    rpass.draw(0..3, 0..1);
    drop(rpass);
    renderer.queue.submit([encoder.finish()]);
}

impl SsaaPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: usize,
        height: usize,
        samples: usize,
    ) -> Self {
        let dst_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: None,
            view_formats: &[],
        };
        let dst = device.create_texture(&dst_desc);
        let dst_view = dst.create_view(&Default::default());

        let mandelbrot_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: (width * samples) as u32,
                height: (height * samples) as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
            view_formats: &[],
        };
        let mandelbrot = device
            .create_texture(&mandelbrot_desc)
            .create_view(&Default::default());

        let buddha_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            label: None,
            view_formats: &[],
        };
        let buddha = device
            .create_texture(&buddha_desc)
            .create_view(&Default::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<SsaaUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
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
                    resource: wgpu::BindingResource::TextureView(&mandelbrot),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&buddha),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/ssaa.wgsl"));
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(format.into())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform,
            mandelbrot,
            buddha,
            dst,
            dst_view,
            samples,
        }
    }

    pub fn ssaa_factor(&self) -> usize {
        self.samples
    }

    pub fn mandelbrot_render_target(&self) -> &wgpu::TextureView {
        &self.mandelbrot
    }

    pub fn buddha_render_target(&self) -> &wgpu::TextureView {
        &self.buddha
    }

    pub fn output_texture(&self) -> &wgpu::Texture {
        &self.dst
    }
}
