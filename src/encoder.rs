use crate::{
    animation::DeltaTime,
    byte_slice,
    render::{Renderer, ssaa::SsaaPipeline},
};
use bevy_app::{AppExit, Last, Plugin, PostStartup};
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
        let spawn_encoder =
            move |mut commands: Commands, renderer: Single<&Renderer>| -> bevy_ecs::error::Result {
                commands.spawn(Encoder::new(
                    output_path.clone(),
                    data_path.clone(),
                    renderer.width,
                    renderer.height,
                    fps,
                    sample_rate,
                )?);
                commands.spawn(DeltaTime(1.0 / fps as f32));
                Ok(())
            };
        app.add_systems(PostStartup, spawn_encoder)
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
    width: usize,
    height: usize,
    fps: usize,
    sample_rate: usize,
    samples_per_frame: usize,
    audio_file: BufWriter<File>,
    frame: usize,
    time: f32,
}

impl Encoder {
    pub fn new(
        output_path: String,
        data_path: String,
        width: usize,
        height: usize,
        fps: usize,
        sample_rate: usize,
    ) -> std::io::Result<Self> {
        assert!(
            sample_rate.is_multiple_of(fps),
            "the sample rate needs to be divisible by fps in order \
            to collect whole samples"
        );
        let file = File::create(format!("{data_path}/samples.ppm"))?;

        Ok(Self {
            audio_file: BufWriter::new(file),
            output_path,
            data_path,
            width,
            height,
            fps,
            sample_rate,
            samples_per_frame: sample_rate / fps,
            frame: 0,
            time: 0.0,
        })
    }

    /// Write `pixels` and `samples` into `data_path`.
    ///
    /// `pixels` generates the entire frame at once, whereas `samples` is repeatedly
    /// called until the necessary number of samples is collected.
    pub fn render_frame<S>(&mut self, pixels: &[Srgb], samples: &mut S) -> std::io::Result<()>
    where
        S: FnMut(f32) -> (f32, f32),
    {
        assert_eq!(
            pixels.len(),
            self.width * self.height,
            "tried to call `Encoder::render_frames` \
            with the incorrect pixel dimensions"
        );
        let output = format!("{}/{}.png", self.data_path, self.frame);
        png(&output, byte_slice(pixels), self.width, self.height)?;
        let dt = 1.0 / self.sample_rate as f32;
        for _ in 0..self.samples_per_frame {
            let (l, r) = samples(self.time);
            self.audio_file.write_all(&l.to_le_bytes())?;
            self.audio_file.write_all(&r.to_le_bytes())?;
            self.time += dt;
        }
        println!("[LOG] Rendered frame {}", self.frame);
        self.frame += 1;
        Ok(())
    }
}

pub fn render_frame(
    renderer: Single<&Renderer>,
    ssaa: Single<&SsaaPipeline>,
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

    video_encoder.render_frame(&pixels, &mut |_| (0.0, 0.0))?;
    Ok(())
}

pub fn finish(
    mut writer: MessageWriter<AppExit>,
    mut encoder: Single<&mut Encoder>,
) -> bevy_ecs::error::Result {
    encoder.audio_file.flush()?;
    ffmpeg(
        &encoder.data_path,
        &encoder.output_path,
        encoder.fps,
        encoder.sample_rate,
    )?;
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
