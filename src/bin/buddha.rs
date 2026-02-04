use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");

    let hq = false;
    let (scale, super_samples, fps) = if hq { (160, 2, 60) } else { (32, 1, 10) };
    App::default()
        .add_plugins(fanim::FanimPlugin {
            width: 2000,
            height: 2000,
            // width: 16 * scale,
            // height: 9 * scale,
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

fn spawn_animation(mut commands: Commands) {
    commands.spawn(AudioPlayer::new("assets/bleed.mp3"));
    let target = commands
        .spawn(default_fractal())
        .insert((
            View {
                x: 0.0,
                y: 0.0,
                z: 2.0,
            },
            Mandelbrot(0.0),
            Buddha(1.0),
            Rotation(-std::f32::consts::PI / 2.0),
            Iterations(10_000),
        ))
        .id();

    commands.spawn(AnimationTarget(target)).insert(animations![
        // (Keyframe(Mandelbrot(0.0)), Duration(5.0)),
        Duration(1.0 / 10.0),
        (system(fanim::encoder::finish), Duration(0.0))
    ]);
}
