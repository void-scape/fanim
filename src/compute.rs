use crate::{palette::Palette, render::Fractal, ssaa::SsaaPipeline};

/// Perform iterative mandelbrot computation in a compute shader.
///
/// Every frame will increment the orbit of each pixel up to a certain threshold
/// in order to prevent the device from timing out.
pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
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
            bind_group_layouts: &[&bind_group_layout, &Palette::bind_group_layout(device)],
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

    pub fn write_buffers(&self, queue: &wgpu::Queue, config: &Fractal) {
        queue.write_buffer(&self.uniform, 0, crate::byte_slice(&[*config]));
    }

    pub fn compute_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        palette: &Palette,
        ssaa: &SsaaPipeline,
        width: usize,
        height: usize,
    ) {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.set_bind_group(1, &palette.bind_group, &[]);

        let ssaa_factor = ssaa.ssaa_factor();
        let x = (width * ssaa_factor).div_ceil(16) as u32;
        let y = (height * ssaa_factor).div_ceil(16) as u32;
        cpass.dispatch_workgroups(x, y, 1);
    }
}
