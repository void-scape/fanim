@group(0) @binding(0) var bulb: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: Params;
@group(0) @binding(2) var palette: texture_2d<f32>;
@group(0) @binding(3) var palette_sampler: sampler;

@compute @workgroup_size(16, 16)
fn render_bulb(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(bulb);
    if (id.x >= sz.x || id.y >= sz.y) {
        return;
    }
    let width = f32(sz.x);
    let height = f32(sz.y);
    let aspect = width / height;
	let uv = vec2<f32>(
        (f32(id.x) / width * 2.0 - 1.0) * aspect,
        -(f32(id.y) / height * 2.0 - 1.0)
    );

	let ray = normalize(vec3(uv, -1.0));
	var dist = 0.0;
	let max_dist = 100.0;
	let eps = 0.001;
	var hit = false;
	var steps = 0;

	var p: vec3<f32>;
	for (var i = 0; i < 1000; i++) {
		p = vec3(0.0, 0.0, 5.1) + ray * dist;
		let dt = de(p);
		if dt < eps {
			hit = true;
			steps = i;
			break;
		}
		dist += dt;
		if dist > max_dist {
			break;
		}
	}

	if hit {
		textureStore(bulb, id.xy, vec4(color(p), 1.0));
	} else {
		textureStore(bulb, id.xy, vec4(0.0, 0.0, 0.0, 1.0));
	}
}

fn de(p: vec3<f32>) -> f32 {
    var z = p;
    var dr = 1.0;
    var r = 0.0;
    for (var i = 0u; i < args.iterations; i++) {
        r = length(z);
        if r > args.escape_radius {
            break;
        }
        dr = pow(r, args.exponent - 1.0) * args.exponent * dr + 1.0;
		z = compute_bulb(z, p);
    }
    return 0.5 * log(r) * r / dr;
}

fn color(p: vec3<f32>) -> vec3<f32> {
	var min_dist = 1e10;
    var z = p;
    for (var i = 0u; i < args.iterations; i++) {
		let r = length(z);
        if r > args.escape_radius * args.escape_radius {
            break;
        }
		z = compute_bulb(z, p);
		min_dist = min(min_dist, r);
    }
    let uv = vec2(min_dist * args.color_scale + args.rotation * args.color_rotation, 0.5);
    return textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
}

fn compute_bulb(zin: vec3<f32>, p: vec3<f32>) -> vec3<f32> {
	var z = zin;
	let theta = atan2(sqrt(z.x * z.x + z.y * z.y), z.z + 1e-7);
	let phi = atan2(z.y, z.x + 1e-7);
	z = pow(length(z), args.exponent)
		* vec3(
			sin(theta * args.exponent) * cos(phi * args.exponent),
			sin(theta * args.exponent) * sin(phi * args.exponent),
			cos(theta * args.exponent),
		);
	z += p;
	return z;
}
