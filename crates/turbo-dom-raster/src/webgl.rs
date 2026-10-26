//! Opt-in (`gpu-metal`) LIVE WebGL executor: replay a page's own recorded WebGL
//! draw batch on the **real GPU** (wgpu → Metal) and read back genuine framebuffer
//! pixels. Unlike a synthetic canvas replay, this runs the page's *actual* GLSL
//! shaders and vertex data — captured verbatim at the live call site — so a
//! fingerprint draw produces the same content-dependent image a browser would.
//!
//! The whole file is gated behind `gpu-metal`, so the default build never links
//! wgpu/naga/base64. The page's GLSL ES sources are translated to WGSL via naga
//! (`front::glsl` → IR → validate → `back::wgsl`); anything the translator or the
//! GPU can't handle returns `None` so the JS side keeps its stub (honest fallback,
//! never a silent no-op or a panic).
//!
//! Orientation: output is **top-left origin**, row-major, tightly packed RGBA8.
//! wgpu clip space is y-up with +1 mapping to the top framebuffer row, and the
//! texture readback yields row 0 = top — i.e. the on-screen (top-left) image — so
//! no vertical flip is applied. (WebGL `readPixels` is bottom-left; the caller and
//! the JS recorder must hash on this top-left orientation to stay consistent.)

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use base64::Engine;
use wgpu::util::DeviceExt;

// GL enum constants used by the call protocol.
const GL_VERTEX_SHADER: i64 = 0x8B31;
const GL_FRAGMENT_SHADER: i64 = 0x8B30;
const GL_ARRAY_BUFFER: i64 = 0x8892;
const GL_ELEMENT_ARRAY_BUFFER: i64 = 0x8893;
const GL_FLOAT: i64 = 0x1406;
const GL_UNSIGNED_SHORT: i64 = 0x1403;
const GL_UNSIGNED_BYTE: i64 = 0x1401;
const GL_TRIANGLES: i64 = 0x0004;
const GL_TRIANGLE_STRIP: i64 = 0x0005;
const GL_TRIANGLE_FAN: i64 = 0x0006;

/// The offscreen render-target format. `Unorm` (not `Srgb`) so the shader's output
/// colour bytes are stored verbatim, matching WebGL's un-managed default backbuffer.
const TEX_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Execute a recorded WebGL call batch on the GPU; return the framebuffer as RGBA8
/// bytes (len `width*height*4`, row-major, top-left origin). `None` on any failure
/// (the caller falls back to its stub). Never panics.
pub fn webgl_readback(width: u32, height: u32, calls_json: &str) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    // A guard against any panic in naga/lyon/wgpu escaping the API boundary.
    std::panic::catch_unwind(|| match run(width, height, calls_json) {
        Ok(v) => Some(v),
        Err(e) => {
            if std::env::var("TURBO_SURF_WEBGL_DEBUG").is_ok() {
                eprintln!("webgl_readback error: {e}");
            }
            None
        }
    })
    .ok()
    .flatten()
}

/// Fallible core: parse the batch, build the GPU state, render, read back.
fn run(width: u32, height: u32, calls_json: &str) -> Result<Vec<u8>, String> {
    let calls: Vec<serde_json::Value> =
        serde_json::from_str(calls_json).map_err(|e| format!("bad calls json: {e}"))?;
    let mut state = State::default();
    for call in &calls {
        state.apply(call);
    }
    // An empty draw list still renders: the caller gets the cleared framebuffer (a
    // deterministic clear-only WebGL frame), not a fallback.
    let gpu = gpu()?;
    render(gpu, width, height, &state)
}

// ---------------------------------------------------------------------------
// Recorder replay: turn the JSON call log into GPU-ready draw commands.
// ---------------------------------------------------------------------------

/// A compiled-shader record (source captured verbatim).
struct Shader {
    stage: ShaderKind,
    source: String,
}

#[derive(Clone, Copy, PartialEq)]
enum ShaderKind {
    Vertex,
    Fragment,
}

/// A uniform value set by name (last-write-wins per program).
#[derive(Clone)]
enum UniformVal {
    F(Vec<f32>), // 1f/2f/3f/4f
    I(i32),      // 1i
    Mat4([f32; 16]),
}

/// A linked program: its two shaders, the attribute-location bindings the recorder
/// handed the page, and the uniform values set on it.
#[derive(Default)]
struct Program {
    vs: Option<i64>,
    fs: Option<i64>,
    attrib_locs: HashMap<String, u32>,
    uniforms: HashMap<String, UniformVal>,
}

/// A bound vertex attribute pointer (captures the ARRAY_BUFFER live at the time).
#[derive(Clone)]
struct AttribPtr {
    buffer: i64,
    size: u32,
    ty: i64,
    stride: u32,
    offset: u64,
}

/// One recorded draw, snapshotting all state it depends on.
struct Draw {
    program: i64,
    attribs: Vec<(u32, AttribPtr)>, // (location, pointer) for enabled attribs
    uniforms: HashMap<String, UniformVal>,
    mode: i64,
    kind: DrawKind,
}

enum DrawKind {
    Arrays {
        first: u32,
        count: u32,
    },
    Elements {
        count: u32,
        ty: i64,
        offset: u64,
        buffer: i64,
    },
}

/// The state a draw captures at issue time: `(program, enabled attribs, uniforms)`.
type DrawSnapshot = (i64, Vec<(u32, AttribPtr)>, HashMap<String, UniformVal>);

/// A prepared index buffer for a draw: `(buffer, format, index count)`.
type IndexBuf = (wgpu::Buffer, wgpu::IndexFormat, u32);

#[derive(Default)]
struct State {
    shaders: HashMap<i64, Shader>,
    programs: HashMap<i64, Program>,
    buffers: HashMap<i64, Vec<u8>>,
    bound_array: Option<i64>,
    bound_element: Option<i64>,
    cur_program: Option<i64>,
    clear_color: [f32; 4],
    attribs: HashMap<u32, AttribPtr>,
    enabled: HashSet<u32>,
    draws: Vec<Draw>,
}

