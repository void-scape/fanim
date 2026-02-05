use crate::render::{RenderSystems, Renderer, ssaa::SsaaPipeline};
use bevy_app::{AppExit, Plugin, PostUpdate, Startup};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use std::os::unix::fs::MetadataExt;
use tint::Srgb;

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Startup, spawn_image)
            .add_systems(PostUpdate, map_image.in_set(RenderSystems::MapImage));
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct Image(pub Vec<Srgb>);

fn spawn_image(mut commands: Commands, renderer: Single<&Renderer>) {
    commands.spawn(Image(vec![
        Srgb::from_rgb(255, 0, 255);
        renderer.width * renderer.height
    ]));
}

fn map_image(
    renderer: Single<&Renderer>,
    ssaa: Single<&SsaaPipeline>,
    mut image: Single<&mut Image>,
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
            buffer: &renderer.output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(renderer.bytes_per_row as u32),
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

    let buffer_slice = renderer.output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();

    let padded_data = buffer_slice.get_mapped_range();
    image.clear();
    for chunk in padded_data.chunks(renderer.bytes_per_row) {
        image.extend(
            chunk[..renderer.width * 4]
                .chunks(4)
                .map(|c| Srgb::new(c[0], c[1], c[2], c[3])),
        );
    }
    drop(padded_data);
    renderer.output_buffer.unmap();
}

// Fast png encoding using the rust `png` crate.
pub fn png(output: &str, frame: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    let file = std::fs::File::create(output)?;
    let output = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(output, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(frame).unwrap();
    Ok(())
}

#[derive(Component)]
pub struct ImageOutput(pub String);

pub fn finish(
    mut writer: MessageWriter<AppExit>,
    image: Single<&Image>,
    output: Single<&ImageOutput>,
    renderer: Single<&Renderer>,
) -> bevy_ecs::error::Result {
    png(
        &output.0,
        crate::byte_slice(image.as_slice()),
        renderer.width,
        renderer.height,
    )?;
    writer.write(AppExit::Success);
    println!(
        "[LOG] Wrote {} bytes to {}",
        std::fs::metadata(&output.0)?.size(),
        output.0
    );
    Ok(())
}
