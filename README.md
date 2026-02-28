# fanim

Expressive fractal renderer. `fanim` relies on the [Bevy](https://bevy.org/) engine's entity hierarchy for rendering images and videos.

If you render something cool with `fanim`, feel free to share it in the [Bevy Discord](https://discord.com/invite/bevy)!

## Getting Started

### Images

There are three components required for rendering an image:
- `ImageEncoder` - Renders the entity as an image to the specified path.
- `Params` - Populates the entity with the default parameters.
- `Mandelbrot`/`Buddha`/`Bulb` - The kind of fractal you want to render.

See [pickover.rs](examples/pickover.rs) and [bulb.rs](examples/bulb.rs) for examples.

```rust
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
        Mandelbrot::default(),
        View {
            x: 0.0,
            y: 0.0,
            z: 1.5,
        },
        Pickover(1.0),
        inferno(),
    ));
}
```

![Pickover Image Render](assets/pickover.png)

### Video

There are five components required for rendering a video:
- `VideoEncoder` - Renders the entity as an image to the specified path.
- `Params` - Populates the entity with the default parameters.
- `Mandelbrot`/`Buddha`/`Bulb` - The kind of fractal you want to render.
- `AnimationTarget` - Tells `animations!` what entity to animate.
- `animations!` - Describes an animation. When finished, the `VideoEncoder` uses `ffmpeg` to render a video.

See [bleed.rs](examples/bleed.rs) and [snow.rs](examples/snow.rs) for examples.

```rust
use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use fanim::prelude::*;
use std::f32::consts::PI;

fn main() {
    let scale = 32;
    App::default()
        .add_plugins((
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
```

![Snow Video Render](assets/snow.gif)
