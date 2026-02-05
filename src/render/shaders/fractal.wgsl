const PALETTE_LEN: f32 = 32.0;

struct Fractal {
	iterations: u32,
	escape_radius: f32,
	color_scale: f32,
	exponent: f32,
	x: f32,
	y: f32,
	z: f32,
	rotation: f32,
	julia: f32,
	burning_ship: f32,
	cx: f32,
	cy: f32,
	zx: f32,
	zy: f32,
	color_rotation: f32,
	buddha_samples: u32,
	riter: u32,
	giter: u32,
	biter: u32,
	pickover: f32,
}

struct BuddhaNorm {
	rmin: atomic<u32>,
	rmax: atomic<u32>,
	gmin: atomic<u32>,
	gmax: atomic<u32>,
	bmin: atomic<u32>,
	bmax: atomic<u32>,
}

@group(0) @binding(0) var mandelbrot: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: Fractal;

@group(0) @binding(2) var<storage, read_write> buddha_iterations: array<atomic<u32>>;
@group(0) @binding(3) var buddha: texture_storage_2d<rgba32float, write>;
@group(0) @binding(4) var<storage, read_write> buddha_norm: BuddhaNorm;

@group(1) @binding(0) var palette: texture_2d<f32>;
@group(1) @binding(1) var palette_sampler: sampler;

fn c_mul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn c_pow(z: vec2<f32>, n: f32) -> vec2<f32> {
    let r = length(z);
    if (r == 0.0) { return vec2(0.0); }
    let a = atan2(z.y, z.x);
    let rn = pow(r, n);
    let na = n * a;
    return vec2(rn * cos(na), rn * sin(na));
}

@compute @workgroup_size(8, 8)
fn compute_buddha(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(buddha);
    if (id.x >= sz.x * args.buddha_samples || id.y >= sz.y * args.buddha_samples) {
        return;
    }
    let width = f32(sz.x);
    let height = f32(sz.y);
    let x = f32(id.x);
    let y = f32(id.y);
    let aspect = width / height;
	let max_iterations = max(max(args.riter, args.giter), args.biter);

    let px0 = (x / width / f32(args.buddha_samples) * 2.0 - 1.0) * args.z;
    let py0 = (y / height / f32(args.buddha_samples) * 2.0 - 1.0) * args.z;
    let point = vec2(px0 * aspect, py0);
    
    let rot = vec2(cos(args.rotation), sin(args.rotation));
    let rotated = c_mul(point, rot);

    let x0 = rotated.x + args.x;
    let y0 = rotated.y + args.y;

	// do the loop once, determine if it escapes
	let tcz = cz(x0, y0);
	let tc = tcz.c;
	var tz = tcz.z;
    var titeration: u32 = 0;
    while dot(tz, tz) < args.escape_radius * args.escape_radius && titeration < max_iterations {
		tz = func(tz, tc);
        titeration++;
    }
	if titeration == max_iterations {
		return;
	}
	let rescaped = titeration < args.riter;
	let gescaped = titeration < args.giter;
	let bescaped = titeration < args.biter;

	let cz = cz(x0, y0);
	let c = cz.c;
	var z = cz.z;
    var iteration: u32 = 0;

    while dot(z, z) < args.escape_radius * args.escape_radius && iteration < max_iterations {
		z = func(z, c);
        iteration++;

		// need to undo all of the transformations to index into the storage buffer
        let zt = vec2(z.x - args.x, z.y - args.y);
        let zr = c_mul(zt, vec2(cos(-args.rotation), sin(-args.rotation)));
        let zn = vec2(zr.x / aspect, zr.y);
        
        let px = (zn.x / args.z + 1.0) * width / 2.0;
        let py = (zn.y / args.z + 1.0) * height / 2.0;

		if px >= 0.0 && px < width && py >= 0.0 && py < height {
			let index = (u32(py) * sz.x + u32(px)) * 3;
			if rescaped && iteration < args.riter {
				atomicAdd(&buddha_iterations[index], 1);
			}
			if gescaped && iteration < args.giter {
				atomicAdd(&buddha_iterations[index + 1], 1);
			}
			if bescaped && iteration < args.biter {
				atomicAdd(&buddha_iterations[index + 2], 1);
			}
		}
    }
}

