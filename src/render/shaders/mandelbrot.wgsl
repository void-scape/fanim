@group(0) @binding(0) var mandelbrot: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: Params;
@group(0) @binding(2) var palette: texture_2d<f32>;
@group(0) @binding(3) var palette_sampler: sampler;

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
    let py0 = ((1.0 - y / height) * 2.0 - 1.0) * args.z;
    let point = vec2(px0, py0);

    let rot = vec2(cos(args.rotation), sin(args.rotation));
    let rotated = cmul(point, rot);

    let x0 = rotated.x + args.x;
    let y0 = rotated.y + args.y;

	let cz = cz(x0, y0);
	let c = cz.c;
	var z = cz.z;
    var iteration: u32 = 0;
	var trap = 1000000.0;

    while (dot(z, z) < radius && iteration < args.iterations) {
		z = func(z, c);
		let dist = min(abs(z.x), abs(z.y));
		trap = min(trap, dist);
        iteration++;
    }

	let zc = color_z(iteration, z);
	let tc = color_trap(trap);
	let result = mix(zc, tc, args.pickover);
    textureStore(mandelbrot, vec2(id.xy), vec4(result, 1.0));
}

fn color_z(iteration: u32, z: vec2<f32>) -> vec3<f32> {
	if (iteration == args.iterations) {
        return vec3(0.0, 0.0, 0.0);
    } 
    let zn = dot(z, z);
    var iter: f32;
    if (zn > 3.0e38) {
		// When zn explodes due to a large exponent, a black bar appears and looks
		// like an artifact. In that case, this should just use the raw iteration value.
        iter = f32(iteration); 
    } else {
		// smooth otherwise
        let nu = log2(log2(zn) * 0.5) / log2(args.exponent);
        iter = f32(iteration) + 1.0 - nu;
    }
    let uv = vec2(iter * args.color_scale / PALETTE_LEN + args.rotation * args.color_rotation, 0.5);
    return textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
}

fn color_trap(dist: f32) -> vec3<f32> {
	// clamping makes this more aesthetic
	let d = clamp(dist, 0.0000001, 1.0);
	// very noisy without log
    let t = -log(d) * 0.1;
    let uv = vec2(t * args.color_scale + args.rotation * args.color_rotation, 0.5);
    return textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
}
