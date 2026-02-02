use bevy_app::{Plugin, PreUpdate, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use std::{mem::ManuallyDrop, num::NonZeroU32, path::Path};
use symphonium::SymphoniumLoader;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // NOTE: `Samples` and `SampleRate` are created by the `EncoderPlugin`.
        app.add_systems(PreUpdate, clear).add_systems(
            Update,
            (load_audio, audio_player)
                .chain()
                .in_set(AudioSystem::AudioPlayers),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum AudioSystem {
    AudioPlayers,
    Filters,
    Volume,
}

#[derive(Component, Deref)]
#[component(immutable)]
pub struct SampleRate(pub usize);

#[derive(Component, Deref, DerefMut)]
pub struct Samples(pub Vec<(f32, f32)>);

fn clear(mut samples: Single<&mut Samples>) {
    samples.fill((0.0, 0.0));
}

#[derive(Component)]
pub struct AudioPlayer {
    path: String,
    cursor: usize,
}

impl AudioPlayer {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_string_lossy().to_string(),
            cursor: 0,
        }
    }
}

#[derive(Component, Deref)]
#[component(immutable)]
struct AudioSource(Vec<(f32, f32)>);

fn load_audio(
    mut commands: Commands,
    players: Query<(Entity, &AudioPlayer), Without<AudioSource>>,
    sample_rate: Single<&SampleRate>,
) -> bevy_ecs::error::Result {
    for (entity, player) in players.iter() {
        let mut loader = SymphoniumLoader::new();
        let samples = loader
            .load_f32(
                &player.path,
                Some(NonZeroU32::new(sample_rate.0 as u32).unwrap()),
                Default::default(),
                None,
            )
            .map_err(|e| e.to_string())?;
        let mut interleaved = ManuallyDrop::new(samples.as_interleaved());
        // SAFETY: This might leak some capacity memory if it is not divisible
        // by 2, but otherwise the samples are garaunteed to be interleaved
        // such that casting to a Vec<(f32, f32)> with half the size is correct.
        let samples = unsafe {
            Vec::from_raw_parts(
                interleaved.as_mut_ptr().cast(),
                interleaved.len() / 2,
                interleaved.capacity() / 2,
            )
        };
        commands.entity(entity).insert(AudioSource(samples));
    }
    Ok(())
}

fn audio_player(
    mut players: Query<(&mut AudioPlayer, &AudioSource)>,
    mut samples: Single<&mut Samples>,
) {
    for (mut player, source) in players.iter_mut() {
        for ((l, r), (il, ir)) in samples
            .iter_mut()
            .zip(source[player.cursor.min(source.len())..].iter())
        {
            *l += *il;
            *r += *ir;
            player.cursor += 1;
        }
    }
}

// #[derive(Clone, Copy, PartialEq)]
// pub struct Audio {
//     pub crush: f64,
//     pub lpf_cutoff: f64,
//     pub lpf_q: f64,
//     pub volume: f64,
// }
//
// impl Default for Audio {
//     fn default() -> Self {
//         Self {
//             crush: 16.0,
//             lpf_cutoff: 20_000.0,
//             lpf_q: 1.0,
//             volume: 1.0,
//         }
//     }
// }
//
// impl Audio {
//     pub fn process(&self, sample_rate: usize, lpf: &mut Biquad, s: f32) -> f32 {
//         let s = s as f64;
//         let crushed = if self.crush >= 16.0 {
//             s
//         } else {
//             let levels = 2f64.powf(self.crush);
//             (s * levels).round() / levels
//         };
//         let s = if self.lpf_cutoff < 20_000.0 || self.lpf_q != 1.0 {
//             lpf.sample(sample_rate, crushed, self.lpf_cutoff, self.lpf_q)
//         } else {
//             crushed
//         };
//         (s * self.volume).clamp(-1.0, 1.0) as f32
//     }
// }
//
// #[derive(Default)]
// pub struct Biquad {
//     params: BiquadParams,
//     freq: f64,
//     q: f64,
//     x1: f64,
//     x2: f64,
//     y1: f64,
//     y2: f64,
// }
//
// impl Biquad {
//     fn sample(&mut self, sample_rate: usize, s: f64, cutoff_hz: f64, q: f64) -> f64 {
//         if self.q != q || self.freq != cutoff_hz {
//             self.params = BiquadParams::lpf(cutoff_hz, q, sample_rate as f64);
//         }
//         let params = &self.params;
//
//         let x0 = s;
//         let y0 = params.b0 * x0 + params.b1 * self.x1 + params.b2 * self.x2
//             - params.a1 * self.y1
//             - params.a2 * self.y2;
//         self.x2 = self.x1;
//         self.x1 = x0;
//         self.y2 = self.y1;
//         self.y1 = y0;
//         y0
//     }
// }
//
// /// Implementation based on these resources:
// /// - https://github.com/SamiPerttu/fundsp/blob/master/src/biquad.rs
// /// - https://webaudio.github.io/Audio-EQ-Cookbook/audio-eq-cookbook.html
// #[derive(Default)]
// pub struct BiquadParams {
//     a1: f64,
//     a2: f64,
//     b0: f64,
//     b1: f64,
//     b2: f64,
// }
//
// impl BiquadParams {
//     pub fn lpf(cutoff_hz: f64, q: f64, sample_rate: f64) -> Self {
//         let omega = core::f64::consts::TAU * cutoff_hz / sample_rate;
//         let (osin, ocos) = omega.sin_cos();
//         let alpha = osin / (2.0 * q);
//         let a0 = 1.0 + alpha;
//         let ocosm1 = 1.0 - ocos;
//         let b0 = (ocosm1 / 2.0) / a0;
//         let b1 = ocosm1 / a0;
//         let b2 = b0;
//         let a1 = (-2.0 * ocos) / a0;
//         let a2 = (1.0 - alpha) / a0;
//         Self { a1, a2, b0, b1, b2 }
//     }
// }
