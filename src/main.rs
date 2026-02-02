use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;
use std::f32::consts::PI;

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");
    let scale = 64;

    App::default()
        .add_plugins(fanim::FanimPlugin {
            width: 16 * scale,
            height: 9 * scale,
            super_samples: 1,
            fps: 10,
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

fn spawn_animation(mut commands: Commands) {
    commands.spawn(AudioPlayer::new("assets/foreign.wav"));
    let target = commands
        .spawn(default_fractal())
        .insert((
            View {
                x: 0.0,
                y: 0.0,
                z: 1.25,
            },
            CPlane { x: -0.8, y: 0.156 },
        ))
        .id();

    commands.spawn(AnimationTarget(target)).insert(animations![
        parallel![
            (
                Keyframe(Exponent(4.0)),
                Keyframe(Palette(palette::cubehelix_default())),
                Keyframe(View {
                    x: -0.25,
                    y: 0.0,
                    z: 1.25
                }),
                EaseFunction::SineInOut,
                Duration(5.0)
            ),
            animations![
                (Keyframe(Rotation(PI)), EaseFunction::CubicIn, Duration(3.5)),
                (
                    Keyframe(Rotation(PI * 1.5)),
                    EaseFunction::CubicOut,
                    Duration(2.0)
                )
            ],
            animations![
                Duration(3.5),
                (Keyframe(Julia(1.0)), EaseFunction::SineInOut, Duration(4.0)),
            ],
            animations![
                Duration(5.0),
                (
                    Keyframe(Palette(palette::viridis())),
                    Keyframe(View {
                        x: 0.0,
                        y: 0.0,
                        z: 1.25
                    }),
                    EaseFunction::SineInOut,
                    Duration(2.0),
                )
            ]
        ],
        (
            Keyframe(CPlane { x: -0.81, y: 0.166 }),
            EaseFunction::SineInOut,
            Duration(2.0)
        ),
        (system(fanim::encoder::finish), Duration(0.0))
    ]);
}
