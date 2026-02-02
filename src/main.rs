use fanim::audio::*;
use fanim::encoder::Encoder;
use fanim::render::*;
use fanim::tween::*;
use hound::SampleFormat;

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Scene {
    pub audio: Audio,
    pub fractal: Fractal,
}

impl Interpolate for Scene {
    type Output = Self;
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self {
        Self {
            audio: start.audio.lerp(&end.audio, t),
            fractal: start.fractal.lerp(&end.fractal, t),
        }
    }
}

fn audio(f: Box<dyn Fn(&mut Audio)>) -> Box<dyn Fn(&mut Scene)> {
    Box::new(move |scene| f(&mut scene.audio))
}

fn fractal(f: Box<dyn Fn(&mut Fractal)>) -> Box<dyn Fn(&mut Scene)> {
    Box::new(move |scene| f(&mut scene.fractal))
}

fn main() -> std::io::Result<()> {
    let scale = 16;
    let width = 16 * scale;
    let height = 9 * scale;
    println!("{}x{}", width, height);

    let fps = 10;
    let sample_rate = 44_100;
    let samples = 1;

    let data_path = "data";
    _ = std::fs::remove_dir_all(data_path);
    _ = std::fs::create_dir_all(data_path);
    let mut encoder = Encoder::new(data_path, width, height, fps, sample_rate)?;

    let mut reader = hound::WavReader::open("assets/foreign.wav").unwrap();
    assert_eq!(reader.spec().sample_rate, sample_rate as u32);
    assert_eq!(reader.spec().channels, 2);
    assert_eq!(reader.spec().sample_format, SampleFormat::Int);

    let timeline = Timeline::builder(Scene {
        audio: Audio {
            crush: 16.0,
            ..Default::default()
        },
        fractal: Fractal {
            iterations: 1_000,
            color_scale: 0.0,
            // dragon
            cx: -0.835,
            cy: -0.2321,
            ..Default::default()
        },
    })
    // fade in
    .event(
        2.5,
        EaseFunction::QuinticInOut,
        &[fractal(color_scale(1.0)), audio(crush(8.0))],
    )
    // zoom 1
    .event(
        3.5,
        EaseFunction::ExponentialInOut,
        &[
            fractal(view(View {
                x: -0.1070738,
                y: -0.9124549,
                z: 0.001953125,
            })),
            fractal(rotation(std::f32::consts::PI)),
            audio(lpf_cutoff(1_000.0)),
            audio(lpf_q(6.0)),
            audio(crush(16.0)),
        ],
    )
    // zoom out
    .event(
        2.0,
        EaseFunction::ExponentialInOut,
        &[
            fractal(view(View {
                z: 1.25,
                ..Default::default()
            })),
            fractal(rotation(std::f32::consts::TAU)),
            audio(lpf_cutoff(20_000.0)),
            audio(lpf_q(1.0)),
        ],
    )
    // exp out
    .event(
        1.5,
        EaseFunction::CircularIn,
        &[fractal(exponent(12.0)), audio(crush(4.0))],
    )
    .event(
        1.5,
        EaseFunction::CircularOut,
        &[fractal(exponent(22.0)), audio(crush(16.0))],
    )
    // rotate
    .event(
        3.5,
        EaseFunction::SineIn,
        &[
            fractal(rotation(std::f32::consts::TAU * 2.0)),
            fractal(exponent(4.0)),
            fractal(color_scale(10.0)),
            audio(lpf_q(3.0)),
            audio(lpf_cutoff(5_000.0)),
        ],
    )
    .event(
        2.0,
        EaseFunction::SineOut,
        &[
            fractal(rotation(std::f32::consts::TAU * 3.0)),
            fractal(exponent(22.0)),
            fractal(color_scale(1.0)),
            audio(lpf_q(1.0)),
            audio(lpf_cutoff(20_000.0)),
        ],
    )
    // exp in
    .event(3.0, EaseFunction::CubicIn, &[fractal(exponent(2.0))])
    // julia!
    .event(3.5, EaseFunction::CircularOut, &[fractal(julia(1.0))])
    // dendrite
    .event(
        5.0,
        EaseFunction::SineInOut,
        &[fractal(cx(-0.8)), fractal(cy(0.156))],
    )
    .event(
        7.0,
        EaseFunction::SineInOut,
        &[fractal(cx(-0.4)), fractal(cy(0.6))],
    )
    // burning ship!
    .event(3.5, EaseFunction::SineInOut, &[fractal(burning_ship(1.0))])
    .event(
        5.0,
        EaseFunction::SineIn,
        &[
            fractal(rotation(std::f32::consts::TAU * 3.5)),
            fractal(color_scale(4.0)),
            // dragon
            fractal(cx(-0.835)),
            fractal(cy(-0.2321)),
        ],
    )
    .event(
        6.5,
        EaseFunction::SineOut,
        &[
            fractal(rotation(std::f32::consts::TAU * 4.0)),
            fractal(color_scale(0.0)),
            // dendrite
            fractal(cx(-0.8)),
            fractal(cy(0.156)),
        ],
    )
    .build();

    let mut renderer = Renderer::new(
        samples,
        width,
        height,
        &fanim::palette::parse_palette("magma"),
    );
    let mut lpfl = Biquad::default();
    let mut lpfr = Biquad::default();
    let mut time = 0.0;
    encoder.render_event(&mut |encoder: &mut Encoder, dt: f32| {
        if let Some(scene) = timeline.step(time) {
            time += dt;
            renderer.config = scene.fractal;
            renderer.render();
            let pixels = renderer.read_output_buffer();
            encoder.render_frame(&pixels, &mut |sample_time| {
                if let Some(sample_scene) = timeline.step(sample_time) {
                    let mut samples = reader.samples::<i16>();
                    match (samples.next(), samples.next()) {
                        (Some(l), Some(r)) => {
                            let l = l.unwrap() as f32 / 32768.0;
                            let r = r.unwrap() as f32 / 32768.0;
                            (
                                sample_scene.audio.process(sample_rate, &mut lpfl, l),
                                sample_scene.audio.process(sample_rate, &mut lpfr, r),
                            )
                        }
                        _ => (0.0, 0.0),
                    }
                } else {
                    (0.0, 0.0)
                }
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    })?;

    encoder.finish("out.mp4")?;
    std::process::Command::new("open")
        .arg("out.mp4")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    Ok(())
}