impl State {
    /// Apply one recorded call to the state machine. Unknown ops are ignored.
    fn apply(&mut self, call: &serde_json::Value) {
        let m = call.get("m").and_then(|v| v.as_str()).unwrap_or("");
        let a = call.get("a");
        let id = call.get("id").and_then(serde_json::Value::as_i64);
        match m {
            "clearColor" => {
                if let Some(c) = a.and_then(f32x4) {
                    self.clear_color = c;
                }
            }
            "createShader" => {
                if let (Some(h), Some(t)) = (id, arg_i64(a, 0)) {
                    let stage = match t {
                        GL_FRAGMENT_SHADER => ShaderKind::Fragment,
                        GL_VERTEX_SHADER => ShaderKind::Vertex,
                        _ => return, // unknown shader type: ignore
                    };
                    self.shaders.insert(
                        h,
                        Shader {
                            stage,
                            source: String::new(),
                        },
                    );
                }
            }
            "shaderSource" => {
                if let (Some(h), Some(src)) = (arg_i64(a, 0), arg_str(a, 1)) {
                    if let Some(s) = self.shaders.get_mut(&h) {
                        s.source = src;
                    }
                }
            }
            "createProgram" => {
                if let Some(h) = id {
                    self.programs.insert(h, Program::default());
                }
            }
            "attachShader" => {
                if let (Some(p), Some(sh)) = (arg_i64(a, 0), arg_i64(a, 1)) {
                    let stage = self.shaders.get(&sh).map(|s| s.stage);
                    if let (Some(prog), Some(stage)) = (self.programs.get_mut(&p), stage) {
                        match stage {
                            ShaderKind::Vertex => prog.vs = Some(sh),
                            ShaderKind::Fragment => prog.fs = Some(sh),
                        }
                    }
                }
            }
            "attribLocation" => {
                if let (Some(p), Some(loc), Some(name)) =
                    (arg_i64(a, 0), arg_i64(a, 1), arg_str(a, 2))
                {
                    if let Some(prog) = self.programs.get_mut(&p) {
                        prog.attrib_locs.insert(name, loc as u32);
                    }
                }
            }
            "uniform" => self.apply_uniform(a),
            "useProgram" => self.cur_program = arg_i64(a, 0),
            "createBuffer" => {
                if let Some(h) = id {
                    self.buffers.insert(h, Vec::new());
                }
            }
            "bindBuffer" => {
                let target = arg_i64(a, 0);
                let buf = arg_i64(a, 1);
                match target {
                    Some(GL_ARRAY_BUFFER) => self.bound_array = buf,
                    Some(GL_ELEMENT_ARRAY_BUFFER) => self.bound_element = buf,
                    _ => {}
                }
            }
            "bufferData" => self.apply_buffer_data(a),
            "enableVertexAttribArray" => {
                if let Some(loc) = arg_i64(a, 0) {
                    self.enabled.insert(loc as u32);
                }
            }
            "vertexAttribPointer" => self.apply_vertex_attrib_pointer(a),
            "drawArrays" => self.apply_draw_arrays(a),
            "drawElements" => self.apply_draw_elements(a),
            // createShader/compile/link have no extra state to record here beyond
            // the source we already keep; viewport/clear/enable/etc. unaffected.
            _ => {}
        }
    }

    fn apply_uniform(&mut self, a: Option<&serde_json::Value>) {
        let (Some(prog_h), Some(name), Some(kind)) = (arg_i64(a, 0), arg_str(a, 1), arg_str(a, 2))
        else {
            return;
        };
        let Some(vals) = a.and_then(|a| a.get(3)).and_then(|v| v.as_array()) else {
            return;
        };
        let floats: Vec<f32> = vals
            .iter()
            .filter_map(serde_json::Value::as_f64)
            .map(|f| f as f32)
            .collect();
        let val = match kind.as_str() {
            "1f" | "2f" | "3f" | "4f" => UniformVal::F(floats),
            "1i" => UniformVal::I(floats.first().copied().unwrap_or(0.0) as i32),
            "Matrix4fv" => {
                let mut m = [0.0f32; 16];
                for (dst, src) in m.iter_mut().zip(floats.iter()) {
                    *dst = *src;
                }
                UniformVal::Mat4(m)
            }
            _ => return,
        };
        if let Some(prog) = self.programs.get_mut(&prog_h) {
            prog.uniforms.insert(name, val);
        }
    }

    fn apply_buffer_data(&mut self, a: Option<&serde_json::Value>) {
        let Some(target) = arg_i64(a, 0) else { return };
        let handle = match target {
            GL_ARRAY_BUFFER => self.bound_array,
            GL_ELEMENT_ARRAY_BUFFER => self.bound_element,
            _ => None,
        };
        let Some(handle) = handle else { return };
        let Some(obj) = a.and_then(|a| a.get(1)) else {
            return;
        };
        if let Some(bytes) = decode_buffer(obj) {
            self.buffers.insert(handle, bytes);
        }
    }

    fn apply_vertex_attrib_pointer(&mut self, a: Option<&serde_json::Value>) {
        let (Some(loc), Some(size), Some(ty)) = (arg_i64(a, 0), arg_i64(a, 1), arg_i64(a, 2))
        else {
            return;
        };
        let Some(buffer) = self.bound_array else {
            return;
        };
        let stride = arg_i64(a, 4).unwrap_or(0).max(0) as u32;
        let offset = arg_i64(a, 5).unwrap_or(0).max(0) as u64;
        self.attribs.insert(
            loc as u32,
            AttribPtr {
                buffer,
                size: size as u32,
                ty,
                stride,
                offset,
            },
        );
    }

    /// Snapshot the currently enabled attribs + program uniforms for a draw.
    fn snapshot(&self) -> Option<DrawSnapshot> {
        let prog = self.cur_program?;
        let mut attribs: Vec<(u32, AttribPtr)> = self
            .enabled
            .iter()
            .filter_map(|loc| self.attribs.get(loc).map(|p| (*loc, p.clone())))
            .collect();
        attribs.sort_by_key(|(loc, _)| *loc);
        let uniforms = self
            .programs
            .get(&prog)
            .map(|p| p.uniforms.clone())
            .unwrap_or_default();
        Some((prog, attribs, uniforms))
    }

