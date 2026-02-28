use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;

fn main() {
    let scale = 240;
    App::default()
        .add_plugins((
            ParamPlugin,
            EncoderPlugin,
            RenderPlugin {
                width: 16 * scale,
                height: 9 * scale,
                super_samples: 2,
            },
        ))
        .add_systems(Startup, spawn_animation)
        .set_runner(fanim::runner)
        .run();
}

fn spawn_animation(mut commands: Commands) {
    commands.spawn((
        ImageEncoder::new("out.png"),
        Params::default(),
        Bulb::default(),
        Iterations(200),
        Exponent(2.0),
        cubehelix_default(),
    ));
}
