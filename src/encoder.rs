use crate::{
    audio::{SampleRate, Samples},
    params::Params,
    prelude::{AnimationOf, AnimationTarget, Animations, DeltaTime, Finished},
    render::{OutputBuffer, RenderSystems, Renderer, Rerender},
};
use bevy_app::{AppExit, Plugin, PostUpdate, PreUpdate};
use bevy_ecs::{lifecycle::HookContext, prelude::*, world::DeferredWorld};
use std::{
    fs::File,
    io::{BufWriter, Write},
    os::unix::fs::MetadataExt,
    path::Path,
};

pub struct EncoderPlugin;

impl Plugin for EncoderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PreUpdate, choose_target).add_systems(
            PostUpdate,
            (encode_image, encode_video).after(RenderSystems::MapOutput),
        );
    }
}

#[derive(Component)]
#[require(Rerender)]
pub struct EncodingTarget;

fn choose_target(
    mut commands: Commands,
    mut writer: MessageWriter<AppExit>,
    encoders: Query<Entity, (Or<(With<ImageEncoder>, With<VideoEncoder>)>, With<Params>)>,
    active: Query<
        (),
        (
            With<EncodingTarget>,
            Or<(With<ImageEncoder>, With<VideoEncoder>)>,
        ),
    >,
) {
    if !active.is_empty() {
        return;
    }
    match encoders.iter().next() {
        Some(entity) => {
            println!("encoding {entity}");
            commands.entity(entity).insert(EncodingTarget);
        }
        None => {
            writer.write(AppExit::Success);
        }
    }
}

#[derive(Component)]
pub struct ImageEncoder(String);

impl ImageEncoder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self(path.as_ref().to_string_lossy().to_string())
    }
}

fn encode_image(
    mut commands: Commands,
    renderer: Single<&Renderer>,
    image_encoder: Single<(Entity, &ImageEncoder), With<EncodingTarget>>,
    output: Single<&OutputBuffer>,
) -> bevy_ecs::error::Result {
    let (entity, encoder) = image_encoder.into_inner();
    png(
        &encoder.0,
        crate::byte_slice(output.pixels()),
        renderer.width,
        renderer.height,
    )?;
    commands.entity(entity).despawn();
    println!(
        "[LOG] Wrote {} bytes to {}",
        std::fs::metadata(&encoder.0)?.size(),
        encoder.0
    );
    Ok(())
}

#[derive(Component)]
#[component(on_insert = Self::insert)]
pub struct VideoEncoder {
    output_path: String,
    data_path: String,
    sample_rate: usize,
    fps: usize,
    audio_file: BufWriter<File>,
    frame: usize,
}

impl VideoEncoder {
    #[track_caller]
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(
        output_path: P1,
        data_path: P2,
        sample_rate: usize,
        fps: usize,
    ) -> std::io::Result<Self> {
        let output_path = output_path.as_ref().to_string_lossy().to_string();
        let data_path = data_path.as_ref().to_string_lossy().to_string();
        let file = File::create(format!("{}/samples.ppm", data_path))?;
        assert!(
            sample_rate.is_multiple_of(fps),
            "the sample rate needs to be divisible by fps in order \
                to collect whole samples"
        );
        Ok(Self {
            audio_file: BufWriter::new(file),
            output_path,
            data_path,
            fps,
            sample_rate,
            frame: 0,
        })
    }

    fn insert(mut world: DeferredWorld, ctx: HookContext) {
        let encoder = world.entity(ctx.entity).get::<Self>().unwrap();
        let sample_rate = encoder.sample_rate;
        let fps = encoder.fps;
        world.commands().entity(ctx.entity).insert((
            DeltaTime(1.0 / fps as f32),
            Samples(vec![(0.0, 0.0); sample_rate / fps]),
            SampleRate(sample_rate),
        ));
    }
}

fn encode_video(
    mut commands: Commands,
    renderer: Single<&Renderer>,
    video_encoder: Single<(Entity, &mut VideoEncoder), With<EncodingTarget>>,
    animations: Query<&AnimationTarget, (With<Animations>, With<Finished>, Without<AnimationOf>)>,
    output: Single<&OutputBuffer>,
    samples: Single<&Samples>,
) -> bevy_ecs::error::Result {
    let (entity, mut encoder) = video_encoder.into_inner();
    let path = format!("{}/{}.png", encoder.data_path, encoder.frame);
    png(
        &path,
        crate::byte_slice(output.pixels()),
        renderer.width,
        renderer.height,
    )?;
    // TODO: endianness
    encoder.audio_file.write_all(unsafe {
        std::slice::from_raw_parts(samples.as_ptr().cast(), samples.len() * 8)
    })?;
    println!("[LOG] Rendered frame {}", encoder.frame);
    encoder.frame += 1;

    if animations.iter().any(|t| t.0 == entity) {
        commands.entity(entity).despawn();
        encoder.audio_file.flush()?;
        ffmpeg(
            &encoder.data_path,
            &encoder.output_path,
            encoder.fps,
            encoder.sample_rate,
        )?;
        println!(
            "[LOG] Wrote {} bytes to {}",
            std::fs::metadata(&encoder.output_path)?.size(),
            encoder.output_path
        );
    }

    Ok(())
}