    fn apply_draw_arrays(&mut self, a: Option<&serde_json::Value>) {
        let (Some(mode), Some(first), Some(count)) = (arg_i64(a, 0), arg_i64(a, 1), arg_i64(a, 2))
        else {
            return;
        };
        let Some((program, attribs, uniforms)) = self.snapshot() else {
            return;
        };
        self.draws.push(Draw {
            program,
            attribs,
            uniforms,
            mode,
            kind: DrawKind::Arrays {
                first: first.max(0) as u32,
                count: count.max(0) as u32,
            },
        });
    }

    fn apply_draw_elements(&mut self, a: Option<&serde_json::Value>) {
        let (Some(mode), Some(count), Some(ty), Some(offset)) =
            (arg_i64(a, 0), arg_i64(a, 1), arg_i64(a, 2), arg_i64(a, 3))
        else {
            return;
        };
        let Some(buffer) = self.bound_element else {
            return;
        };
        let Some((program, attribs, uniforms)) = self.snapshot() else {
            return;
        };
        self.draws.push(Draw {
            program,
            attribs,
            uniforms,
            mode,
            kind: DrawKind::Elements {
                count: count.max(0) as u32,
                ty,
                offset: offset.max(0) as u64,
                buffer,
            },
        });
    }
}

/// Decode a `bufferData` payload object `{"b64":"...","t":"f32|u16|u8"}` to bytes.
/// The bytes are already the little-endian in-memory layout, so `t` is advisory
/// (we store the raw bytes and reinterpret at bind time).
fn decode_buffer(obj: &serde_json::Value) -> Option<Vec<u8>> {
    let b64 = obj.get("b64").and_then(|v| v.as_str())?;
    base64::engine::general_purpose::STANDARD.decode(b64).ok()
}

// ---------------------------------------------------------------------------
// GLSL ES → WGSL translation (naga).
// ---------------------------------------------------------------------------

/// A uniform block field (name + std140 slot) shared by both stages of a program.
struct UboField {
    name: String,
    ty: UboType,
    offset: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum UboType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Int,
    Mat4,
}

impl UboType {
    fn glsl(self) -> &'static str {
        match self {
            UboType::Float => "float",
            UboType::Vec2 => "vec2",
            UboType::Vec3 => "vec3",
            UboType::Vec4 => "vec4",
            UboType::Int => "int",
            UboType::Mat4 => "mat4",
        }
    }
    /// (std140 alignment, size) in bytes.
    fn align_size(self) -> (u32, u32) {
        match self {
            UboType::Float | UboType::Int => (4, 4),
            UboType::Vec2 => (8, 8),
            UboType::Vec3 => (16, 12),
            UboType::Vec4 => (16, 16),
            UboType::Mat4 => (16, 64),
        }
    }
}

/// The uniform layout of a program: the ordered fields + total (16-aligned) size.
struct Ubo {
    fields: Vec<UboField>,
    size: u32,
}

/// Everything needed to build a pipeline for one program.
struct TranslatedProgram {
    vs_wgsl: String,
    fs_wgsl: String,
    ubo: Option<Ubo>,
}

/// Round `v` up to a multiple of `align` (a power of two).
fn align_up(v: u32, align: u32) -> u32 {
    (v + align - 1) & !(align - 1)
}

/// Collect the `uniform <type> <name>;` declarations across both stages into a
/// single std140 block. Returns `Err` if a sampler/opaque uniform is present (we
/// don't support textures) — the whole batch then falls back.
fn build_ubo(vs: &str, fs: &str) -> Result<Option<Ubo>, String> {
    let mut fields: Vec<UboField> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for src in [vs, fs] {
        for (ty, name) in scan_uniforms(src)? {
            if seen.insert(name.clone()) {
                fields.push(UboField {
                    name,
                    ty,
                    offset: 0,
                });
            }
        }
    }
    if fields.is_empty() {
        return Ok(None);
    }
    let mut cursor = 0u32;
    for f in &mut fields {
        let (align, size) = f.ty.align_size();
        cursor = align_up(cursor, align);
        f.offset = cursor;
        cursor += size;
    }
    Ok(Some(Ubo {
        fields,
        size: align_up(cursor, 16).max(16),
    }))
}

/// Scan a GLSL source for scalar/vector/matrix `uniform` declarations. A sampler
/// uniform yields `Err` (unsupported). Very small hand parser: `uniform T N;`.
fn scan_uniforms(src: &str) -> Result<Vec<(UboType, String)>, String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("uniform ") else {
            continue;
        };
        let rest = rest.trim().trim_end_matches(';');
        let mut it = rest.split_whitespace();
        let (Some(ty), Some(name)) = (it.next(), it.next()) else {
            continue;
        };
        let name = name.trim_end_matches(';').trim();
        // Reject arrays / structs / unknowns we can't lay out.
        if name.contains('[') {
            return Err(format!("unsupported uniform array: {name}"));
        }
        let ty = match ty {
            "float" | "highp" | "mediump" | "lowp" => {
                // A precision-qualified declaration: re-read after the qualifier.
                if ty == "float" {
                    UboType::Float
                } else {
                    let real_ty = name; // the token after the precision qualifier
                    let real_name = it.next().unwrap_or("").trim_end_matches(';');
                    let ub = map_ubo_type(real_ty)?;
                    out.push((ub, real_name.to_string()));
                    continue;
                }
            }
            other => map_ubo_type(other)?,
        };
        out.push((ty, name.to_string()));
    }
    Ok(out)
}

fn map_ubo_type(ty: &str) -> Result<UboType, String> {
    match ty {
        "float" => Ok(UboType::Float),
        "vec2" => Ok(UboType::Vec2),
        "vec3" => Ok(UboType::Vec3),
        "vec4" => Ok(UboType::Vec4),
        "int" => Ok(UboType::Int),
        "mat4" => Ok(UboType::Mat4),
        // Samplers/opaque types can't be packed into a UBO: bail to fallback.
        other if other.contains("sampler") => Err(format!("sampler uniform unsupported: {other}")),
        other => Err(format!("unsupported uniform type: {other}")),
    }
}

