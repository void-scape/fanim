struct BuddhaNorm {
	rmin: atomic<u32>,
	rmax: atomic<u32>,
	gmin: atomic<u32>,
	gmax: atomic<u32>,
	bmin: atomic<u32>,
	bmax: atomic<u32>,
}

@group(0) @binding(0) var<uniform> args: Params;
@group(0) @binding(1) var<storage, read_write> buddha_iterations: array<atomic<u32>>;
@group(0) @binding(2) var buddha: texture_storage_2d<rgba32float, write>;
@group(0) @binding(3) var<storage, read_write> buddha_norm: BuddhaNorm;

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
    let py0 = ((1.0 - y / height) / f32(args.buddha_samples) * 2.0 - 1.0) * args.z;
    let point = vec2(px0 * aspect, py0);
    
    let rot = vec2(cos(args.rotation), sin(args.rotation));
    let rotated = cmul(point, rot);

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
        let zr = cmul(zt, vec2(cos(-args.rotation), sin(-args.rotation)));
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
