use bevy_app::{App, PostStartup, Startup};
use bevy_ecs::prelude::*;
use fanim::animation::{Active, AnimationSystems};

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");
    let scale = 16;

    App::default()
        .add_plugins((
            fanim::animation::AnimationPlugin,
            fanim::encoder::EncoderPlugin {
                fps: 10,
                sample_rate: 44_100,
                data_path: "data".into(),
                output_path: "out.mp4".into(),
            },
            fanim::render::RenderPlugin {
                width: 16 * scale,
                height: 9 * scale,
                samples: 2,
            },
        ))
        .add_systems(Startup, animation.before(AnimationSystems::Startup))
        .add_systems(PostStartup, start_animation)
        .set_runner(fanim::runner)
        .run();

    std::process::Command::new("open")
        .arg("out.mp4")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    // let mut reader = hound::WavReader::open("assets/foreign.wav").unwrap();
    // assert_eq!(reader.spec().sample_rate, sample_rate as u32);
    // assert_eq!(reader.spec().channels, 2);
    // assert_eq!(reader.spec().sample_format, SampleFormat::Int);
    //
    // let mut renderer = Renderer::new(
    //     samples,
    //     width,
    //     height,
    //     &fanim::palette::parse_palette("magma"),
    // );
    // let mut lpfl = Biquad::default();
    // let mut lpfr = Biquad::default();
    // let mut time = 0.0;
    // encoder.render_event(&mut |encoder: &mut Encoder, dt: f32| {
    //     if let Some(scene) = timeline.step(time) {
    //         time += dt;
    //         renderer.config = scene.fractal;
    //         renderer.render();
    //         let pixels = renderer.read_output_buffer();
    //         encoder.render_frame(&pixels, &mut |sample_time| {
    //             if let Some(sample_scene) = timeline.step(sample_time) {
    //                 let mut samples = reader.samples::<i16>();
    //                 match (samples.next(), samples.next()) {
    //                     (Some(l), Some(r)) => {
    //                         let l = l.unwrap() as f32 / 32768.0;
    //                         let r = r.unwrap() as f32 / 32768.0;
    //                         (
    //                             sample_scene.audio.process(sample_rate, &mut lpfl, l),
    //                             sample_scene.audio.process(sample_rate, &mut lpfr, r),
    //                         )
    //                     }
    //                     _ => (0.0, 0.0),
    //                 }
    //             } else {
    //                 (0.0, 0.0)
    //             }
    //         })?;
    //         Ok(true)
    //     } else {
    //         Ok(false)
    //     }
    // })?;
}

#[derive(Component)]
struct Animation;

// annoying hack due to animation target not being propagated until after `PostStartup`
fn start_animation(mut commands: Commands, animation: Single<Entity, With<Animation>>) {
    commands.entity(*animation).insert(Active);
}

fn animation(mut commands: Commands) {
    use fanim::animation::*;
    use fanim::animations;
    use fanim::render::*;

    commands
        .spawn((Animation, AnimationTarget::entity(), default_fractal()))
        .insert(View {
            x: 0.0,
            y: 0.0,
            z: 1.25,
        })
        .insert(animations![
            (
                Keyframe(Exponent(3.0)),
                EaseFunction::SineInOut,
                Duration(5.0)
            ),
            (system(fanim::encoder::finish), Duration(0.0))
        ]);
}