/// Translate one program's GLSL ES vertex + fragment sources to WGSL.
fn translate_program(
    vs_src: &str,
    fs_src: &str,
    attribs: &HashMap<String, u32>,
) -> Option<TranslatedProgram> {
    let ubo = build_ubo(vs_src, fs_src).ok()?;
    // Shared varying → location map (consistent between the two stages).
    let mut varyings: Vec<String> = Vec::new();
    for src in [vs_src, fs_src] {
        for name in scan_varyings(src) {
            if !varyings.contains(&name) {
                varyings.push(name);
            }
        }
    }
    let vs_glsl = rewrite(vs_src, ShaderKind::Vertex, attribs, &varyings, ubo.as_ref());
    let fs_glsl = rewrite(
        fs_src,
        ShaderKind::Fragment,
        attribs,
        &varyings,
        ubo.as_ref(),
    );
    let vs_wgsl = glsl_to_wgsl(&vs_glsl, naga::ShaderStage::Vertex)?;
    let fs_wgsl = glsl_to_wgsl(&fs_glsl, naga::ShaderStage::Fragment)?;
    Some(TranslatedProgram {
        vs_wgsl,
        fs_wgsl,
        ubo,
    })
}

/// Collect `varying <type> <name>;` names (ignoring precision qualifiers).
fn scan_varyings(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("varying ") else {
            continue;
        };
        let rest = rest.trim().trim_end_matches(';');
        let toks: Vec<&str> = rest.split_whitespace().collect();
        if let Some(name) = toks.last() {
            out.push(name.trim_end_matches(';').to_string());
        }
    }
    out
}

/// Rewrite a GLSL ES 1.00 source into desktop GLSL 450 that naga's frontend
/// accepts: drop `#version`/`precision`, promote `attribute`/`varying` to located
/// `in`/`out`, lift `gl_FragColor` to a declared output, and fold `uniform`
/// declarations into references to the shared UBO block.
fn rewrite(
    src: &str,
    stage: ShaderKind,
    attribs: &HashMap<String, u32>,
    varyings: &[String],
    ubo: Option<&Ubo>,
) -> String {
    let mut body = String::new();
    let mut next_attrib_loc = 0u32;
    // GLSL declarations (attribute/varying/uniform/precision) are matched per LINE, but a real
    // shader often puts several on one line separated by `;` (e.g. minified fingerprint shaders:
    // `attribute vec2 p; void main(){...}`). Pre-split so each top-level (`;`-terminated, OUTSIDE
    // any `{}` body) statement is its own line; semicolons inside the function body (brace depth
    // > 0) are left intact so the body stays verbatim.
    let normalized = split_top_level_statements(src);
    for raw in normalized.lines() {
        let line = raw.trim();
        if line.starts_with("#version") || line.starts_with("precision ") {
            continue;
        }
        if let Some(decl) = line.strip_prefix("attribute ") {
            if stage == ShaderKind::Vertex {
                let (ty, name) = split_decl(decl);
                let loc = attribs.get(&name).copied().unwrap_or_else(|| {
                    let l = next_attrib_loc;
                    next_attrib_loc += 1;
                    l
                });
                body.push_str(&format!("layout(location={loc}) in {ty} {name};\n"));
                continue;
            }
        }
        if let Some(decl) = line.strip_prefix("varying ") {
            let (ty, name) = split_decl(decl);
            let loc = varyings.iter().position(|v| v == &name).unwrap_or(0);
            let dir = if stage == ShaderKind::Vertex {
                "out"
            } else {
                "in"
            };
            body.push_str(&format!("layout(location={loc}) {dir} {ty} {name};\n"));
            continue;
        }
        if line.starts_with("uniform ") {
            // Folded into the UBO block; drop the standalone declaration.
            continue;
        }
        body.push_str(raw);
        body.push('\n');
    }

    // Rewrite references in the BODY only (never inside the UBO block declaration
    // we are about to prepend, whose field names must stay bare).
    if let Some(ubo) = ubo {
        for f in &ubo.fields {
            body = replace_ident(&body, &f.name, &format!("_u.{}", f.name));
        }
    }
    let uses_frag_color = stage == ShaderKind::Fragment && body.contains("gl_FragColor");
    if uses_frag_color {
        body = replace_ident(&body, "gl_FragColor", "_glFragColor");
    }

    // Header: version, the UBO block, and (fragment) the lifted gl_FragColor out.
    let mut out = String::from("#version 450\n");
    if let Some(ubo) = ubo {
        out.push_str("layout(set=0, binding=0) uniform _Globals {\n");
        for f in &ubo.fields {
            out.push_str(&format!("    {} {};\n", f.ty.glsl(), f.name));
        }
        out.push_str("} _u;\n");
    }
    if uses_frag_color {
        out.push_str("layout(location=0) out vec4 _glFragColor;\n");
    }
    out.push_str(&body);
    out
}

/// Split a `<type> <name>` declaration tail (with optional precision qualifier and
/// trailing `;`) into `(type, name)`.
fn split_decl(decl: &str) -> (String, String) {
    let decl = decl.trim().trim_end_matches(';');
    let toks: Vec<&str> = decl.split_whitespace().collect();
    match toks.as_slice() {
        [.., ty, name] => (ty.to_string(), name.to_string()),
        [one] => (one.to_string(), String::new()),
        _ => (String::new(), String::new()),
    }
}

/// Whole-identifier replace (GLSL identifier chars are `[A-Za-z0-9_]`).
fn replace_ident(src: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return src.to_string();
    }
    let bytes = src.as_bytes();
    let from_b = from.as_bytes();
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(from_b) {
            let before_ok = i == 0 || !is_ident(bytes[i - 1]);
            let after = i + from_b.len();
            let after_ok = after >= bytes.len() || !is_ident(bytes[after]);
            if before_ok && after_ok {
                out.push_str(to);
                i = after;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Put each top-level (brace-depth-0) `;`-terminated statement — and each `{`/`}` — on its own
/// line, so the line-based declaration rewrite sees one declaration per line even when the source
/// is minified onto a single line. Semicolons inside a `{}` body (depth > 0) are preserved so the
/// function body is emitted verbatim.
fn split_top_level_statements(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 16);
    let mut depth: i32 = 0;
    for ch in src.chars() {
        match ch {
            '{' => {
                depth += 1;
                out.push(ch);
            }
            '}' => {
                depth -= 1;
                out.push(ch);
            }
            ';' if depth <= 0 => {
                out.push(';');
                out.push('\n');
            }
            _ => out.push(ch),
        }
    }
    out
}

/// GLSL (desktop 450) → naga IR → validate → WGSL string.
fn glsl_to_wgsl(src: &str, stage: naga::ShaderStage) -> Option<String> {
    let dbg = std::env::var("TURBO_SURF_WEBGL_DEBUG").is_ok();
    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options::from(stage);
    let module = match frontend.parse(&options, src) {
        Ok(m) => m,
        Err(e) => {
            if dbg {
                eprintln!(
                    "naga glsl parse error ({stage:?}): {e:?}\n--- rewritten glsl ---\n{src}\n---"
                );
            }
            return None;
        }
    };
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = match validator.validate(&module) {
        Ok(i) => i,
        Err(e) => {
            if dbg {
                eprintln!("naga validate error ({stage:?}): {e:?}");
            }
            return None;
        }
    };
    naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::empty()).ok()
}

