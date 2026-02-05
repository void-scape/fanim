use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");

    let hq = false;
    let (scale, super_samples, fps) = if hq { (240, 2, 60) } else { (32, 1, 10) };
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

fn spawn_animation(mut commands: Commands) {
    commands.spawn(AudioPlayer::new("assets/snow.mp3"));
    let target = commands
        .spawn(Fractal::default().into_bundle())
        .insert((
            View {
                x: 0.0,
                y: 0.0,
                z: 1.25,
            },
            CPlane {
                x: -0.752,
                y: -1.131,
            },
            Julia(1.0),
        ))
        .id();

    commands.spawn(AnimationTarget(target)).insert(animations![
        (
            Keyframe(Pickover(1.0)),
            EaseFunction::SmootherStep,
            Duration(5.0)
        ),
        (system(fanim::encoder::finish), Duration(0.0))
    ]);
}
