use crate::{
    compute::ComputePipeline,
    palette::Palette,
    ssaa::SsaaPipeline,
    tween::{Interpolate, Lerp, LogF32},
};
use tint::Srgb;

crate::lerp! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Fractal {
        pub iterations: u32,
        pub escape_radius: f32,
        pub color_scale: f32,
        pub exponent: f32,
        pub view: View,
        pub rotation: f32,
        pub julia: f32,
        pub burning_ship: f32,
        pub cx: f32,
        pub cy: f32,
        pub zx: f32,
        pub zy: f32,
        pub pad1: u32,
        pub pad2: u32,
    }
}

impl Default for Fractal {
    fn default() -> Self {
        Self {
            iterations: 1000,
            escape_radius: 100.0,
            color_scale: 1.0,
            exponent: 2.0,
            view: View::default(),
            rotation: 0.0,
            julia: 0.0,
            burning_ship: 0.0,
            cx: 0.0,
            cy: 0.0,
            zx: 0.0,
            zy: 0.0,
            pad1: 0,
            pad2: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 1.25,
        }
    }
}

impl Interpolate for View {
    type Output = View;
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self::Output {
        let z = LogF32::interpolate(&LogF32(start.z), &LogF32(end.z), t);
        let factor = if (end.z - start.z).abs() < f32::EPSILON {
            t
        } else {
            (z - start.z) / (end.z - start.z)
        };
        View {
            x: start.x.lerp(&end.x, factor),
            y: start.y.lerp(&end.y, factor),
            z,
        }
    }
}

pub struct Renderer {
    pub config: Fractal,
    width: usize,
    height: usize,
    device: wgpu::Device,
    queue: wgpu::Queue,
    compute: ComputePipeline,
    ssaa: SsaaPipeline,
    palette: Palette,
    output_buffer: wgpu::Buffer,
    bytes_per_row: usize,
}

impl Renderer {
    pub fn new(samples: usize, width: usize, height: usize, palette: &[Srgb]) -> Self {
        env_logger::init();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        println!("[ADAPTER] {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .expect("Failed to create device");

        let texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let ssaa = SsaaPipeline::new(&device, texture_format, width, height, samples);
        let compute = ComputePipeline::new(&device, &ssaa);
        let palette = Palette::new(&device, &queue, palette);

        let (bytes_per_row, buffer_size) = output_buffer_bytes_per_row_and_size(width, height);
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            config: Fractal::default(),
            width,
            height,
            device,
            queue,
            compute,
            ssaa,
            palette,
            output_buffer,
            bytes_per_row,
        }
    }

    pub fn render(&mut self) {
        self.compute.write_buffers(&self.queue, &self.config);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.compute.compute_pass(
            &mut encoder,
            &self.palette,
            &self.ssaa,
            self.width,
            self.height,
        );
        self.ssaa.render_pass(&mut encoder);
        self.queue.submit([encoder.finish()]);
    }

    /// Copy the output buffer into a staging buffer then block while staging
    /// buffer maps to CPU memory.
    pub fn read_output_buffer(&self) -> Vec<Srgb> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.ssaa.output_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();

        let padded_data = buffer_slice.get_mapped_range();
        let mut result = Vec::with_capacity(self.width * self.height);
        for chunk in padded_data.chunks(self.bytes_per_row) {
            result.extend(
                chunk[..self.width * 4]
                    .chunks(4)
                    .map(|c| Srgb::new(c[0], c[1], c[2], c[3])),
            );
        }
        drop(padded_data);
        self.output_buffer.unmap();

        result
    }
}

fn output_buffer_bytes_per_row_and_size(width: usize, height: usize) -> (usize, usize) {
    let bytes_per_pixel = 4;
    let align = 256;
    let bpr = width * bytes_per_pixel;
    let padding = (align - bpr % align) % align;
    let bpr = bpr + padding;
    let buffer_size = bpr * height;
    (bpr, buffer_size)
}
