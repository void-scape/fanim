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
                super_samples: 1,
            },
        ))
        .add_systems(Startup, spawn_fractal)
        .set_runner(fanim::runner)
        .run();
}

fn spawn_fractal(mut commands: Commands) {
    commands
        .spawn(CollageEncoder::new("out.png", "data"))
        .with_children(|s| {
            s.spawn((
                Params::default(),
                View {
                    x: 0.0,
                    y: 0.0,
                    z: 1.5,
                },
                CPlane { x: 1.0, y: 1.0 },
                Julia(1.0),
                Exponent(4.0),
                Pickover(1.0),
                ColorScale(1.0),
                cubehelix_default(),
            ));

            s.spawn((
                Params::default(),
                View {
                    x: 0.0,
                    y: 0.0,
                    z: 10.5,
                },
                CPlane { x: 1.0, y: 1.0 },
                Julia(1.0),
                Exponent(3.0),
                Pickover(0.5),
                ColorScale(2.0),
            ));
        });
}
