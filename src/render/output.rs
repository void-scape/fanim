use crate::render::{Renderer, ssaa::SsaaPipeline};
use bevy_ecs::prelude::*;
use tint::Srgb;

#[derive(Component)]
pub struct OutputBuffer {
    pixels: Vec<Srgb>,
    buffer: wgpu::Buffer,
    bytes_per_row: usize,
}

impl OutputBuffer {
    pub fn pixels(&self) -> &[Srgb] {
        &self.pixels
    }
}

pub fn spawn(mut commands: Commands, renderer: Single<&Renderer>) {
    let align = 256;
    let bpr = renderer.width * 4;
    let padding = (align - bpr % align) % align;
    let buffer_size = bpr * renderer.height;
    let bytes_per_row = bpr + padding;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    commands.spawn(OutputBuffer {
        pixels: vec![Srgb::default(); renderer.width * renderer.height],
        buffer,
        bytes_per_row,
    });
}

pub fn map_output(
    mut output: Single<&mut OutputBuffer>,
    renderer: Single<&Renderer>,
    ssaa: Single<&SsaaPipeline>,
) {
    let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: ssaa.output_texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output.buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(output.bytes_per_row as u32),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: renderer.width as u32,
            height: renderer.height as u32,
            depth_or_array_layers: 1,
        },
    );
    renderer.queue.submit(Some(encoder.finish()));

    let buffer_slice = output.buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();

    let padded_data = buffer_slice.get_mapped_range();
    output.pixels.clear();
    for chunk in padded_data.chunks(output.bytes_per_row) {
        output.pixels.extend(
            chunk[..renderer.width * 4]
                .chunks(4)
                .map(|c| Srgb::new(c[0], c[1], c[2], c[3])),
        );
    }
    drop(padded_data);
    output.buffer.unmap();
}
