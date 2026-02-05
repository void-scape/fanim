use crate::{
    animation::DeltaTime,
    audio::{SampleRate, Samples},
    image::Image,
    prelude::{AnimationOf, Animations, Finished},
    render::{RenderSystems, Renderer},
};
use bevy_app::{AppExit, Last, Plugin, PostUpdate, PreStartup};
use bevy_ecs::prelude::*;
use std::{
    fs::File,
    io::{BufWriter, Write},
    os::unix::fs::MetadataExt,
};

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
            .add_systems(PostUpdate, encode_frame.after(RenderSystems::MapImage))
            .add_systems(Last, finish);
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

fn encode_frame(
    renderer: Single<&Renderer>,
    samples: Single<&Samples>,
    mut video_encoder: Single<&mut Encoder>,
    image: Single<&Image>,
) -> bevy_ecs::error::Result {
    // write image data
    let output = format!("{}/{}.png", video_encoder.data_path, video_encoder.frame);
    crate::image::png(
        &output,
        crate::byte_slice(image.as_slice()),
        renderer.width,
        renderer.height,
    )?;
    // write sample data
    // TODO: endianness
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
    animations: Query<(), (With<Animations>, Without<AnimationOf>, Without<Finished>)>,
) -> bevy_ecs::error::Result {
    if !animations.is_empty() {
        return Ok(());
    }

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
