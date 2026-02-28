use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;
use std::f32::consts::PI;

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
                // NOTE: super_samples does nothing for `Buddha`.
                super_samples: 1,
            },
        ))
        .add_systems(Startup, spawn_animation(30, 200, 8))
        .set_runner(fanim::runner)
        .run();
}

fn spawn_animation(
    fps: usize,
    buddha: u32,
    buddha_samples: u32,
) -> impl FnMut(Commands) -> bevy_ecs::error::Result {
    move |mut commands: Commands| {
        commands.spawn(AudioPlayer::new("assets/snow.mp3"));
        commands.spawn((
            VideoEncoder::new("out.mp4", "data", 44_100, fps)?,
            Params::default(),
            Buddha::default(),
            View {
                x: -0.25,
                y: 0.0,
                z: 2.0,
            },
            Rotation(-PI / 2.0),
            BuddhaSamples(buddha_samples),
            RgbIterations {
                r: buddha,
                g: buddha * 10,
                b: buddha * 100,
            },
            AnimationTarget::entity(),
            animations![(
                Keyframe(Julia(1.0)),
                Keyframe(CPlane { x: 1.0, y: 0.0 }),
                Duration(5.0)
            )],
        ));
        Ok(())
    }
}
