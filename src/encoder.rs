use crate::{
    animation::DeltaTime,
    audio::{SampleRate, Samples},
    byte_slice,
    render::{Renderer, ssaa::SsaaPipeline},
};
use bevy_app::{AppExit, Last, Plugin, PreStartup};
use bevy_ecs::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write},
    os::unix::fs::MetadataExt,
};
use tint::Srgb;

pub struct EncoderPlugin {
    pub fps: usize,
    pub sample_rate: usize,
    pub data_path: String,
    pub output_path: String,
}

impl Plugin for EncoderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let fps = self.fps;
        let sample_rate = self.sample_rate;
        let output_path = self.output_path.clone();
        let data_path = self.data_path.clone();
        let spawn_encoder = move |mut commands: Commands| -> bevy_ecs::error::Result {
            commands.spawn(Encoder::new(
                output_path.clone(),
                data_path.clone(),
                fps,
                sample_rate,
            )?);
            commands.spawn(DeltaTime(1.0 / fps as f32));
            assert!(
                sample_rate.is_multiple_of(fps),
                "the sample rate needs to be divisible by fps in order \
                    to collect whole samples"
            );
            commands.spawn(Samples(vec![(0.0, 0.0); sample_rate / fps]));
            commands.spawn(SampleRate(sample_rate));
            Ok(())
        };
        app.add_systems(PreStartup, spawn_encoder)
            .add_systems(Last, render_frame);
    }
}

/// Encodes image and audio data into an mp4 video.
///
/// Uses a `data_path` to store intermediate frame and audio data before compiling
/// with `ffmpeg` in [`finish`].
#[derive(Component)]
pub struct Encoder {
    data_path: String,
    output_path: String,
    fps: usize,
    sample_rate: usize,
    audio_file: BufWriter<File>,
    frame: usize,
}

impl Encoder {
    pub fn new(
        output_path: String,
        data_path: String,
        fps: usize,
        sample_rate: usize,
    ) -> std::io::Result<Self> {
        let file = File::create(format!("{data_path}/samples.ppm"))?;
        Ok(Self {
            audio_file: BufWriter::new(file),
            output_path,
            data_path,
            fps,
            sample_rate,
            frame: 0,
        })
    }
}

pub fn render_frame(
    renderer: Single<&Renderer>,
    ssaa: Single<&SsaaPipeline>,
    samples: Single<&Samples>,
    mut video_encoder: Single<&mut Encoder>,
) -> bevy_ecs::error::Result {
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
    let mut pixels = Vec::with_capacity(renderer.width * renderer.height);
    for chunk in padded_data.chunks(renderer.bytes_per_row) {
        pixels.extend(
            chunk[..renderer.width * 4]
                .chunks(4)
                .map(|c| Srgb::new(c[0], c[1], c[2], c[3])),
        );
    }
    drop(padded_data);
    renderer.output_buffer.unmap();

    // write image data
    let output = format!("{}/{}.png", video_encoder.data_path, video_encoder.frame);
    png(
        &output,
        byte_slice(&pixels),
        renderer.width,
        renderer.height,
    )?;
    // write sample data
    // TODO: endianess
    video_encoder.audio_file.write_all(unsafe {
        std::slice::from_raw_parts(samples.as_ptr().cast(), samples.len() * 8)
    })?;

    println!("[LOG] Rendered frame {}", video_encoder.frame);
    video_encoder.frame += 1;

    Ok(())
}

pub fn finish(
    mut commands: Commands,
    mut writer: MessageWriter<AppExit>,
    encoder: Single<(Entity, &mut Encoder)>,
) -> bevy_ecs::error::Result {
    let (entity, mut encoder) = encoder.into_inner();
    encoder.audio_file.flush()?;
    ffmpeg(
        &encoder.data_path,
        &encoder.output_path,
        encoder.fps,
        encoder.sample_rate,
    )?;
    // NOTE: Despawn encoder so that another frame isn't rendered.
    commands.entity(entity).despawn();
    writer.write(AppExit::Success);
    println!(
        "[LOG] Wrote {} bytes to {}",
        std::fs::metadata(&encoder.output_path)?.size(),
        encoder.output_path
    );
    Ok(())
}

// Fast png encoding using the rust `png` crate.
fn png(output: &str, frame: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    let file = std::fs::File::create(output)?;
    let output = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(output, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();
    writer.write_image_data(frame).unwrap();
    Ok(())
}

// Spawn ffmpeg in a process, maybe this can become static later, but looking into
// it, that seems heinous.
fn ffmpeg(root: &str, output: &str, fps: usize, sample_rate: usize) -> std::io::Result<()> {
    let fps = &format!("{fps}");
    let frames = &format!("{root}/%d.png");
    let audio = &format!("{root}/samples.ppm");
    let sample_rate = &format!("{sample_rate}");
    #[rustfmt::skip]
    std::process::Command::new("ffmpeg")
        .args([
            // force overwrite
            "-y",
            "-framerate", fps,
            "-i", frames,
            "-f", "f32le",
            "-ar", sample_rate,
            "-ac", "2",
            "-i", audio,
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "23",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "192k",
            output,
        ])
            .spawn()
            .unwrap()
            .wait()?;
    Ok(())
}