// ---------------------------------------------------------------------------
// GPU context + render.
// ---------------------------------------------------------------------------

/// Process-global GPU context (memoized like `paint_canvas_gpu`).
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    _adapter: wgpu::Adapter,
    _instance: wgpu::Instance,
}

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
                label: Some("turbo-surf webgl"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .map_err(|e| format!("request_device: {e}"))?;
        Ok(Gpu {
            device,
            queue,
            info,
            _adapter: adapter,
            _instance: instance,
        })
    }
}

#[cfg(test)]
fn adapter_backend() -> Result<wgpu::Backend, String> {
    Ok(gpu()?.info.backend)
}

/// Identity of the actual Metal adapter used for readback, in WebGL's ANGLE
/// representation. A failed device initialization exposes no fabricated identity.
pub fn webgl_metadata() -> Option<String> {
    let info = &gpu().ok()?.info;
    Some(serde_json::json!({
        "name": info.name,
        "backend": "Metal",
        "vendor": "Google Inc. (Apple)",
        "renderer": format!("ANGLE (Apple, ANGLE Metal Renderer: {}, Unspecified Version)", info.name),
    }).to_string())
}

/// wgpu index format for a GL element type (U8 is widened to U16 on the host).
fn index_format(ty: i64) -> Option<wgpu::IndexFormat> {
    match ty {
        GL_UNSIGNED_SHORT | GL_UNSIGNED_BYTE => Some(wgpu::IndexFormat::Uint16),
        _ => None,
    }
}

/// wgpu vertex format for a (size, GL type) attribute — only FLOAT is supported.
fn vertex_format(size: u32, ty: i64) -> Option<wgpu::VertexFormat> {
    if ty != GL_FLOAT {
        return None;
    }
    match size {
        1 => Some(wgpu::VertexFormat::Float32),
        2 => Some(wgpu::VertexFormat::Float32x2),
        3 => Some(wgpu::VertexFormat::Float32x3),
        4 => Some(wgpu::VertexFormat::Float32x4),
        _ => None,
    }
}

/// wgpu primitive topology for a GL draw mode. FAN has no wgpu equivalent, so the
/// caller expands it to an indexed triangle list; here it maps to TriangleList.
fn topology(mode: i64) -> Option<wgpu::PrimitiveTopology> {
    match mode {
        GL_TRIANGLES | GL_TRIANGLE_FAN => Some(wgpu::PrimitiveTopology::TriangleList),
        GL_TRIANGLE_STRIP => Some(wgpu::PrimitiveTopology::TriangleStrip),
        _ => None,
    }
}

/// Bytes-per-component for a vertex attribute (only FLOAT supported → 4).
fn component_bytes(ty: i64) -> u64 {
    match ty {
        GL_FLOAT => 4,
        _ => 4,
    }
}

/// Fill a program's UBO buffer bytes from the recorded uniform values.
fn ubo_bytes(ubo: &Ubo, uniforms: &HashMap<String, UniformVal>) -> Vec<u8> {
    let mut buf = vec![0u8; ubo.size as usize];
    for f in &ubo.fields {
        let Some(val) = uniforms.get(&f.name) else {
            continue;
        };
        let off = f.offset as usize;
        match (f.ty, val) {
            (UboType::Float | UboType::Vec2 | UboType::Vec3 | UboType::Vec4, UniformVal::F(v)) => {
                for (i, x) in v.iter().enumerate() {
                    write_f32(&mut buf, off + i * 4, *x);
                }
            }
            (UboType::Int, UniformVal::I(i)) => {
                buf[off..off + 4].copy_from_slice(&i.to_le_bytes());
            }
            (UboType::Mat4, UniformVal::Mat4(m)) => {
                for (i, x) in m.iter().enumerate() {
                    write_f32(&mut buf, off + i * 4, *x);
                }
            }
            _ => {}
        }
    }
    buf
}

fn write_f32(buf: &mut [u8], off: usize, x: f32) {
    if off + 4 <= buf.len() {
        buf[off..off + 4].copy_from_slice(&x.to_le_bytes());
    }
}

