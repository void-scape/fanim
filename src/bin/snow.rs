use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;
use std::f32::consts::{PI, TAU};

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");

    let hq = false;
    let (scale, super_samples, fps, buddha, buddha_samples) = if hq {
        (120, 2, 30, 500, 32)
    } else {
        (32, 1, 10, 500, 2)
    };
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
        .add_systems(Startup, spawn_animation(buddha, buddha_samples))
        .set_runner(fanim::runner)
        .run();

    std::process::Command::new("open")
        .arg("out.mp4")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

fn spawn_animation(buddha: u32, buddha_samples: u32) -> impl FnMut(Commands) {
    move |mut commands: Commands| {
        commands.spawn(AudioPlayer::new("assets/snow.mp3"));
        let target = commands
            .spawn(Fractal::default().into_bundle())
            .insert((
                View {
                    x: -0.25,
                    y: 0.0,
                    z: 2.0,
                },
                Mandelbrot(0.0),
                Buddha(1.0),
                Rotation(-PI / 2.0),
                Exponent(0.8),
                BuddhaSamples(buddha_samples),
                RgbIterations {
                    r: buddha,
                    g: buddha * 10,
                    b: buddha * 100,
                },
            ))
            .id();

        commands.spawn(AnimationTarget(target)).insert(animations![
            (
                Keyframe(Exponent(1.35)),
                EaseFunction::SmootherStep,
                Duration(0.9),
            ),
            parallel![
                (
                    Keyframe(Exponent(2.25)),
                    EaseFunction::SineInOut,
                    Duration(1.5),
                ),
                animations![
                    (
                        Keyframe(Rotation(-TAU)),
                        EaseFunction::ExponentialIn,
                        Duration(1.5)
                    ),
                    (
                        Keyframe(Rotation(-TAU * 1.15)),
                        EaseFunction::ExponentialOut,
                        Duration(1.5)
                    ),
                ],
            ],
            (system(fanim::encoder::finish), Duration(0.0))
        ]);
    }
}
