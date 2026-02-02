pub mod audio;
mod compute;
pub mod encoder;
pub mod palette;
pub mod render;
mod ssaa;
pub mod tween;

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

// use clap::Parser;
// use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
// use num_complex::Complex;
// use rand::{Rng, SeedableRng};
// use rand_xorshift::XorShiftRng;
// use rayon::iter::{IntoParallelIterator, ParallelIterator};
// use std::sync::atomic::{AtomicU32, Ordering};
// use tint::Srgb;
//
// #[derive(Parser, Debug)]
// #[command(version, about, long_about = None)]
// struct Args {
//     /// Output png file.
//     output: String,
//
//     /// Path to a config toml.
//     #[arg(short, long)]
//     config: String,
//
//     /// Path to the data directory.
//     #[arg(short, long)]
//     data: String,
// }
//
// const DEFAULT_SIZE: usize = 800;
// const DEFAULT_ITERATIONS: usize = 1_000;
// const DEFAULT_COLOR_SCALE: f32 = 8_000.0;
//
// #[derive(Debug, serde::Deserialize, serde::Serialize)]
// #[serde(default, rename_all = "kebab-case")]
// struct Config {
//     size: usize,
//     ssaa: f32,
//     pow: f32,
//     theta: f32,
//     r_channel: Channel,
//     g_channel: Channel,
//     b_channel: Channel,
// }
//
// impl Default for Config {
//     fn default() -> Self {
//         Self {
//             size: DEFAULT_SIZE,
//             ssaa: 32.0,
//             pow: 2.0,
//             theta: 0.0,
//             r_channel: Channel::default(),
//             g_channel: Channel::default(),
//             b_channel: Channel::default(),
//         }
//     }
// }
//
// #[derive(Debug, serde::Deserialize, serde::Serialize)]
// #[serde(default)]
// struct Channel {
//     scale: f32,
//     iterations: usize,
//     color_scale: f32,
// }
//
// impl Default for Channel {
//     fn default() -> Self {
//         Self {
//             scale: 1.0,
//             iterations: DEFAULT_ITERATIONS,
//             color_scale: DEFAULT_COLOR_SCALE,
//         }
//     }
// }
//
// fn main() -> std::io::Result<()> {
//     let args = Args::parse();
//     let mut config = toml::from_str::<Config>(&std::fs::read_to_string(args.config)?).unwrap();
//
//     config.ssaa = 64.0;
//     config.r_channel.color_scale = 10_000.0;
//     config.g_channel.color_scale = 10_000.0;
//     config.b_channel.color_scale = 10_000.0;
//     // render_frame(&args.output, &args.data, &config)?;
//     // return Ok(());
//
//     let start = 0.0;
//     let end = std::f32::consts::TAU;
//     let steps = 180;
//     let dt = (end - start) / steps as f32;
//
//     let mut theta = start;
//     for t in 0..=steps as usize {
//         let data = format!("{}/data_{t}", args.data);
//         _ = std::fs::create_dir_all(&data);
//         let output = format!("{data}/../{t}.png");
//         config.theta = theta;
//         render_frame(&output, &data, &config)?;
//         theta += dt;
//     }
//
//     fn render_frame(output: &str, data: &str, config: &Config) -> std::io::Result<()> {
//         let multi = MultiProgress::new();
//
//         let (r, g, b) = if config.r_channel.iterations == config.b_channel.iterations
//             && config.b_channel.iterations == config.g_channel.iterations
//         {
//             let r = process_channel(config, &config.r_channel, data, &multi)?;
//             (r.clone(), r.clone(), r)
//         } else {
//             let (r, (g, b)) = rayon::join(
//                 || process_channel(config, &config.r_channel, data, &multi),
//                 || {
//                     rayon::join(
//                         || process_channel(config, &config.g_channel, data, &multi),
//                         || process_channel(config, &config.b_channel, data, &multi),
//                     )
//                 },
//             );
//             let r = r?;
//             let g = g?;
//             let b = b?;
//             (r, g, b)
//         };
//
//         let mut frame_buffer = vec![Srgb::default(); config.size * config.size];
//         let color_scale = [
//             config.r_channel.color_scale,
//             config.g_channel.color_scale,
//             config.b_channel.color_scale,
//         ];
//         for (i, pixel) in frame_buffer.iter_mut().enumerate() {
//             let rgb = [
//                 (r[i] as f32 / color_scale[0]).powf(1.2),
//                 (g[i] as f32 / color_scale[1]).powf(1.2),
//                 (b[i] as f32 / color_scale[2]).powf(1.2),
//             ];
//             *pixel = Srgb::from_rgb(
//                 ((rgb[0] * config.r_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
//                 ((rgb[1] * config.g_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
//                 ((rgb[2] * config.b_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
//             );
//         }
//
//         fract::encoder::png(
//             output,
//             fract::byte_slice(&frame_buffer),
//             config.size,
//             config.size,
//             false,
//         )?;
//
//         Ok(())
//     }
//
//     Ok(())
// }
//
// fn find_channel_hist(
//     size: usize,
//     channel: &Channel,
//     data: &str,
// ) -> std::io::Result<Option<Vec<u32>>> {
//     let mut out = None;
//     let target_stem = format!("{}-{}", size, channel.iterations);
//     for entry in std::fs::read_dir(data)? {
//         if entry?
//             .path()
//             .file_stem()
//             .is_some_and(|str| *str == *target_stem)
//         {
//             let bin = std::fs::read(format!("{}/{}.bin", data, target_stem))?;
//             out = Some(
//                 bin.chunks(4)
//                     .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
//                     .collect(),
//             );
//         }
//     }
//     Ok(out)
// }
//
// fn process_channel(
//     config: &Config,
//     channel_config: &Channel,
//     data_dir: &str,
//     multi: &MultiProgress,
// ) -> std::io::Result<Vec<u32>> {
//     // if let Some(existing) = find_channel_hist(config.size, channel_config, data_dir)? {
//     //     return Ok(existing);
//     // }
//
//     let iterations = channel_config.iterations;
//     let bar = multi.add(progress_bar(config.size, iterations, config.ssaa));
//     let hist = compute_hist(Some(&bar), config, iterations);
//
//     let target_stem = format!("{}-{}", config.size, iterations);
//     std::fs::write(
//         format!("{}/{}.bin", data_dir, target_stem),
//         fract::byte_slice(&hist),
//     )?;
//
//     Ok(hist)
// }
//
// fn progress_bar(size: usize, iter: usize, ssaa: f32) -> ProgressBar {
//     let bar = ProgressBar::no_length();
//     let width = (size * ssaa as usize / 2).to_string().len();
//     bar.set_style(
//         ProgressStyle::with_template(&format!(
//             "[{{elapsed_precise}}] iter={iter:<8} {{bar:40.cyan/blue}} \
//                 cols={{pos:>{width}}}/{{len:{}}} eta={{eta_precise}}",
//             width * 2,
//         ))
//         .unwrap()
//         .progress_chars("##-"),
//     );
//     bar
// }
//
// const SPANX: f32 = 3.5;
// const SPANY: f32 = 3.5;
// const XOFFSET: f32 = -0.25;
//
// fn compute_hist(
//     progress_bar: Option<&ProgressBar>,
//     config: &Config,
//     iterations: usize,
// ) -> Vec<u32> {
//     let size = config.size;
//     let ssaa = config.ssaa;
//     let pow = config.pow;
//
//     // only computes one half of the y-axis then copies to the other if no there
//     // is no rotation
//     let mirror = config.theta == 0.0;
//     let factor = if mirror { 2 } else { 1 };
//
//     if let Some(bar) = progress_bar {
//         bar.set_length((size * ssaa as usize / factor) as u64);
//         bar.set_position(0);
//     }
//
//     let hist = (0..size * size)
//         .map(|_| AtomicU32::new(0))
//         .collect::<Vec<_>>();
//
//     let fsize = size as f32;
//     let add = if mirror { 1 } else { 0 };
//     let total_pixels = (add + size * ssaa as usize / factor) * (size * ssaa as usize);
//     (0..total_pixels).into_par_iter().for_each(|i| {
//         // let py = i / (size * ssaa as usize);
//         // let px = i % (size * ssaa as usize);
//
//         let mut rng = XorShiftRng::seed_from_u64(i as u64);
//         let x0 = rng.random_range::<f32, _>(-2.0..2.0);
//         let y0 = rng.random_range::<f32, _>(-2.0..2.0);
//
//         // let y0 = (py as f32 - fsize * ssaa / 2.0) / (fsize * ssaa) * SPANY;
//         // let x0 = (px as f32 - fsize * ssaa / 2.0) / (fsize * ssaa) * SPANX + XOFFSET;
//
//         // simple cardioid and bulb check
//         //
//         // https://mathr.co.uk/blog/2022-11-19_cardioid_and_bulb_checking.html
//         let y2 = y0 * y0;
//         let q = (x0 - 0.25).powi(2) + y2;
//         if q * (q + (x0 - 0.25)) < 0.25 * y2 || (x0 + 1.0).powi(2) + y2 < 0.25 * 0.25 {
//             return;
//         }
//
//         let c = Complex::new(x0, y0);
//         // let mut z = Complex::new(0.0, 0.0);
//         let r = 2.0;
//         let mut z = Complex::new(rng.random_range(-r..r), rng.random_range(-r..r));
//
//         let (sin, cos) = config.theta.sin_cos();
//
//         let mut path = Vec::new();
//         let mut iteration = 0;
//         while z.norm_sqr() < 10_000.0 && iteration < iterations {
//             iteration += 1;
//             if pow == 2.0 {
//                 z = z * z + c;
//             } else {
//                 z = z.powf(pow) + c;
//             }
//             // path.push(z);
//
//             // to mandelbrot
//             // let xproj = z.re * cos + c.re * sin;
//             // let yproj = z.im * cos + c.im * sin;
//
//             // looks around y
//             let xproj = z.re;
//             let yproj = z.im * cos + c.im * sin;
//
//             // looks around x
//             // let xproj = z.re * cos + c.re * sin;
//             // let yproj = z.im;
//
//             let z_depth = c.re; // Using an unused rotation plane for depth
//
//             let camera_dist = 16.0;
//             let perspective = camera_dist / (camera_dist + z_depth);
//
//             path.push(Complex::new(xproj, yproj) * perspective);
//         }
//
//         if iteration == iterations {
//             return;
//         }
//
//         for z in path.iter() {
//             let x = z.re;
//             let y = z.im;
//             let px = (((x - XOFFSET) / SPANX + 0.5) * fsize) as isize;
//             if px < 0 || px >= size as isize {
//                 continue;
//             }
//
//             let py = ((y / SPANY + 0.5) * fsize) as isize;
//             if py < 0 || py >= size as isize {
//                 continue;
//             }
//
//             // rotate, this works because width == height
//             let x = px as usize;
//             let y = py as usize;
//             // write to both sides of the y-axis
//             if mirror {
//                 hist[x * size + (size - 1 - y)].fetch_add(1, Ordering::Relaxed);
//             }
//             hist[x * size + y].fetch_add(1, Ordering::Relaxed);
//         }
//
//         if i % (size * ssaa as usize) == 0
//             && let Some(bar) = progress_bar
//         {
//             bar.inc(1);
//         }
//     });
//     if let Some(bar) = progress_bar {
//         bar.finish();
//     }
//     // SAFETY: Nobody has access to hist at this point in time.
//     unsafe { std::mem::transmute(hist) }
// }
