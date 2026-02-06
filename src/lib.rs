#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy_app::{App, AppExit};

pub mod animation;
pub mod audio;
pub mod encoder;
pub mod params;
pub mod render;

pub mod prelude {
    pub use super::animation::*;
    pub use super::audio::*;
    pub use super::encoder::*;
    pub use super::params::*;
    pub use super::render::*;
    pub use super::{animations, parallel};
}

// pub struct ImagePlugin {
//     pub width: usize,
//     pub height: usize,
//     pub super_samples: usize,
//     pub output_path: String,
// }
//
// impl Plugin for ImagePlugin {
//     fn build(&self, app: &mut App) {
//         let output = self.output_path.clone();
//         app.add_plugins((
//             render::RenderPlugin {
//                 width: self.width,
//                 height: self.height,
//                 super_samples: self.super_samples,
//             },
//         ))
//         .add_systems(Startup, move |mut commands: Commands| {
//             commands.spawn(image::ImageOutput(output.clone()));
//         })
//         .add_systems(Last, image::finish);
//     }
// }
//
// pub struct VideoPlugin {
//     pub width: usize,
//     pub height: usize,
//     pub super_samples: usize,
//     pub fps: usize,
//     pub sample_rate: usize,
//     pub data_path: String,
//     pub output_path: String,
// }
//
// impl Plugin for VideoPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_plugins((
//             audio::AudioPlugin,
//             animation::AnimationPlugin,
//             encoder::EncoderPlugin {
//                 fps: self.fps,
//                 sample_rate: self.sample_rate,
//                 data_path: self.data_path.clone(),
//                 output_path: self.output_path.clone(),
//             },
//             render::RenderPlugin {
//                 width: self.width,
//                 height: self.height,
//                 super_samples: self.super_samples,
//             },
//         ));
//     }
// }

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

/// Default runner for `fanim`.
pub fn runner(mut app: App) -> AppExit {
    loop {
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
}
