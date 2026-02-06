use bevy_app::{Plugin, PostStartup, PostUpdate, PreStartup};
use bevy_ecs::prelude::*;

use crate::{
    encoder::EncodingTarget,
    params::{Palette, Params},
};
pub use output::OutputBuffer;

mod compute;
mod output;
mod ssaa;

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

        app.add_systems(PreStartup, spawn_renderer)
            .add_systems(
                PostStartup,
                (output::spawn, ssaa::spawn, compute::spawn).chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    rerender,
                    compute::compute_pass.in_set(RenderSystems::Compute),
                    ssaa::render_pass.in_set(RenderSystems::Render),
                    output::map_output.in_set(RenderSystems::MapOutput),
                )
                    .chain(),
            );

        app.configure_sets(
            PostUpdate,
            (
                RenderSystems::Compute.before(RenderSystems::Render),
                RenderSystems::Render.before(RenderSystems::MapOutput),
            ),
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum RenderSystems {
    Compute,
    Render,
    MapOutput,
}

#[derive(Default, Component)]
pub struct Rerender;

fn rerender(
    mut commands: Commands,
    params: Query<(Entity, Ref<Params>, Ref<Palette>), With<EncodingTarget>>,
) {
    for (entity, params, palette) in params.iter() {
        if params.is_changed() || palette.is_changed() {
            commands.entity(entity).insert(Rerender);
        }
    }
}

#[derive(Component)]
#[component(immutable)]
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub width: usize,
    pub height: usize,
    pub super_samples: usize,
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
        .expect("failed to create render device");

        Self {
            device,
            queue,
            width,
            height,
            super_samples,
        }
    }
}
