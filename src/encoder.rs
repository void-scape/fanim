use crate::{
    audio::{SampleRate, Samples},
    params::Params,
    prelude::{AnimationOf, AnimationTarget, Animations, DeltaTime, Finished},
    render::{OutputBuffer, RenderSystems, Renderer},
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
            ((encode_image, encode_video), encode_collage)
                .chain()
                .after(RenderSystems::MapOutput),
        );
    }
}

#[derive(Component)]
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
    collages: Query<(), With<CollageEncoder>>,
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
            if collages.is_empty() {
                writer.write(AppExit::Success);
            }
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

#[derive(Component)]
pub struct CollageEncoder {
    output_path: String,
    data_path: String,
    init: Option<usize>,
}

impl CollageEncoder {
    pub fn new<P1: AsRef<Path>, P2: AsRef<Path>>(output_path: P1, data_path: P2) -> Self {
        Self {
            output_path: output_path.as_ref().to_string_lossy().to_string(),
            data_path: data_path.as_ref().to_string_lossy().to_string(),
            init: None,
        }
    }
}

fn encode_collage(
    mut commands: Commands,
    renderer: Single<&Renderer>,
    collage_encoder: Single<(Entity, &mut CollageEncoder, Option<&Children>)>,
) -> bevy_ecs::error::Result {
    let (entity, mut encoder, children) = collage_encoder.into_inner();
    match encoder.init {
        Some(count) => {
            if children.is_none() {
                commands.entity(entity).despawn();
                collage(
                    &(0..count)
                        .map(|i| format!("{}/{i}.png", encoder.data_path))
                        .collect::<Vec<_>>(),
                    renderer.width,
                    renderer.height,
                    &encoder.output_path,
                )?;
                println!(
                    "[LOG] Wrote {} bytes to {}",
                    std::fs::metadata(&encoder.output_path)?.size(),
                    encoder.output_path
                );
            }
        }
        None => {
            for (i, entity) in children
                .expect("`CollageEncoder` has children")
                .iter()
                .enumerate()
            {
                commands
                    .entity(entity)
                    .insert(ImageEncoder::new(format!("{}/{i}.png", encoder.data_path)));
            }
            encoder.init = Some(children.unwrap().len());
        }
    }
    Ok(())
}

fn collage(files: &[String], width: usize, height: usize, output: &str) -> std::io::Result<()> {
    let count = files.len();
    let mut cols = count.isqrt();
    if cols * cols != count {
        cols += 1;
    }
    let rows = count.div_ceil(cols);
    let mut collage = vec![0u8; width * cols * height * rows * 4];
    for (i, file) in files.iter().enumerate() {
        let file = std::fs::File::open(file)?;
        let reader = std::io::BufReader::new(file);
        let mut reader = png::Decoder::new(reader).read_info()?;
        let mut frame = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut frame).unwrap();

        let xoffset = (i % cols) * width;
        let yoffset = (i / cols) * height;

        for y in 0..height {
            let src_start = y * width * 4;
            let dest_start = ((yoffset + y) * width * cols + xoffset) * 4;
            collage[dest_start..dest_start + width * 4]
                .copy_from_slice(&frame[src_start..src_start + width * 4]);
        }
    }
    png(output, &collage, width * cols, height * rows)
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