/// Build the offscreen texture, replay the draws, read RGBA back (top-left origin).
fn render(gpu: &Gpu, width: u32, height: u32, state: &State) -> Result<Vec<u8>, String> {
    let device = &gpu.device;
    device.push_error_scope(wgpu::ErrorFilter::Validation);

    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("webgl target"),
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

    // Translate every program used by a draw (once each).
    let mut translated: HashMap<i64, TranslatedProgram> = HashMap::new();
    for d in &state.draws {
        if translated.contains_key(&d.program) {
            continue;
        }
        let prog = state
            .programs
            .get(&d.program)
            .ok_or_else(|| "draw references unknown program".to_string())?;
        let vs = prog
            .vs
            .and_then(|h| state.shaders.get(&h))
            .ok_or_else(|| "program missing vertex shader".to_string())?;
        let fs = prog
            .fs
            .and_then(|h| state.shaders.get(&h))
            .ok_or_else(|| "program missing fragment shader".to_string())?;
        let t = translate_program(&vs.source, &fs.source, &prog.attrib_locs)
            .ok_or_else(|| "shader translation failed".to_string())?;
        translated.insert(d.program, t);
    }

    // Build the per-draw GPU resources.
    let mut built: Vec<BuiltDraw> = Vec::with_capacity(state.draws.len());
    for d in &state.draws {
        let t = &translated[&d.program];
        built.push(build_draw(device, state, d, t)?);
    }

    let clear = wgpu::Color {
        r: state.clear_color[0] as f64,
        g: state.clear_color[1] as f64,
        b: state.clear_color[2] as f64,
        a: state.clear_color[3] as f64,
    };

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("webgl"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        for b in &built {
            pass.set_pipeline(&b.pipeline);
            if let Some(bg) = &b.bind_group {
                pass.set_bind_group(0, bg, &[]);
            }
            for (slot, buf, start) in &b.vertex_buffers {
                pass.set_vertex_buffer(*slot, buf.slice(*start..));
            }
            match &b.index {
                Some((ibuf, fmt, n)) => {
                    pass.set_index_buffer(ibuf.slice(..), *fmt);
                    pass.draw_indexed(0..*n, 0, 0..1);
                }
                None => pass.draw(b.vertex_range.clone(), 0..1),
            }
        }
    }

    let rgba = readback(gpu, &mut encoder, &tex, width, height);
    gpu.queue.submit(Some(encoder.finish()));
    let rgba = rgba(gpu)?;

    if let Some(err) = pollster::block_on(device.pop_error_scope()) {
        return Err(format!("gpu validation: {err}"));
    }
    Ok(rgba)
}

/// GPU resources for a single draw.
struct BuiltDraw {
    pipeline: wgpu::RenderPipeline,
    bind_group: Option<wgpu::BindGroup>,
    vertex_buffers: Vec<(u32, wgpu::Buffer, u64)>, // (slot, buffer, byte start)
    index: Option<IndexBuf>,
    vertex_range: std::ops::Range<u32>,
}

fn build_draw(
    device: &wgpu::Device,
    state: &State,
    d: &Draw,
    t: &TranslatedProgram,
) -> Result<BuiltDraw, String> {
    let topo = topology(d.mode).ok_or_else(|| format!("unsupported draw mode {}", d.mode))?;

    let vs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("webgl vs"),
        source: wgpu::ShaderSource::Wgsl(t.vs_wgsl.as_str().into()),
    });
    let fs = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("webgl fs"),
        source: wgpu::ShaderSource::Wgsl(t.fs_wgsl.as_str().into()),
    });

    // One wgpu vertex buffer slot per enabled attribute (handles both separate and
    // interleaved GL buffers: interleaved attribs share bytes but bind at distinct
    // byte offsets/strides).
    let mut layouts: Vec<VbLayout> = Vec::new();
    let mut vertex_buffers: Vec<(u32, wgpu::Buffer, u64)> = Vec::new();
    for (slot, (loc, ptr)) in d.attribs.iter().enumerate() {
        let format = vertex_format(ptr.size, ptr.ty)
            .ok_or_else(|| format!("unsupported attrib format size={} ty={}", ptr.size, ptr.ty))?;
        let comp = component_bytes(ptr.ty);
        let stride = if ptr.stride > 0 {
            ptr.stride as u64
        } else {
            comp * ptr.size as u64
        };
        let bytes = state
            .buffers
            .get(&ptr.buffer)
            .ok_or_else(|| "attrib references unknown buffer".to_string())?;
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("webgl vbo"),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        layouts.push(VbLayout::new(stride, *loc, format));
        vertex_buffers.push((slot as u32, buf, ptr.offset));
    }

    // Pipeline layout: a single uniform bind group if the program has uniforms.
    let (pipeline_layout, bind_group, bgl) = build_bindings(device, state, d, t);

    let vb_layouts: Vec<wgpu::VertexBufferLayout> = layouts
        .iter()
        .map(|l| wgpu::VertexBufferLayout {
            array_stride: l.array_stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: std::slice::from_ref(&l.attr),
        })
        .collect();

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("webgl"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vs,
            entry_point: "main",
            compilation_options: Default::default(),
            buffers: &vb_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &fs,
            entry_point: "main",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: TEX_FORMAT,
                // WebGL blending defaults to off (opaque overwrite).
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: topo,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    let _ = bgl; // kept alive only through pipeline creation

    // Index buffer (drawElements or a fan expanded to an index list).
    let (index, vertex_range) = build_indices(device, state, d)?;

    Ok(BuiltDraw {
        pipeline,
        bind_group,
        vertex_buffers,
        index,
        vertex_range,
    })
}

/// A vertex buffer layout with its single attribute (stored so the borrow of the
/// `wgpu::VertexAttribute` outlives pipeline creation).
struct VbLayout {
    array_stride: u64,
    attr: wgpu::VertexAttribute,
}

impl VbLayout {
    fn new(array_stride: u64, location: u32, format: wgpu::VertexFormat) -> Self {
        VbLayout {
            array_stride,
            attr: wgpu::VertexAttribute {
                format,
                offset: 0,
                shader_location: location,
            },
        }
    }
}

/// Build the pipeline layout + optional uniform bind group for a draw.
fn build_bindings(
    device: &wgpu::Device,
    _state: &State,
    d: &Draw,
    t: &TranslatedProgram,
) -> (
    wgpu::PipelineLayout,
    Option<wgpu::BindGroup>,
    Option<wgpu::BindGroupLayout>,
) {
    let Some(ubo) = &t.ubo else {
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("webgl (no uniforms)"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        return (pl, None, None);
    };
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("webgl ubo"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("webgl ubo data"),
        contents: &ubo_bytes(ubo, &d.uniforms),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("webgl ubo"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buf.as_entire_binding(),
        }],
    });
    let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("webgl"),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    (pl, Some(bg), Some(bgl))
}