@compute @workgroup_size(256)
fn buddha_min_max(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(buddha);
    if (id.x >= sz.x * sz.y) {
        return;
    }
	let index = id.x * 3;
	if args.riter > 0 {
		let r = buddha_iterations[index];
		if r > 0 {
			atomicMin(&buddha_norm.rmin, r);
			atomicMax(&buddha_norm.rmax, r);
		}
	}
	if args.giter > 0 {
		let g = buddha_iterations[index + 1];
		if g > 0 {
			atomicMin(&buddha_norm.gmin, g);
			atomicMax(&buddha_norm.gmax, g);
		}
	}
	if args.biter > 0 {
		let b = buddha_iterations[index + 2];
		if b > 0 {
			atomicMin(&buddha_norm.bmin, b);
			atomicMax(&buddha_norm.bmax, b);
		}
	}
}

@compute @workgroup_size(16, 16)
fn render_buddha(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(buddha);
    if (id.x >= sz.x || id.y >= sz.y) {
        return;
    }
	let index = (id.y * sz.x + id.x) * 3;

	var ri = 0.0;
	var gi = 0.0;
	var bi = 0.0;

	if args.riter > 0 && buddha_norm.rmax != buddha_norm.rmin {
		let riter = buddha_iterations[index];
		ri = (f32(riter)) / f32(buddha_norm.rmax);
	}
	if args.giter > 0 && buddha_norm.gmax != buddha_norm.gmin {
		let giter = buddha_iterations[index + 1];
		gi = (f32(giter)) / f32(buddha_norm.gmax);
	}
	if args.biter > 0 && buddha_norm.bmax != buddha_norm.bmin {
		let biter = buddha_iterations[index + 2];
		bi = (f32(biter)) / f32(buddha_norm.bmax);
	}

    textureStore(buddha, vec2(id.xy), vec4(ri, gi, bi, 1.0));
}

@compute @workgroup_size(16, 16)
fn render_mandelbrot(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(mandelbrot);
    if (id.x >= sz.x || id.y >= sz.y) {
        return;
    }
    let width = f32(sz.x);
    let height = f32(sz.y);
    let x = f32(id.x);
    let y = f32(id.y);
    let aspect = width / height;
	let radius = args.escape_radius * args.escape_radius;

    let px0 = (x / width * 2.0 - 1.0) * aspect * args.z;
    let py0 = (y / height * 2.0 - 1.0) * args.z;
    let point = vec2(px0, py0);

    let rot = vec2(cos(args.rotation), sin(args.rotation));
    let rotated = c_mul(point, rot);

    let x0 = rotated.x + args.x;
    let y0 = rotated.y + args.y;

	let cz = cz(x0, y0);
	let c = cz.c;
	var z = cz.z;
    var iteration: u32 = 0;
	var trap = 1000000.0;

    while (dot(z, z) < radius && iteration < args.iterations) {
		z = func(z, c);
		let dist = abs(z);
		trap = min(trap, min(dist.x, dist.y));
        iteration++;
    }

	let col = color(iteration, z);
	let ctrap = col * trap;
	let result = col * (1.0 - args.pickover) + ctrap * args.pickover;
    textureStore(mandelbrot, vec2(id.xy), vec4(result.rgb, 1.0));
}

struct CandZ {
	c: vec2<f32>,
	z: vec2<f32>,
}

fn cz(x0: f32, y0: f32) -> CandZ {
	var out: CandZ;
    let pc = vec2(x0, y0);
    let pz = vec2(args.zx, args.zy);
    let julia = vec2(args.cx, args.cy);
    out.c = pc * (1.0 - args.julia) + julia * args.julia;
    out.z = pz * (1.0 - args.julia) + pc * args.julia;
	return out;
}

fn color(iteration: u32, z: vec2<f32>) -> vec4<f32> {
	if (iteration == args.iterations) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    } 
    let zn = dot(z, z);
    var iter: f32;
	// When zn explodes due to a large exponent, a black bar appears and looks
	// like an artifact. In that case, this should just use the raw iteration value.
    if (zn > 3.0e38) {
        iter = f32(iteration); 
    } else {
		// smooth otherwise
        let nu = log2(log2(zn) * 0.5) / log2(args.exponent);
        iter = f32(iteration) + 1.0 - nu;
    }
    let uv = vec2(iter * args.color_scale / PALETTE_LEN + args.rotation * args.color_rotation, 0.5);
    let rgb = textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
    return vec4(rgb, 1.0);
}

fn func(z: vec2<f32>, c: vec2<f32>) -> vec2<f32> {
	if (args.burning_ship != 0.0) {
		let z_abs = vec2(abs(z.x), abs(z.y));
		let mz = c_pow(z, args.exponent) + c;
		let bz = c_pow(z_abs, args.exponent) + c;
		return mz * (1.0 - args.burning_ship) + bz * args.burning_ship;
	} else if (args.exponent == 2.0) {
		return c_mul(z, z) + c;
	} else {
		return c_pow(z, args.exponent) + c;
	}
}
