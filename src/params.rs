use crate::{
    prelude::{Lerp, LogF32},
    render::RenderSystems,
};
use bevy_app::{Plugin, PostUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use fanim_macros::{Add, Lerp, Param};
use tint::{Color, Srgb};

pub struct ParamPlugin;

impl Plugin for ParamPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PostUpdate,
            (
                // mandelbrot family
                ColorScale::system,
                ColorRotation::system,
                Pickover::system,
                // buddha
                BuddhaSamples::system,
                RgbIterations::system,
                // shared
                Iterations::system,
                EscapeRadius::system,
                Exponent::system,
                Rotation::system,
                Julia::system,
                BurningShip::system,
                View::system,
                CPlane::system,
                ZPlane::system,
            )
                .before(RenderSystems::Compute),
        );
    }
}

// Mandelbrot Family

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Component, Deref, DerefMut)]
pub struct Mandelbrot(pub f32);

impl Default for Mandelbrot {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct ColorScale(pub f32);

impl Default for ColorScale {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct ColorRotation(pub f32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct Pickover(pub f32);

// Buddha

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Component, Deref, DerefMut)]
pub struct Buddha(pub f32);

impl Default for Buddha {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct BuddhaSamples(pub u32);

impl Default for BuddhaSamples {
    fn default() -> Self {
        Self(32)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component)]
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

// Bulb

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Component, Deref, DerefMut)]
pub struct Bulb(pub f32);

impl Default for Bulb {
    fn default() -> Self {
        Self(1.0)
    }
}

// Shared

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct Iterations(pub u32);

impl Default for Iterations {
    fn default() -> Self {
        Self(1_000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct EscapeRadius(pub f32);

impl Default for EscapeRadius {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct Exponent(pub f32);

impl Default for Exponent {
    fn default() -> Self {
        Self(2.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct Rotation(pub f32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct Julia(pub f32);

#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component, Deref, DerefMut)]
pub struct BurningShip(pub f32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Add, Param, Component)]
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

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component)]
pub struct CPlane {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Lerp, Add, Param, Component)]
pub struct ZPlane {
    pub x: f32,
    pub y: f32,
}

pub const PALETTE_LEN: usize = 32;
#[derive(Clone, Component, Deref, DerefMut)]
pub struct Palette(pub [Srgb; PALETTE_LEN]);

impl Default for Palette {
    fn default() -> Self {
        magma()
    }
}

impl Lerp for Palette {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        let mut out = self.0;
        for (rhs, lhs) in out.iter_mut().zip(rhs.0) {
            *rhs = (rhs.to_linear() * (1.0 - t) + lhs.to_linear() * t).to_srgb();
        }
        Self(out)
    }
}

macro_rules! palette_builder {
    ($($palette:ident,)*) => {
        $(
            #[allow(unused)]
            pub fn $palette() -> Palette {
                gradient_palette(&colorgrad::preset::$palette())
            }
        )*
    };
}

palette_builder!(
    blues,
    br_bg,
    bu_gn,
    bu_pu,
    cividis,
    cool,
    cubehelix_default,
    gn_bu,
    greens,
    greys,
    inferno,
    magma,
    or_rd,
    oranges,
    pi_yg,
    plasma,
    pr_gn,
    pu_bu,
    pu_bu_gn,
    pu_or,
    pu_rd,
    purples,
    rainbow,
    rd_bu,
    rd_gy,
    rd_pu,
    rd_yl_bu,
    rd_yl_gn,
    reds,
    sinebow,
    spectral,
    turbo,
    viridis,
    warm,
    yl_gn,
    yl_gn_bu,
    yl_or_br,
    yl_or_rd,
);

pub fn gradient_palette(grad: &impl colorgrad::Gradient) -> Palette {
    let mut palette = [Srgb::default(); PALETTE_LEN];
    let samples = PALETTE_LEN / 2;
    let mut i = 0;
    for x in 0..=samples {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette[i] = Srgb::new(r, g, b, 255);
        i += 1;
    }
    for x in (1..samples).rev() {
        let rgb = grad.at(x as f32 / samples as f32);
        let [r, g, b, _] = rgb.to_rgba8();
        palette[i] = Srgb::new(r, g, b, 255);
        i += 1;
    }
    Palette(palette)
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Component)]
#[require(
    // mandelbrot family
    ColorScale,
    ColorRotation,
    Pickover,
    // buddha
    BuddhaSamples,
    RgbIterations,
    // shared
    Iterations,
    EscapeRadius,
    Exponent,
    Rotation,
    Julia,
    BurningShip,
    View,
    CPlane,
    ZPlane,
    Palette,
)]
pub struct Params {
    // mandelbrot family
    pub color_scale: ColorScale,
    pub color_rotation: ColorRotation,
    pub pickover: Pickover,
    // buddha
    pub buddha_samples: BuddhaSamples,
    pub rgb_iterations: RgbIterations,
    // shared
    pub iterations: Iterations,
    pub escape_radius: EscapeRadius,
    pub exponent: Exponent,
    pub view: View,
    pub rotation: Rotation,
    pub julia: Julia,
    pub burning_ship: BurningShip,
    pub c_plane: CPlane,
    pub z_plane: ZPlane,
}
