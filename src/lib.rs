#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use bevy_app::{App, AppExit};

pub mod animation;
pub mod audio;
pub mod encoder;
pub mod render;

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

pub fn runner(mut app: App) -> AppExit {
    loop {
        app.update();
        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }
}