/// Produce the index buffer + draw range for a draw. drawArrays with TRIANGLES/
/// STRIP draws non-indexed; TRIANGLE_FAN is expanded to an indexed triangle list
/// (wgpu has no fan topology). drawElements builds an index buffer from the bound
/// ELEMENT_ARRAY_BUFFER (widening U8 → U16, expanding a fan).
fn build_indices(
    device: &wgpu::Device,
    state: &State,
    d: &Draw,
) -> Result<(Option<IndexBuf>, std::ops::Range<u32>), String> {
    match &d.kind {
        DrawKind::Arrays { first, count } => {
            if d.mode == GL_TRIANGLE_FAN {
                let idx = fan_indices((*first..first + count).map(|i| i as u16).collect());
                let buf = make_index_buffer(device, &idx);
                Ok((
                    Some((buf, wgpu::IndexFormat::Uint16, idx.len() as u32)),
                    0..0,
                ))
            } else {
                Ok((None, *first..first + count))
            }
        }
        DrawKind::Elements {
            count,
            ty,
            offset,
            buffer,
        } => {
            let bytes = state
                .buffers
                .get(buffer)
                .ok_or_else(|| "drawElements references unknown buffer".to_string())?;
            let idx = read_indices(bytes, *ty, *count, *offset)?;
            let idx = if d.mode == GL_TRIANGLE_FAN {
                fan_indices(idx)
            } else {
                idx
            };
            let _ = index_format(*ty).ok_or_else(|| format!("bad index type {ty}"))?;
            let buf = make_index_buffer(device, &idx);
            Ok((
                Some((buf, wgpu::IndexFormat::Uint16, idx.len() as u32)),
                0..0,
            ))
        }
    }
}

/// Expand a fan vertex order (v0, v1, v2, v3, …) to triangle-list indices
/// (v0,v1,v2, v0,v2,v3, …).
fn fan_indices(order: Vec<u16>) -> Vec<u16> {
    let mut out = Vec::new();
    if order.len() < 3 {
        return out;
    }
    for i in 1..order.len() - 1 {
        out.push(order[0]);
        out.push(order[i]);
        out.push(order[i + 1]);
    }
    out
}

/// Read `count` indices from an element buffer at `offset` bytes, as U16 values
/// (U8 is widened). U16 must be 2-byte aligned in the source.
fn read_indices(bytes: &[u8], ty: i64, count: u32, offset: u64) -> Result<Vec<u16>, String> {
    let off = offset as usize;
    let count = count as usize;
    let mut out = Vec::with_capacity(count);
    match ty {
        GL_UNSIGNED_BYTE => {
            for i in 0..count {
                let b = *bytes.get(off + i).ok_or("index oob")?;
                out.push(b as u16);
            }
        }
        GL_UNSIGNED_SHORT => {
            for i in 0..count {
                let p = off + i * 2;
                let lo = *bytes.get(p).ok_or("index oob")?;
                let hi = *bytes.get(p + 1).ok_or("index oob")?;
                out.push(u16::from_le_bytes([lo, hi]));
            }
        }
        _ => return Err(format!("bad index type {ty}")),
    }
    Ok(out)
}

fn make_index_buffer(device: &wgpu::Device, idx: &[u16]) -> wgpu::Buffer {
    let mut bytes = Vec::with_capacity(idx.len() * 2);
    for i in idx {
        bytes.extend_from_slice(&i.to_le_bytes());
    }
    // wgpu requires index buffers to be at least COPY_BUFFER_ALIGNMENT (4) bytes and
    // a multiple of 4 for the whole buffer; pad if a single u16 was written.
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("webgl ibo"),
        contents: &bytes,
        usage: wgpu::BufferUsages::INDEX,
    })
}

/// Copy the render target into a readback buffer and return a closure that, after
/// the queue submit, maps + un-pads it into a tight top-left RGBA8 buffer.
fn readback<'e>(
    gpu: &'e Gpu,
    encoder: &mut wgpu::CommandEncoder,
    tex: &wgpu::Texture,
    width: u32,
    height: u32,
) -> impl FnOnce(&Gpu) -> Result<Vec<u8>, String> + 'e {
    let unpadded = width * 4;
    let padded = align_up(unpadded, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("webgl readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
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
    move |gpu: &Gpu| {
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        gpu.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| format!("map recv: {e}"))?
            .map_err(|e| format!("buffer map: {e}"))?;
        let mapped = buffer.slice(..).get_mapped_range();
        let mut rgba = Vec::with_capacity((unpadded * height) as usize);
        // Emit TOP-LEFT origin rows (wgpu's native order). The caller (the render-tier readPixels
        // shim) does the GL bottom-left flip when copying into the destination, so flipping here
        // too would double-flip.
        for row in 0..height {
            let start = (row * padded) as usize;
            rgba.extend_from_slice(&mapped[start..start + unpadded as usize]);
        }
        drop(mapped);
        buffer.unmap();
        Ok(rgba)
    }
}

// ---------------------------------------------------------------------------
// Small JSON accessors.
// ---------------------------------------------------------------------------

fn arg_i64(a: Option<&serde_json::Value>, i: usize) -> Option<i64> {
    a.and_then(|a| a.get(i)).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_bool().map(|b| b as i64))
    })
}

