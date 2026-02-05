use crate::animation::{Lerp, LogF32};
use bevy_app::{Plugin, PostStartup, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use tint::{Color, Srgb};

mod compute;
pub mod palette;
pub mod ssaa;

pub struct RenderPlugin {
    pub width: usize,
    pub height: usize,
    pub super_samples: usize,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let width = self.width;
        let height = self.height;
        let super_samples = self.super_samples;
        let spawn_renderer = move |mut commands: Commands| {
            commands.spawn(Renderer::new(width, height, super_samples));
        };

        app.add_systems(
            PostStartup,
            (spawn_renderer, palette::spawn, ssaa::spawn, compute::spawn).chain(),
        )
        .add_systems(
            PostUpdate,
            (
                palette::write_texture,
                compute::compute_pass,
                ssaa::render_pass,
            )
                .chain(),
        );
    }
}

macro_rules! fractal_component {
    ($name:ident, $type:ty) => {
        crate::lerp_newtype! {
            #[derive(Debug, Clone, Copy, Component, Deref, DerefMut)]
            pub struct $name(pub $type);
        }
    };
}

fractal_component!(Iterations, u32);
fractal_component!(EscapeRadius, f32);
fractal_component!(ColorScale, f32);
fractal_component!(Exponent, f32);
fractal_component!(Rotation, f32);
fractal_component!(Julia, f32);
fractal_component!(BurningShip, f32);
fractal_component!(Mandelbrot, f32);
fractal_component!(Buddha, f32);
fractal_component!(BuddhaSamples, u32);
fractal_component!(ColorRotation, f32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct View {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for View {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 2.0,
        }
    }
}

impl Lerp for View {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        let z = LogF32(self.z).lerp(&LogF32(rhs.z), t).0;
        let factor = if (rhs.z - self.z).abs() < f32::EPSILON {
            t
        } else {
            (z - self.z) / (rhs.z - self.z)
        };
        Self {
            x: self.x.lerp(&rhs.x, factor),
            y: self.y.lerp(&rhs.y, factor),
            z,
        }
    }
}

impl std::ops::Add for View {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
pub struct CPlane {
    pub x: f32,
    pub y: f32,
}

impl Lerp for CPlane {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            x: self.x.lerp(&rhs.x, t),
            y: self.y.lerp(&rhs.y, t),
        }
    }
}

impl std::ops::Add for CPlane {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
pub struct ZPlane {
    pub x: f32,
    pub y: f32,
}

impl Lerp for ZPlane {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            x: self.x.lerp(&rhs.x, t),
            y: self.y.lerp(&rhs.y, t),
        }
    }
}

impl std::ops::Add for ZPlane {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

#[derive(Clone, Component, Deref, DerefMut)]
pub struct Palette(pub [Srgb; 32]);

impl Lerp for Palette {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        let mut out = self.0;
        for (rhs, lhs) in out.iter_mut().zip(rhs.0) {
            *rhs = (rhs.to_linear() * (1.0 - t) + lhs.to_linear() * t).to_srgb();
        }
        Self(out)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct RgbIterations {
    pub r: u32,
    pub g: u32,
    pub b: u32,
}

impl Default for RgbIterations {
    fn default() -> Self {
        Self {
            r: 1_000,
            g: 1_000,
            b: 1_000,
        }
    }
}

impl Lerp for RgbIterations {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            r: self.r.lerp(&rhs.r, t),
            g: self.g.lerp(&rhs.g, t),
            b: self.b.lerp(&rhs.b, t),
        }
    }
}

impl std::ops::Add for RgbIterations {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fractal {
    iterations: u32,
    escape_radius: f32,
    color_scale: f32,
    exponent: f32,
    view: View,
    rotation: f32,
    julia: f32,
    burning_ship: f32,
    c: CPlane,
    z: ZPlane,
    color_rotation: f32,
    buddha_samples: u32,
    rgb_iterations: RgbIterations,
}

impl Default for Fractal {
    fn default() -> Self {
        Self {
            iterations: 1_000,
            escape_radius: 100.0,
            color_scale: 1.0,
            exponent: 2.0,
            view: View::default(),
            rotation: 0.0,
            julia: 0.0,
            burning_ship: 0.0,
            c: CPlane::default(),
            z: ZPlane::default(),
            color_rotation: 0.0,
            buddha_samples: 32,
            rgb_iterations: RgbIterations::default(),
        }
    }
}

impl Fractal {
    pub fn into_bundle(self) -> impl Bundle {
        let Self {
            iterations,
            escape_radius,
            color_scale,
            exponent,
            view,
            rotation,
            julia,
            burning_ship,
            c,
            z,
            color_rotation,
            buddha_samples,
            rgb_iterations,
        } = self;

        (
            (
                Iterations(iterations),
                EscapeRadius(escape_radius),
                Exponent(exponent),
            ),
            (view, Rotation(rotation)),
            (
                ColorScale(color_scale),
                palette::magma(),
                rgb_iterations,
                ColorRotation(color_rotation),
            ),
            (Julia(julia), BurningShip(burning_ship), c, z),
            (Mandelbrot(1.0), Buddha(0.0), BuddhaSamples(buddha_samples)),
        )
    }
}

#[derive(Component)]
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub width: usize,
    pub height: usize,
    pub super_samples: usize,
    pub output_buffer: wgpu::Buffer,
    pub bytes_per_row: usize,
}

impl Renderer {
    pub fn new(width: usize, height: usize, super_samples: usize) -> Self {
        env_logger::init();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        println!("[ADAPTER] {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .expect("Failed to create device");

        let (bytes_per_row, buffer_size) = output_buffer_bytes_per_row_and_size(width, height);
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            width,
            height,
            super_samples,
            output_buffer,
            bytes_per_row,
        }
    }
}

fn output_buffer_bytes_per_row_and_size(width: usize, height: usize) -> (usize, usize) {
    let bytes_per_pixel = 4;
    let align = 256;
    let bpr = width * bytes_per_pixel;
    let padding = (align - bpr % align) % align;
    let bpr = bpr + padding;
    let buffer_size = bpr * height;
    (bpr, buffer_size)
}
