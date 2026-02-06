use bevy_app::{Plugin, PreUpdate, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use fanim_macros::Lerp;
use std::{f32::consts::PI, mem::ManuallyDrop, num::NonZeroU32, path::Path};
use symphonium::SymphoniumLoader;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // NOTE: `Samples` and `SampleRate` are created by `VideoEncoder`s.
        app.add_systems(PreUpdate, clear).add_systems(
            Update,
            (
                (load_audio, audio_player)
                    .chain()
                    .in_set(AudioSystems::AudioPlayers),
                volume.in_set(AudioSystems::Volume),
            ),
        );

        app.configure_sets(
            Update,
            (
                AudioSystems::Volume.after(AudioSystems::Filters),
                AudioSystems::AudioPlayers.before(AudioSystems::Filters),
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum AudioSystems {
    AudioPlayers,
    Filters,
    Volume,
}

#[derive(Clone, Copy, Component, Deref)]
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

#[derive(Clone, Copy, Lerp, Component, Deref, DerefMut)]
pub struct Volume(pub f32);

fn volume(mut samples: Single<&mut Samples>, volume: Single<&Volume>) {
    for (l, r) in samples.iter_mut() {
        *l *= volume.0;
        *r *= volume.0;
    }
}

// Implementation taken from the lovely DaisySP:
// https://github.com/electro-smith/DaisySP/blob/master/Source/Filters/onepole.h
#[derive(Component)]
pub struct LowPass {
    g: f32,
    gi: f32,
    state: f32,
}

impl LowPass {
    pub fn new(freq: f32, sample_rate: SampleRate) -> Self {
        let mut slf = Self {
            g: 0.0,
            gi: 0.0,
            state: 0.0,
        };
        slf.set_freq(freq, sample_rate);
        slf
    }

    pub fn set_freq(&mut self, freq: f32, sample_rate: SampleRate) {
        let clipped_freq = (freq / sample_rate.0 as f32).clamp(0.0, 0.497);
        self.g = (PI * clipped_freq).tan();
        self.gi = 1.0 / (1.0 + self.g);
    }

    pub fn process(&mut self, sample: f32) -> f32 {
        let lp = (self.g * sample + self.state) * self.gi;
        self.state = self.g * (sample - lp) + lp;
        lp
    }
}
