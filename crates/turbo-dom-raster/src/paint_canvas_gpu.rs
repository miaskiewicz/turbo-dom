//! Opt-in (`gpu-metal`) backend for the 2D-canvas op-log painter: rasterize the
//! shared [`crate::canvas_ops`] op stream on the **real GPU** (wgpu → Metal) into
//! an offscreen texture, read the pixels back, and encode PNG. These are genuine
//! Apple-GPU pixels, not a CPU emulation.
//!
//! The whole file is gated behind `gpu-metal`, so the default build never links
//! wgpu/lyon/pollster. Solid fills and strokes are tessellated to triangles with
//! lyon (glyph outlines traced by [`crate::glyph`], flattened by the shared
//! interpreter), drawn in op order — source-over for paints, replace-with-zero for
//! `clearRect` — so blending and z-order match the tiny-skia backend. Every entry
//! point returns `Err` instead of panicking, so [`crate::canvas_ops_png`] can fall
//! back to tiny-skia on any GPU failure.

use std::sync::OnceLock;

use image::ImageEncoder;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};
use turbo_html2pdf_core::Rgba;
use wgpu::util::DeviceExt;

use crate::canvas_ops::{self, CanvasBackend, Op, SubPath};

/// The offscreen render-target format. `Unorm` (not `Srgb`) so the CSS colour
/// bytes are stored and blended verbatim — matching tiny-skia's gamma-space blend.
const TEX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Process-global GPU context. Building an adapter/device costs tens of ms, far too
/// much per call, so it is created once and shared (wgpu handles are `Send + Sync`).
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    // Read only by the feature's tests (adapter backend/name assertions).
    #[cfg_attr(not(test), allow(dead_code))]
    info: wgpu::AdapterInfo,
    /// Source-over pipeline for fills/strokes.
    pipeline_over: wgpu::RenderPipeline,
    /// Replace (no blend) pipeline: `clearRect` writes transparent black.
    pipeline_replace: wgpu::RenderPipeline,
    // Keep the instance/adapter alive for the process lifetime.
    _adapter: wgpu::Adapter,
    _instance: wgpu::Instance,
}

/// Get (or lazily build) the shared GPU context. The result is memoized including
/// the error, so a host without a usable Metal adapter fails fast on every call.
fn gpu() -> Result<&'static Gpu, String> {
    static G: OnceLock<Result<Gpu, String>> = OnceLock::new();
    G.get_or_init(Gpu::new).as_ref().map_err(String::clone)
}

impl Gpu {
    fn new() -> Result<Gpu, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok_or_else(|| "no Metal adapter".to_string())?;
        let info = adapter.get_info();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("turbo-surf canvas gpu"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|e| format!("request_device: {e}"))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("canvas"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let pipeline_over = make_pipeline(&device, &layout, &shader, Some(over_blend()));
        let pipeline_replace = make_pipeline(&device, &layout, &shader, None);

        Ok(Gpu {
            device,
            queue,
            info,
            pipeline_over,
            pipeline_replace,
            _adapter: adapter,
            _instance: instance,
        })
    }
}

/// Position-only vertex → pass-through colour fragment. Positions arrive in NDC
/// (the CPU maps device pixels to clip space), so the shader is a straight copy.
const SHADER: &str = r#"
struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};
@vertex
fn vs(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos, 0.0, 1.0);
    out.color = color;
    return out;
}
@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

/// Source-over blend for straight-alpha source colours over a transparent target.
fn over_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

/// Build a triangle-list pipeline over the interleaved `[pos.xy, color.rgba]`
/// vertex; `blend` `None` replaces the target (for `clearRect`).
fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let attrs = [
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x2,
            offset: 0,
            shader_location: 0,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 8,
            shader_location: 1,
        },
    ];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("canvas"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs",
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: VERTEX_BYTES as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &attrs,
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: "fs",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: TEX_FORMAT,
                blend,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// 6 floats per vertex: x, y (NDC) + r, g, b, a (0..1).
const VERTEX_BYTES: usize = 6 * 4;

/// Rasterize the parsed op stream at `width × height` on the GPU into a PNG.
pub(crate) fn replay(width: u32, height: u32, ops: &[Op]) -> Result<Vec<u8>, String> {
    let gpu = gpu()?;
    // Tessellate every op into device-space triangles + ordered draw ranges.
    let mut canvas = GpuCanvas::new(width as f32, height as f32);
    canvas_ops::run(ops, &mut canvas);
    render(gpu, width, height, &canvas)
}

/// The adapter's backend (`Metal` on this host) — used by the feature's tests to
/// assert the render really targets Apple's GPU.
#[cfg(test)]
pub(crate) fn adapter_backend() -> Result<wgpu::Backend, String> {
    Ok(gpu()?.info.backend)
}

#[cfg(test)]
pub(crate) fn adapter_name() -> Result<String, String> {
    Ok(gpu()?.info.name.clone())
}

/// Which blend a draw range uses.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Over,
    Replace,
}

