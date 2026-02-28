const PALETTE_LEN: f32 = 32.0;

struct Params {
	// mandelbrot family
	color_scale: f32,
	color_rotation: f32,
	pickover: f32,
	// buddha
	buddha_samples: u32,
	riter: u32,
	giter: u32,
	biter: u32,
	// shared
	iterations: u32,
	escape_radius: f32,
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
}

fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

fn cpow(z: vec2<f32>, n: f32) -> vec2<f32> {
    let r = length(z);
    if (r == 0.0) { return vec2(0.0); }
    let a = atan2(z.y, z.x);
    let rn = pow(r, n);
    let na = n * a;
    return vec2(rn * cos(na), rn * sin(na));
}

fn csin(a: vec2<f32>) -> vec2<f32> {
	return vec2(sin(a.x) * cosh(a.y), cos(a.x) * sinh(a.y));
}

fn cln(a: vec2<f32>) -> vec2<f32> {
	return vec2(log(length(a)), atan2(abs(a.y), a.x));
}

fn cexp(a: vec2<f32>) -> vec2<f32> {
	return vec2(cos(a.y), sin(a.y)) * exp(a.x);
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

fn func(z: vec2<f32>, c: vec2<f32>) -> vec2<f32> {
	if (args.burning_ship != 0.0) {
		let z_abs = vec2(abs(z.x), abs(z.y));
		let mz = cpow(z, args.exponent) + c;
		let bz = cpow(z_abs, args.exponent) + c;
		return mz * (1.0 - args.burning_ship) + bz * args.burning_ship;
	} else if (args.exponent == 2.0) {
		// tippetts
		// let x = z.x * z.x - z.y * z.y + c.x;
		// return vec2(x, 2.0 * x * z.y + c.y);
		return cmul(z, z) + c;
	} else {
		return cpow(z, args.exponent) + c;
	}
}