// use crate::{
//     animation::DeltaTime,
//     audio::{SampleRate, Samples},
//     prelude::{AnimationOf, Animations, Finished},
//     render::{RenderSystems, Renderer},
// };
// use bevy_app::{AppExit, Last, Plugin, PostUpdate, PreStartup};
// use bevy_ecs::prelude::*;
// use std::{
//     fs::File,
//     io::{BufWriter, Write},
//     os::unix::fs::MetadataExt,
// };
//
// pub struct EncoderPlugin {
//     pub fps: usize,
//     pub sample_rate: usize,
//     pub data_path: String,
//     pub output_path: String,
// }
//
// impl Plugin for EncoderPlugin {
//     fn build(&self, app: &mut bevy_app::App) {
//         let fps = self.fps;
//         let sample_rate = self.sample_rate;
//         let output_path = self.output_path.clone();
//         let data_path = self.data_path.clone();
//         let spawn_encoder = move |mut commands: Commands| -> bevy_ecs::error::Result {
//             commands.spawn(Encoder::new(
//                 output_path.clone(),
//                 data_path.clone(),
//                 fps,
//                 sample_rate,
//             )?);
//             commands.spawn(DeltaTime(1.0 / fps as f32));
//             assert!(
//                 sample_rate.is_multiple_of(fps),
//                 "the sample rate needs to be divisible by fps in order \
//                     to collect whole samples"
//             );
//             commands.spawn(Samples(vec![(0.0, 0.0); sample_rate / fps]));
//             commands.spawn(SampleRate(sample_rate));
//             Ok(())
//         };
//         app.add_systems(PreStartup, spawn_encoder)
//             .add_systems(PostUpdate, encode_frame.after(RenderSystems::MapImage))
//             .add_systems(Last, finish);
//     }
// }
//
// /// Encodes image and audio data into an mp4 video.
// ///
// /// Uses a `data_path` to store intermediate frame and audio data before compiling
// /// with `ffmpeg` in [`finish`].
// #[derive(Component)]
// pub struct Encoder {
//     data_path: String,
//     output_path: String,
//     fps: usize,
//     sample_rate: usize,
//     audio_file: BufWriter<File>,
//     frame: usize,
// }
//
// impl Encoder {
//     pub fn new(
//         output_path: String,
//         data_path: String,
//         fps: usize,
//         sample_rate: usize,
//     ) -> std::io::Result<Self> {
//         let file = File::create(format!("{data_path}/samples.ppm"))?;
//         Ok(Self {
//             audio_file: BufWriter::new(file),
//             output_path,
//             data_path,
//             fps,
//             sample_rate,
//             frame: 0,
//         })
//     }
// }
//
// fn encode_frame(
//     renderer: Single<&Renderer>,
//     samples: Single<&Samples>,
//     mut video_encoder: Single<&mut Encoder>,
//     output: Single<&Output>,
// ) -> bevy_ecs::error::Result {
//     // write image data
//     let output = format!("{}/{}.png", video_encoder.data_path, video_encoder.frame);
//     png(
//         &output,
//         crate::byte_slice(image.as_slice()),
//         renderer.width,
//         renderer.height,
//     )?;
//     // write sample data
//     // TODO: endianness
//     video_encoder.audio_file.write_all(unsafe {
//         std::slice::from_raw_parts(samples.as_ptr().cast(), samples.len() * 8)
//     })?;
//
//     println!("[LOG] Rendered frame {}", video_encoder.frame);
//     video_encoder.frame += 1;
//
//     Ok(())
// }
//
// pub fn finish(
//     mut commands: Commands,
//     mut writer: MessageWriter<AppExit>,
//     encoder: Single<(Entity, &mut Encoder)>,
//     animations: Query<(), (With<Animations>, Without<AnimationOf>, Without<Finished>)>,
// ) -> bevy_ecs::error::Result {
//     if !animations.is_empty() {
//         return Ok(());
//     }
//
//     let (entity, mut encoder) = encoder.into_inner();
//     encoder.audio_file.flush()?;
//     ffmpeg(
//         &encoder.data_path,
//         &encoder.output_path,
//         encoder.fps,
//         encoder.sample_rate,
//     )?;
//     // NOTE: Despawn encoder so that another frame isn't rendered.
//     commands.entity(entity).despawn();
//     writer.write(AppExit::Success);
//     println!(
//         "[LOG] Wrote {} bytes to {}",
//         std::fs::metadata(&encoder.output_path)?.size(),
//         encoder.output_path
//     );
//     Ok(())
// }

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