/// A contiguous run of vertices drawn with one pipeline.
struct Range {
    kind: Kind,
    start: u32,
    count: u32,
}

/// Accumulates GPU triangles (in NDC) + ordered draw ranges as the shared
/// interpreter walks the op stream.
struct GpuCanvas {
    verts: Vec<f32>,
    ranges: Vec<Range>,
    w: f32,
    h: f32,
}

impl GpuCanvas {
    fn new(w: f32, h: f32) -> Self {
        GpuCanvas {
            verts: Vec::new(),
            ranges: Vec::new(),
            w,
            h,
        }
    }

    /// Map a device pixel to NDC (y flipped: pixel-space is y-down).
    fn ndc(&self, x: f32, y: f32) -> (f32, f32) {
        (x / self.w * 2.0 - 1.0, 1.0 - y / self.h * 2.0)
    }

    /// Append one device-space triangle in `kind`'s pipeline, merging into the last
    /// range when the pipeline matches (draw order is preserved either way).
    fn tri(&mut self, kind: Kind, p: [(f32, f32); 3], c: Rgba) {
        let start = (self.verts.len() / 6) as u32;
        let rgba = [
            c.r as f32 / 255.0,
            c.g as f32 / 255.0,
            c.b as f32 / 255.0,
            c.a as f32 / 255.0,
        ];
        for &(x, y) in &p {
            let (nx, ny) = self.ndc(x, y);
            self.verts
                .extend_from_slice(&[nx, ny, rgba[0], rgba[1], rgba[2], rgba[3]]);
        }
        match self.ranges.last_mut() {
            Some(r) if r.kind == kind && r.start + r.count == start => r.count += 3,
            _ => self.ranges.push(Range {
                kind,
                start,
                count: 3,
            }),
        }
    }

    /// Emit an indexed triangle mesh (lyon output) as individual triangles.
    fn mesh(&mut self, kind: Kind, mesh: &VertexBuffers<[f32; 2], u32>, c: Rgba) {
        let (tris, _rem) = mesh.indices.as_chunks::<3>();
        for idx in tris {
            let v = |i: u32| {
                let p = mesh.vertices[i as usize];
                (p[0], p[1])
            };
            self.tri(kind, [v(idx[0]), v(idx[1]), v(idx[2])], c);
        }
    }
}

/// Build a lyon path from device-space subpaths (subpaths with < 2 points are
/// dropped — nothing to draw).
fn lyon_path(subs: &[SubPath]) -> lyon_tessellation::path::Path {
    use lyon_tessellation::math::point;
    let mut b = lyon_tessellation::path::Path::builder();
    for sub in subs {
        if sub.pts.len() < 2 {
            continue;
        }
        b.begin(point(sub.pts[0].0, sub.pts[0].1));
        for &(x, y) in &sub.pts[1..] {
            b.line_to(point(x, y));
        }
        b.end(sub.closed);
    }
    b.build()
}

impl CanvasBackend for GpuCanvas {
    fn fill(&mut self, subs: &[SubPath], color: Rgba) {
        if color.a == 0 {
            return;
        }
        let path = lyon_path(subs);
        let mut mesh: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let opts = FillOptions::default().with_fill_rule(FillRule::NonZero);
        let ok = FillTessellator::new()
            .tessellate_path(
                &path,
                &opts,
                &mut BuffersBuilder::new(&mut mesh, |v: FillVertex| {
                    let p = v.position();
                    [p.x, p.y]
                }),
            )
            .is_ok();
        if ok {
            self.mesh(Kind::Over, &mesh, color);
        }
    }

    fn stroke(&mut self, subs: &[SubPath], color: Rgba, width: f32) {
        if color.a == 0 || width <= 0.0 {
            return;
        }
        let path = lyon_path(subs);
        let mut mesh: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let opts = StrokeOptions::default().with_line_width(width);
        let ok = StrokeTessellator::new()
            .tessellate_path(
                &path,
                &opts,
                &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| {
                    let p = v.position();
                    [p.x, p.y]
                }),
            )
            .is_ok();
        if ok {
            self.mesh(Kind::Over, &mesh, color);
        }
    }

    fn clear(&mut self, quad: &[(f32, f32); 4]) {
        // Two triangles covering the (possibly skewed) rect, written as zeros.
        let zero = Rgba::new(0, 0, 0, 0);
        self.tri(Kind::Replace, [quad[0], quad[1], quad[2]], zero);
        self.tri(Kind::Replace, [quad[0], quad[2], quad[3]], zero);
    }
}

