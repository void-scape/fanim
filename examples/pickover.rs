use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;

fn main() {
    let scale = 200;
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
        .add_systems(Startup, spawn_fractal)
        .set_runner(fanim::runner)
        .run();
}

fn spawn_fractal(mut commands: Commands) {
    commands.spawn((
        CollageEncoder::new("out.png", "data"),
        children![
            (
                Params::default(),
                Mandelbrot::default(),
                View {
                    x: 0.0,
                    y: 0.0,
                    z: 1.5,
                },
                Pickover(1.0),
                inferno(),
            ),
            (
                Params::default(),
                Mandelbrot::default(),
                View {
                    x: 0.0,
                    y: 0.0,
                    z: 1.5,
                },
                CPlane {
                    x: -0.752,
                    y: -1.131
                },
                Julia(1.0),
                Exponent(3.0),
                Pickover(0.8),
                ColorScale(1.5),
                cubehelix_default(),
            )
        ],
    ));
}
