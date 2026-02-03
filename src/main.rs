use bevy_app::{App, Startup};
use bevy_ecs::prelude::*;
use colorgrad::{Color, GradientBuilder};
use fanim::prelude::*;
use rustfft::{FftPlanner, num_complex::Complex};

fn main() {
    _ = std::fs::remove_dir_all("data");
    _ = std::fs::create_dir_all("data");

    let hq = false;
    let (scale, super_samples, fps) = if hq { (160, 2, 60) } else { (32, 1, 10) };
    App::default()
        .add_plugins(fanim::FanimPlugin {
            width: 16 * scale,
            height: 9 * scale,
            super_samples,
            fps,
            sample_rate: 44_100,
            data_path: "data".into(),
            output_path: "out.mp4".into(),
        })
        .add_systems(Startup, spawn_animation)
        .set_runner(fanim::runner)
        .run();

    std::process::Command::new("open")
        .arg("out.mp4")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}

#[derive(Default, Component)]
struct Rms {
    smoothing: f32,
    state: f32,
}

impl Rms {
    pub fn new(sample_rate: SampleRate, timescale: f32) -> Self {
        Self {
            smoothing: 0.5f32.powf(1.0 / (timescale * *sample_rate as f32)),
            state: 0.0,
        }
    }

    pub fn process(&mut self, sample: f32) {
        self.state = self.state * self.smoothing + sample * sample * (1.0 - self.smoothing);
    }

    pub fn sample(&self) -> f32 {
        self.state.sqrt()
    }
}

#[derive(Default, Component)]
struct Peak {
    max: f32,
}

impl Peak {
    pub fn process(&mut self, sample: f32) {
        self.max = self.max.max(sample.abs());
    }
}

#[derive(Component)]
struct FrequencyAnalyzer {
    planner: FftPlanner<f32>,
    buffer: Vec<Complex<f32>>,
    bass: f32,
    mid: f32,
    high: f32,
}

impl FrequencyAnalyzer {
    pub fn new(size: usize) -> Self {
        Self {
            planner: FftPlanner::new(),
            buffer: vec![Complex::new(0.0, 0.0); size],
            bass: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }

    pub fn process(&mut self, samples: &[(f32, f32)]) {
        for (i, (l, r)) in samples.iter().enumerate().take(self.buffer.len()) {
            let mono = (l + r) / 2.0;
            self.buffer[i] = Complex::new(mono, 0.0);
        }
        for i in samples.len()..self.buffer.len() {
            self.buffer[i] = Complex::new(0.0, 0.0);
        }
        let fft = self.planner.plan_fft_forward(self.buffer.len());
        fft.process(&mut self.buffer);
        let bass_end = self.buffer.len() / 16;
        let mid_end = self.buffer.len() / 3;
        self.bass = self.band_energy(0, bass_end);
        self.mid = self.band_energy(bass_end, mid_end);
        self.high = self.band_energy(mid_end, self.buffer.len() / 2);
    }

    fn band_energy(&self, start: usize, end: usize) -> f32 {
        let sum = self.buffer[start..end]
            .iter()
            .map(|c| c.norm())
            .sum::<f32>();
        sum / (end - start) as f32
    }
}

pub fn blood_red() -> Palette {
    let blood = GradientBuilder::new()
        .colors(&[
            Color::from_rgba8(0, 0, 0, 255),
            Color::from_rgba8(20, 0, 0, 255),
            Color::from_rgba8(60, 0, 0, 255),
            Color::from_rgba8(120, 0, 0, 255),
            Color::from_rgba8(180, 10, 10, 255),
            Color::from_rgba8(220, 40, 40, 255),
            Color::from_rgba8(255, 80, 80, 255),
        ])
        .build::<colorgrad::LinearGradient>()
        .unwrap();
    palette::gradient_palette(&blood)
}

fn color_scale_rms(
    mut color_scale: Single<&mut ColorScale>,
    mut rms: Single<&mut Rms>,
    samples: Single<&Samples>,
) {
    for (l, r) in samples.iter() {
        rms.process((*l + *r) / 2.0);
    }
    ***color_scale = rms.sample() * 10.0;
}

fn palette_peak(
    mut palette: Single<&mut Palette>,
    mut rotation: Single<&mut Rotation>,
    analyzer: Single<(&mut FrequencyAnalyzer, &mut LowPass)>,
    peak: Single<(&mut Peak, &mut LowPass), Without<FrequencyAnalyzer>>,
    samples: Single<&Samples>,
    delta: Single<&DeltaTime>,
) {
    let (mut analyzer, mut analyzer_lp) = analyzer.into_inner();
    analyzer.process(samples.as_slice());
    let p1 = blood_red();
    let p2 = palette::cubehelix_default();
    **palette = p1.lerp(&p2, analyzer_lp.process(analyzer.high.clamp(0.0, 1.0)));

    let (mut peak, mut peak_lp) = peak.into_inner();
    for (l, r) in samples.iter() {
        peak.process((*l + *r) / 2.0);
    }
    let peak = peak_lp.process(peak.max);
    // ***color_scale = peak * 3.0;
    ***rotation += peak * ***delta;
}

fn spawn_animation(mut commands: Commands, sample_rate: Single<&SampleRate>) {
    commands.spawn(AudioPlayer::new("assets/bleed.mp3"));
    commands.spawn((
        FrequencyAnalyzer::new(2048),
        LowPass::new(100.0, **sample_rate),
    ));
    commands.spawn(Rms::new(**sample_rate, 0.1));
    commands.spawn((Peak::default(), LowPass::new(100.0, **sample_rate)));
    let target = commands
        .spawn(default_fractal())
        .insert((
            blood_red(),
            BurningShip(1.0),
            Exponent(3.0),
            View {
                x: 0.0,
                y: 0.0,
                z: 8.25,
            },
            CPlane { x: -0.4, y: 0.6 },
        ))
        .id();

    commands.spawn(AnimationTarget(target)).insert(animations![
        parallel![
            (system(color_scale_rms), Duration(28.0)),
            (system(palette_peak), Duration(28.0)),
            animations![
                (
                    Keyframe(View {
                        x: 0.0,
                        y: 0.0,
                        z: 1.25,
                    }),
                    Keyframe(Exponent(5.0)),
                    EaseFunction::SineInOut,
                    Duration(9.0)
                ),
                (
                    Keyframe(Julia(1.0)),
                    Keyframe(BurningShip(0.0)),
                    Keyframe(Exponent(4.0)),
                    // Keyframe(ColorScale(1.0)),
                    EaseFunction::ExponentialInOut,
                    Duration(2.0)
                ),
                (
                    Keyframe(Exponent(3.0)),
                    EaseFunction::SineInOut,
                    Duration(6.0)
                ),
                (
                    Keyframe(CPlane {
                        x: -0.7269,
                        y: 0.1889,
                    }),
                    // Keyframe(Exponent(2.5)),
                    EaseFunction::SineInOut,
                    Duration(6.0)
                )
            ]
        ],
        (system(fanim::encoder::finish), Duration(0.0))
    ]);
}