/// Draw the accumulated geometry into an offscreen texture and read it back to a
/// PNG. An empty geometry still yields the cleared (transparent) texture.
fn render(gpu: &Gpu, width: u32, height: u32, canvas: &GpuCanvas) -> Result<Vec<u8>, String> {
    let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("canvas target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TEX_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

    let vbuf = (!canvas.verts.is_empty()).then(|| {
        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("canvas verts"),
                contents: &f32_bytes(&canvas.verts),
                usage: wgpu::BufferUsages::VERTEX,
            })
    });

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("canvas"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // Canvas initial state is transparent black.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(vbuf) = &vbuf {
            pass.set_vertex_buffer(0, vbuf.slice(..));
            for r in &canvas.ranges {
                let pipeline = match r.kind {
                    Kind::Over => &gpu.pipeline_over,
                    Kind::Replace => &gpu.pipeline_replace,
                };
                pass.set_pipeline(pipeline);
                pass.draw(r.start..r.start + r.count, 0..1);
            }
        }
    }

    // Copy the texture into a readback buffer (rows padded to 256 bytes).
    let unpadded = width * 4;
    let padded = align_up(unpadded, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("canvas readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &readback,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit(Some(encoder.finish()));

    // Map + wait, then un-pad rows into a tight RGBA buffer.
    let (tx, rx) = std::sync::mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| format!("map recv: {e}"))?
        .map_err(|e| format!("buffer map: {e}"))?;

    let mapped = readback.slice(..).get_mapped_range();
    let mut rgba = Vec::with_capacity((unpadded * height) as usize);
    for row in 0..height {
        let start = (row * padded) as usize;
        rgba.extend_from_slice(&mapped[start..start + unpadded as usize]);
    }
    drop(mapped);
    readback.unmap();

    encode_png(width, height, &rgba)
}

/// Round `v` up to the next multiple of `align` (a power of two).
fn align_up(v: u32, align: u32) -> u32 {
    (v + align - 1) & !(align - 1)
}

/// Flatten an f32 slice to little-endian bytes for a wgpu vertex buffer.
fn f32_bytes(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

/// Encode a tight RGBA8 buffer to PNG with the crate's `image` dependency.
fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| format!("png encode: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ops(json: &str) -> Vec<Op> {
        canvas_ops::parse_ops(json).unwrap()
    }

    #[test]
    fn adapter_is_apple_metal() {
        // Proves the render targets the real Apple GPU, not a CPU/software path.
        assert_eq!(adapter_backend().unwrap(), wgpu::Backend::Metal);
        let name = adapter_name().unwrap();
        assert!(!name.is_empty(), "adapter reports a device name ({name})");
    }

    // The op-log is the render tier's `ctx._ops` tuple format:
    // [name, argsArray, fillStyle, strokeStyle, font, textBaseline, textAlign, globalAlpha, comp].
    #[test]
    fn png_decodes_at_requested_size() {
        let png = replay(
            48,
            24,
            &ops(r##"[["fillRect",[0,0,48,24],"#000000","#000","10px x","alphabetic","start",1,"source-over"]]"##),
        )
        .unwrap();
        let img = image::load_from_memory(&png).unwrap();
        assert_eq!((img.width(), img.height()), (48, 24));
    }

    #[test]
    fn distinct_op_lists_give_distinct_bytes() {
        let a = replay(
            24,
            24,
            &ops(r##"[["fillRect",[0,0,24,24],"#000000","#000","10px x","alphabetic","start",1,"source-over"]]"##),
        )
        .unwrap();
        let b = replay(
            24,
            24,
            &ops(r##"[["fillRect",[0,0,6,6],"#000000","#000","10px x","alphabetic","start",1,"source-over"]]"##),
        )
        .unwrap();
        assert_ne!(a, b, "content-dependent pixels");
    }

    #[test]
    fn fillrect_plus_filltext_is_substantial() {
        let json = r##"[
            ["fillRect",[0,0,120,40],"#123456","#000","10px x","alphabetic","start",1,"source-over"],
            ["fillText",["GPU!",6,30],"#ffcc00","#000","22px sans-serif","alphabetic","start",1,"source-over"]
        ]"##;
        let png = replay(120, 40, &ops(json)).unwrap();
        assert!(png.len() > 200, "non-trivial PNG ({} bytes)", png.len());
        // And a pixel inside the rect is the fill colour (real content).
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(img.get_pixel(2, 38).0[3], 255, "rect area is opaque");
    }

    #[test]
    fn empty_ops_are_blank_but_valid() {
        let png = replay(10, 10, &[]).unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!((img.width(), img.height()), (10, 10));
        assert!(img.pixels().all(|p| p.0[3] == 0), "fully transparent");
    }
}
