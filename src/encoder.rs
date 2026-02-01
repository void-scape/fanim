use crate::byte_slice;
use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};
use tint::Srgb;

/// Encodes image and audio data into an mp4 video.
///
/// Uses a `data_path` to store intermediate frame and audio data before compiling
/// with `ffmpeg` in [`Encoder::finish`].
pub struct Encoder {
    data_path: String,
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
    pub fn new<P: AsRef<Path>>(
        data_path: P,
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
        let data_path = data_path.as_ref().to_str().unwrap().to_string();
        let file = File::create(format!("{data_path}/samples.ppm"))?;

        Ok(Self {
            audio_file: BufWriter::new(file),
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

    /// Step `event` until it is completely rendered.
    pub fn render_event(&mut self, event: &mut impl Event) -> std::io::Result<()> {
        event.complete(self, 1.0 / self.fps as f32)
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

    /// Generate an mp4 video from the `data_path` to `output_path`.
    pub fn finish<P: AsRef<Path>>(mut self, output_path: P) -> std::io::Result<()> {
        self.audio_file.flush()?;
        ffmpeg(
            &self.data_path,
            output_path.as_ref().to_str().unwrap(),
            self.fps,
            self.sample_rate,
        )
    }
}

pub trait Event: Sized {
    fn step(&mut self, encoder: &mut Encoder, dt: f32) -> std::io::Result<bool>;
    fn complete(&mut self, encoder: &mut Encoder, dt: f32) -> std::io::Result<()> {
        while self.step(encoder, dt)? {}
        Ok(())
    }
}

impl<F> Event for F
where
    F: FnMut(&mut Encoder, f32) -> std::io::Result<bool>,
{
    fn step(&mut self, encoder: &mut Encoder, dt: f32) -> std::io::Result<bool> {
        self(encoder, dt)
    }
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
