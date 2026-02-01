use crate::tween::{Interpolate, Lerp, LogF32};
use num_complex::Complex;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::atomic::{AtomicU32, Ordering};
use tint::{Color, Srgb};

crate::lerp! {
    #[derive(Clone, Copy, PartialEq)]
    pub struct Fractal {
        pub iterations: usize,
        pub escape_radius: f32,
        pub color_scale: f32,
        pub exponent: f32,
        pub view: View,
        pub rotation: f32,
        pub julia: f32,
        pub burning_ship: f32,
        pub cx: f32,
        pub cy: f32,
        pub zx: f32,
        pub zy: f32,
    }
}

impl Default for Fractal {
    fn default() -> Self {
        Self {
            iterations: 1000,
            escape_radius: 100.0,
            color_scale: 1.0,
            exponent: 2.0,
            view: View::default(),
            rotation: 0.0,
            julia: 0.0,
            burning_ship: 0.0,
            cx: 0.0,
            cy: 0.0,
            zx: 0.0,
            zy: 0.0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
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
            z: 1.25,
        }
    }
}

impl Interpolate for View {
    type Output = View;
    fn interpolate(start: &Self, end: &Self, t: f32) -> Self::Output {
        let z = LogF32::interpolate(&LogF32(start.z), &LogF32(end.z), t);
        let factor = if (end.z - start.z).abs() < f32::EPSILON {
            t
        } else {
            (z - start.z) / (end.z - start.z)
        };
        View {
            x: start.x.lerp(&end.x, factor),
            y: start.y.lerp(&end.y, factor),
            z,
        }
    }
}

pub struct Renderer {
    width: usize,
    height: usize,
    palette: Vec<Srgb>,
    data: Vec<AtomicU32>,
    pixels: Vec<Srgb>,
    pub config: Fractal,
    last_config: Fractal,
}

impl Renderer {
    pub fn new(width: usize, height: usize, palette: Vec<Srgb>) -> Self {
        Self {
            width,
            height,
            palette,
            data: (0..width * height).map(|_| AtomicU32::new(0)).collect(),
            pixels: vec![Srgb::default(); width * height],
            last_config: Fractal {
                // make sure they arent the same
                iterations: 0,
                ..Default::default()
            },
            config: Fractal::default(),
        }
    }

    pub fn render(&mut self) -> &[Srgb] {
        if self.config == self.last_config {
            return &self.pixels;
        }
        self.last_config = self.config;

        let config = &self.config;
        let aspect = self.width as f32 / self.height as f32;
        let er2 = config.escape_radius * config.escape_radius;

        (0..self.width * self.height).into_par_iter().for_each(|i| {
            let y = i / self.width;
            let x = i % self.width;

            let px0 = (x as f32 / self.width as f32 * 2.0 - 1.0) * aspect * config.view.z;
            let py0 = (y as f32 / self.height as f32 * 2.0 - 1.0) * config.view.z;

            let point = Complex::new(px0, py0);
            let rotated = point * Complex::from_polar(1.0, config.rotation);

            let x0 = rotated.re + config.view.x;
            let y0 = rotated.im + config.view.y;

            let pc = Complex::new(x0, y0);
            let pz = Complex::new(config.zx, config.zy);
            let julia = Complex::new(config.cx, config.cy);
            let c = pc * (1.0 - config.julia) + julia * config.julia;
            let mut z = pz * (1.0 - config.julia) + pc * config.julia;

            if config.julia == 0.0 {
                // simple cardioid and bulb check
                //
                // https://mathr.co.uk/blog/2022-11-19_cardioid_and_bulb_checking.html
                let y2 = y0 * y0;
                let q = (x0 - 0.25).powi(2) + y2;
                if q * (q + (x0 - 0.25)) < 0.25 * y2 || (x0 + 1.0).powi(2) + y2 < 0.25 * 0.25 {
                    self.data[y * self.width + x]
                        .store((config.iterations as f32).to_bits(), Ordering::Relaxed);
                    return;
                }
            }

            let mut iteration = 0;
            while z.norm_sqr() < er2 && iteration < config.iterations {
                if config.burning_ship != 0.0 {
                    let mz = z.powf(config.exponent) + c;
                    let bz = (Complex::new(z.re.abs(), z.im.abs())).powf(config.exponent) + c;
                    z = mz * (1.0 - config.burning_ship) + bz * config.burning_ship;
                } else if config.exponent == 2.0 {
                    z = z * z + c;
                } else {
                    z = z.powf(config.exponent) + c;
                }
                iteration += 1;
            }

            if iteration == config.iterations {
                self.data[y * self.width + x]
                    .store((config.iterations as f32).to_bits(), Ordering::Relaxed);
                return;
            }

            let zn = z.norm_sqr();
            let nu = (zn.log2() * 0.5).log2() / config.exponent.log2();
            let iteration = iteration as f32 + 1.0 - nu;
            self.data[y * self.width + x].store(iteration.to_bits(), Ordering::Relaxed);
        });

        let data = unsafe { std::mem::transmute::<&[AtomicU32], &[u32]>(self.data.as_slice()) };
        for (pixel, data) in self.pixels.iter_mut().zip(data.iter()) {
            let iteration = f32::from_bits(*data);
            if iteration >= config.iterations as f32 {
                *pixel = Srgb::from_rgb(0, 0, 0);
            } else {
                let index = (iteration * config.color_scale) % self.palette.len() as f32;
                let c1 = self.palette[index as usize];
                let c2 = self.palette[(index as usize + 1) % self.palette.len()];
                let c = c1.to_linear() * (1.0 - index.fract()) + c2.to_linear() * index.fract();
                *pixel = c.to_srgb();
            }
        }
        &self.pixels
    }
}
