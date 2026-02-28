struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vec2(f32((id << 1u) & 2u), f32(id & 2u));
    out.clip_position = vec4(out.uv * 2.0 + vec2(-1.0, -1.0), 0.0, 1.0);
	out.uv.y = 1.0 - out.uv.y;
    return out;
}

struct Ssaa {
    mandelbrot: f32,
    buddha: f32,
    bulb: f32,
}

@group(0) @binding(0) var mandelbrot: texture_2d<f32>;
@group(0) @binding(1) var buddha: texture_2d<f32>;
@group(0) @binding(2) var bulb: texture_2d<f32>;
@group(0) @binding(3) var texture_sampler: sampler;
@group(0) @binding(4) var<uniform> args: Ssaa;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let m = textureSample(mandelbrot, texture_sampler, in.uv);
	let b = textureSample(buddha, texture_sampler, in.uv);
	let l = textureSample(bulb, texture_sampler, in.uv);
	return vec4(m.rgb * args.mandelbrot + b.rgb * args.buddha + l.rgb * args.bulb, 1.0);
}
