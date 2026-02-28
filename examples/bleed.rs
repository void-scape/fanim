use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use colorgrad::{Color, GradientBuilder};
use fanim::prelude::*;

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");
    let scale = 32;
    App::default()
        .add_plugins((
            AudioPlugin,
            ParamPlugin,
            EncoderPlugin,
            AnimationPlugin,
            RenderPlugin {
                width: 16 * scale,
                height: 9 * scale,
                super_samples: 1,
            },
        ))
        .add_systems(Startup, spawn_animation(30))
        .set_runner(fanim::runner)
        .run();
}

#[derive(Default, Component)]
struct Rms {
    smoothing: f32,
    state: f32,
}

impl Rms {
    pub fn new(sample_rate: SampleRate, timescale: f32) -> Self {
        Self {
            smoothing: 0.5f32.powf(1.0 / (timescale * *sample_rate as f32)),
            state: 0.0,
        }
    }

    pub fn process(&mut self, sample: f32) {
        self.state = self.state * self.smoothing + sample * sample * (1.0 - self.smoothing);
    }

    pub fn sample(&self) -> f32 {
        self.state.sqrt()
    }
}

#[derive(Default, Component)]
struct Peak {
    max: f32,
}

impl Peak {
    pub fn process(&mut self, sample: f32) {
        self.max = self.max.max(sample.abs());
    }
}

pub fn blood_red() -> Palette {
    let blood = GradientBuilder::new()
        .colors(&[
            Color::from_rgba8(0, 0, 0, 255),
            Color::from_rgba8(20, 0, 0, 255),
            Color::from_rgba8(60, 0, 0, 255),
            Color::from_rgba8(120, 0, 0, 255),
            Color::from_rgba8(180, 10, 10, 255),
            Color::from_rgba8(220, 40, 40, 255),
            Color::from_rgba8(255, 80, 80, 255),
        ])
        .build::<colorgrad::LinearGradient>()
        .unwrap();
    gradient_palette(&blood)
}

fn color_scale_rms(
    mut color_scale: Single<&mut ColorScale>,
    mut rms: Single<&mut Rms>,
    samples: Single<&Samples>,
) {
    for (l, r) in samples.iter() {
        rms.process((*l + *r) / 2.0);
    }
    ***color_scale = rms.sample() * 50.0;
}

fn palette_peak(
    mut rotation: Single<&mut Rotation>,
    peak: Single<(&mut Peak, &mut LowPass)>,
    samples: Single<&Samples>,
    delta: Single<&DeltaTime>,
) {
    let (mut peak, mut peak_lp) = peak.into_inner();
    for (l, r) in samples.iter() {
        peak.process((*l + *r) / 2.0);
    }
    let peak = peak_lp.process(peak.max);
    ***rotation += peak * ***delta;
}

fn spawn_animation(fps: usize) -> impl FnMut(Commands) -> bevy_ecs::error::Result {
    move |mut commands: Commands| {
        commands.spawn(AudioPlayer::new("assets/bleed.mp3"));
        commands.spawn(Rms::new(SampleRate(44_100), 0.01));
        commands.spawn((Peak::default(), LowPass::new(100.0, SampleRate(44_100))));
        let target = commands
            .spawn((
                VideoEncoder::new("out.mp4", "data", 44_100, fps)?,
                Params::default(),
                Mandelbrot::default(),
                blood_red(),
                BurningShip(1.0),
                Exponent(3.0),
                Iterations(10_000),
                View {
                    x: 0.0,
                    y: 0.0,
                    z: 16.0,
                },
                CPlane { x: -0.4, y: 0.6 },
            ))
            .id();

        commands
            .spawn(AnimationTarget(target))
            .insert(animations![parallel![
                (system(color_scale_rms), Duration(10.0)),
                (system(palette_peak), Duration(40.0)),
                animations![
                    (
                        Keyframe(View {
                            x: 0.0,
                            y: 0.0,
                            z: 2.0,
                        }),
                        EaseFunction::SineIn,
                        Duration(9.0)
                    ),
                    (
                        Keyframe(View {
                            x: 0.0,
                            y: 0.0,
                            z: 1.25,
                        }),
                        EaseFunction::SineOut,
                        Duration(2.0)
                    ),
                ],
                animations![
                    (
                        Keyframe(Exponent(5.0)),
                        EaseFunction::SineInOut,
                        Duration(9.0)
                    ),
                    (
                        Keyframe(Julia(1.0)),
                        Keyframe(ColorRotation(0.25)),
                        Keyframe(Pickover(1.0)),
                        // Keyframe(BurningShip(0.0)),
                        Keyframe(Exponent(4.0)),
                        Keyframe(ColorScale(1.0)),
                        EaseFunction::ExponentialInOut,
                        Duration(2.0)
                    ),
                    (
                        Keyframe(Exponent(2.0)),
                        EaseFunction::SineInOut,
                        Duration(6.0)
                    ),
                    (
                        Keyframe(CPlane { x: -1.729, y: 0.0 }),
                        EaseFunction::SineInOut,
                        Duration(6.0)
                    ),
                    (
                        Keyframe(CPlane {
                            x: -0.752,
                            y: -1.131
                        }),
                        EaseFunction::SineInOut,
                        Duration(6.0)
                    ),
                    (
                        Keyframe(Exponent(4.0)),
                        Keyframe(ColorScale(2.0)),
                        EaseFunction::SineInOut,
                        Duration(6.0)
                    )
                ]
            ]]);
        Ok(())
    }
}
