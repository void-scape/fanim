use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use colorgrad::{Color, GradientBuilder};
use fanim::prelude::*;

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");

    let hq = false;
    let (scale, super_samples, fps) = if hq { (160, 2, 60) } else { (32, 1, 10) };
    App::default()
        .add_plugins(fanim::FanimPlugin {
            width: 16 * scale,
            height: 9 * scale,
            super_samples,
            fps,
            sample_rate: 44_100,
            data_path: "data".into(),
            output_path: "out.mp4".into(),
        })
        .add_systems(Startup, spawn_animation)
        .set_runner(fanim::runner)
        .run();

    std::process::Command::new("open")
        .arg("out.mp4")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

// Implementation from fundsp:
// https://github.com/SamiPerttu/fundsp/blob/3f867aa05315d9a029419c7e680879aef0bc24b1/src/dynamics.rs#L336
#[derive(Component)]
struct Peak {
    smoothing: f32,
    state: f32,
}

impl Peak {
    pub fn new(sample_rate: SampleRate, timescale: f32) -> Self {
        Self {
            smoothing: 0.5f32.powf(1.0 / (timescale * *sample_rate as f32)),
            state: 0.0,
        }
    }

    pub fn process(&mut self, sample: f32) {
        self.state = (self.state * self.smoothing).max(sample.abs());
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
    palette::gradient_palette(&blood)
}

fn palette_peak(
    // mut palette: Single<&mut Palette>,
    mut color_scale: Single<&mut ColorScale>,
    mut rotation: Single<&mut Rotation>,
    mut peak: Single<&mut Peak>,
    samples: Single<&Samples>,
) {
    for (l, r) in samples.iter() {
        peak.process((*l + *r) / 2.0);
    }
    // let p1 = palette::magma();
    // let p2 = palette::cubehelix_default();
    // **palette = p1.lerp(&p2, (peak.state * 5.0).clamp(0.0, 1.0));
    ***color_scale = peak.state * 3.0;
    ***rotation += peak.state / 5.0;
}

fn spawn_animation(mut commands: Commands, sample_rate: Single<&SampleRate>) {
    commands.spawn(AudioPlayer::new("assets/bleed.mp3"));
    commands.spawn(Peak::new(**sample_rate, 1.0));
    let target = commands
        .spawn(default_fractal())
        .insert((
            blood_red(),
            BurningShip(1.0),
            Exponent(3.0),
            View {
                x: 0.0,
                y: 0.0,
                z: 8.25,
            },
            CPlane { x: -0.4, y: 0.6 },
        ))
        .id();

    commands.spawn(AnimationTarget(target)).insert(animations![
        parallel![
            (system(palette_peak), Duration(20.0)),
            animations![
                (
                    Keyframe(View {
                        x: 0.0,
                        y: 0.0,
                        z: 1.25,
                    }),
                    Keyframe(Exponent(5.0)),
                    EaseFunction::SineInOut,
                    Duration(9.0)
                ),
                (
                    Keyframe(Julia(1.0)),
                    Keyframe(BurningShip(0.0)),
                    Keyframe(Exponent(4.0)),
                    EaseFunction::ExponentialInOut,
                    Duration(2.0)
                ),
                (
                    Delta(CPlane { x: 0.15, y: 0.0 }),
                    EaseFunction::SineInOut,
                    Duration(6.0)
                )
            ]
        ],
        (system(fanim::encoder::finish), Duration(0.0))
    ]);
}