fn arg_str(a: Option<&serde_json::Value>, i: usize) -> Option<String> {
    a.and_then(|a| a.get(i))
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

fn f32x4(a: &serde_json::Value) -> Option<[f32; 4]> {
    let arr = a.as_array()?;
    let mut out = [0.0f32; 4];
    for (dst, v) in out.iter_mut().zip(arr.iter()) {
        *dst = v.as_f64()? as f32;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    // A minified single-line shader (declarations + main on one line, `;`-separated) must split so
    // each top-level declaration is isolated for the line-based rewrite — while the `{}` body's
    // inner semicolons stay intact. This was the bug that made real fingerprint shaders (all on one
    // line) fail to translate → clear-only render.
    #[test]
    fn split_top_level_statements_isolates_declarations_keeps_body() {
        let src =
            "attribute vec2 p; varying vec2 v; void main(){ v=p; gl_Position=vec4(p,0.0,1.0); }";
        let out = split_top_level_statements(src);
        let lines: Vec<&str> = out
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(
            lines[0], "attribute vec2 p;",
            "first decl isolated: {lines:?}"
        );
        assert_eq!(
            lines[1], "varying vec2 v;",
            "second decl isolated: {lines:?}"
        );
        // The main() body stays on one piece (its inner `;` at brace depth > 0 not split).
        assert!(
            lines[2].starts_with("void main(){")
                && lines[2].contains("gl_Position=vec4(p,0.0,1.0);"),
            "body kept verbatim: {lines:?}"
        );
    }

    fn b64_f32(v: &[f32]) -> String {
        let mut bytes = Vec::with_capacity(v.len() * 4);
        for f in v {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    /// A minimal batch: a full-screen (clip-space) triangle through a passthrough
    /// vertex shader + a `frag`-supplied fragment shader, drawArrays TRIANGLES.
    fn batch(frag: &str) -> String {
        // Clip-space triangle covering the whole viewport.
        let verts = b64_f32(&[-1.0, -1.0, 3.0, -1.0, -1.0, 3.0]);
        let vs = "attribute vec2 aPos;\nvarying vec2 vUv;\nvoid main(){ vUv = aPos*0.5+0.5; gl_Position = vec4(aPos,0.0,1.0); }";
        format!(
            r#"[
  {{"m":"clearColor","a":[0.0,0.0,0.0,1.0]}},
  {{"m":"clear","a":[16384]}},
  {{"m":"createShader","id":1,"a":[35633]}},
  {{"m":"shaderSource","a":[1,{vs}]}},
  {{"m":"compileShader","a":[1]}},
  {{"m":"createShader","id":2,"a":[35632]}},
  {{"m":"shaderSource","a":[2,{frag}]}},
  {{"m":"compileShader","a":[2]}},
  {{"m":"createProgram","id":3}},
  {{"m":"attachShader","a":[3,1]}},
  {{"m":"attachShader","a":[3,2]}},
  {{"m":"attribLocation","a":[3,0,"aPos"]}},
  {{"m":"linkProgram","a":[3]}},
  {{"m":"useProgram","a":[3]}},
  {{"m":"createBuffer","id":4}},
  {{"m":"bindBuffer","a":[34962,4]}},
  {{"m":"bufferData","a":[34962,{{"b64":"{verts}","t":"f32"}},35044]}},
  {{"m":"enableVertexAttribArray","a":[0]}},
  {{"m":"vertexAttribPointer","a":[0,2,5126,false,0,0]}},
  {{"m":"drawArrays","a":[4,0,3]}}
]"#,
            vs = serde_json::to_string(vs).unwrap(),
            frag = serde_json::to_string(frag).unwrap(),
            verts = verts,
        )
    }

    #[test]
    fn adapter_is_metal() {
        // Proves the executor targets the real Apple GPU, not software.
        assert_eq!(adapter_backend().unwrap(), wgpu::Backend::Metal);
    }

    #[test]
    fn readback_has_right_shape_and_draws_content() {
        let frag = "precision mediump float;\nvarying vec2 vUv;\nvoid main(){ gl_FragColor = vec4(vUv, 0.25, 1.0); }";
        let px = webgl_readback(64, 64, &batch(frag)).expect("gpu readback");
        assert_eq!(px.len(), 64 * 64 * 4, "tight RGBA8 buffer");
        assert!(px.iter().any(|&b| b != 0), "something drew (not all-zero)");
        // The gradient makes distinct pixels differ (content, not a flat fill).
        let first: [u8; 4] = px[0..4].try_into().unwrap();
        assert!(
            px.as_chunks::<4>().0.iter().any(|p| *p != first),
            "gradient produced varying pixels"
        );
    }

    #[test]
    fn distinct_fragment_shaders_give_distinct_pixels() {
        let a = webgl_readback(
            64,
            64,
            &batch("precision mediump float;\nvarying vec2 vUv;\nvoid main(){ gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0); }"),
        )
        .expect("a");
        let b = webgl_readback(
            64,
            64,
            &batch("precision mediump float;\nvarying vec2 vUv;\nvoid main(){ gl_FragColor = vec4(vUv, 1.0, 1.0); }"),
        )
        .expect("b");
        assert_ne!(a, b, "content-dependent pixels differ across shaders");
    }

    #[test]
    fn uniform_color_is_respected() {
        // A uniform-driven fragment colour exercises the UBO path end-to-end.
        let vs = "attribute vec2 aPos;\nvoid main(){ gl_Position = vec4(aPos,0.0,1.0); }";
        let fs =
            "precision mediump float;\nuniform vec4 uColor;\nvoid main(){ gl_FragColor = uColor; }";
        let verts = b64_f32(&[-1.0, -1.0, 3.0, -1.0, -1.0, 3.0]);
        let json = format!(
            r#"[
  {{"m":"clearColor","a":[0.0,0.0,0.0,1.0]}},
  {{"m":"createShader","id":1,"a":[35633]}},
  {{"m":"shaderSource","a":[1,{vs}]}},
  {{"m":"createShader","id":2,"a":[35632]}},
  {{"m":"shaderSource","a":[2,{fs}]}},
  {{"m":"createProgram","id":3}},
  {{"m":"attachShader","a":[3,1]}},
  {{"m":"attachShader","a":[3,2]}},
  {{"m":"attribLocation","a":[3,0,"aPos"]}},
  {{"m":"linkProgram","a":[3]}},
  {{"m":"useProgram","a":[3]}},
  {{"m":"uniform","a":[3,"uColor","4f",[0.0,1.0,0.0,1.0]]}},
  {{"m":"createBuffer","id":4}},
  {{"m":"bindBuffer","a":[34962,4]}},
  {{"m":"bufferData","a":[34962,{{"b64":"{verts}","t":"f32"}},35044]}},
  {{"m":"enableVertexAttribArray","a":[0]}},
  {{"m":"vertexAttribPointer","a":[0,2,5126,false,0,0]}},
  {{"m":"drawArrays","a":[4,0,3]}}
]"#,
            vs = serde_json::to_string(vs).unwrap(),
            fs = serde_json::to_string(fs).unwrap(),
            verts = verts,
        );
        let px = webgl_readback(32, 32, &json).expect("uniform readback");
        // Center pixel should be green (0,255,0,255).
        let mid = ((16 * 32 + 16) * 4) as usize;
        assert_eq!(px[mid], 0, "R");
        assert_eq!(px[mid + 1], 255, "G");
        assert_eq!(px[mid + 2], 0, "B");
    }

    #[test]
    fn garbage_returns_none() {
        assert!(webgl_readback(16, 16, "not json").is_none());
        assert!(webgl_readback(0, 16, "[]").is_none());
    }
}
