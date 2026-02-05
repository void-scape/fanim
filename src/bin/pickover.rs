use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;

fn main() {
    let scale = 240;
    App::default()
        .add_plugins(fanim::ImagePlugin {
            width: 16 * scale,
            height: 9 * scale,
            super_samples: 4,
            output_path: "out.png".into(),
        })
        .add_systems(Startup, spawn_fractal)
        .set_runner(fanim::runner)
        .run();
}

fn spawn_fractal(mut commands: Commands) {
    commands.spawn(Fractal::default().into_bundle()).insert((
        View {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        CPlane { x: 1.0, y: 1.0 },
        Julia(1.0),
        Exponent(5.0),
        Pickover(1.0),
        ColorScale(1.0),
        palette::cubehelix_default(),
    ));
}
