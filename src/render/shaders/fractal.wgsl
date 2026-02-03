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
	_pad: u32,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: Fractal;

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

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(output);
    if (id.x >= sz.x || id.y >= sz.y) {
        return;
    }
    let width = f32(sz.x);
    let height = f32(sz.y);
    let x = f32(id.x);
    let y = f32(id.y);
    let aspect = width / height;

    let px0 = (x / width * 2.0 - 1.0) * aspect * args.z;
    let py0 = (y / height * 2.0 - 1.0) * args.z;
    let point = vec2(px0, py0);

    let rot = vec2(cos(args.rotation), sin(args.rotation));
    let rotated = c_mul(point, rot);

    let x0 = rotated.x + args.x;
    let y0 = rotated.y + args.y;

    let pc = vec2(x0, y0);
    let pz = vec2(args.zx, args.zy);
    let julia = vec2(args.cx, args.cy);

    let c = pc * (1.0 - args.julia) + julia * args.julia;
    var z = pz * (1.0 - args.julia) + pc * args.julia;

    var iteration: u32 = 0;
    while (dot(z, z) < args.escape_radius * args.escape_radius && iteration < args.iterations) {
        if (args.burning_ship != 0.0) {
            let z_abs = vec2(abs(z.x), abs(z.y));
            let mz = c_pow(z, args.exponent) + c;
            let bz = c_pow(z_abs, args.exponent) + c;
            z = mz * (1.0 - args.burning_ship) + bz * args.burning_ship;
        } else if (args.exponent == 2.0) {
            z = c_mul(z, z) + c;
        } else {
            z = c_pow(z, args.exponent) + c;
        }
        iteration++;
    }
    textureStore(output, vec2(id.xy), color(iteration, z));
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
