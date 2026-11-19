//! AVIF pixel decode for screenshots (miaskiewicz/turbo-surf#30), behind the off-by-default
//! `avif` feature.
//!
//! Sizing never needs this: turbo-dom-layout's shared probe reads an AVIF's `ispe` (and
//! [`turbo_dom_layout::image::avif_payload`] demuxes its AV1 streams) in pure Rust, so an
//! AVIF image box always lays out at its real size. Painting its pixels needs an AV1
//! decoder. The options assessed (2026-10): the `image` crate's AVIF decoding is
//! `avif-native` only — libdav1d, a native C library found through pkg-config; `ravif`
//! encodes only; `zenavif` / `rav1d-safe` are AGPL; `rav1d` (1.1, BSD-2-Clause, the ISRG
//! Prossimo Rust port of dav1d) is pure Rust with its assembly off
//! (`default-features = false`), at the cost of its C-ABI (`unsafe extern "C"`) entry
//! points and a large dependency tree. This module uses `rav1d` (no assembly, two frame
//! contexts — see [`av1::decode`] for why not one) and converts the decoded YUV planes to
//! RGBA itself; without the feature an AVIF paints as the placeholder, and a corrupt AVIF
//! (no pixels) paints nothing, as in Chrome (turbo-dom#107).

use turbo_dom_layout::image::Rgba8Image;

/// Decode an AVIF still image to straight RGBA8 (`None` without the `avif` feature, or
/// on any container / bitstream error).
#[cfg(not(feature = "avif"))]
pub fn decode(_bytes: &[u8]) -> Option<Rgba8Image> {
    None
}

/// Decode an AVIF still image to straight RGBA8, or `None` on any container or
/// bitstream error.
#[cfg(feature = "avif")]
pub fn decode(bytes: &[u8]) -> Option<Rgba8Image> {
    let payload = turbo_dom_layout::image::avif_payload(bytes)?;
    let color = av1::decode(&payload.color)?;
    // An alpha plane that fails to decode or does not match the colour frame is dropped
    // (the image paints opaque) rather than failing the whole image.
    let alpha = payload
        .alpha
        .as_deref()
        .and_then(av1::decode)
        .filter(|a| (a.width, a.height) == (color.width, color.height));
    let (matrix, full_range) = match payload.nclx {
        Some(n) => (u32::from(n.matrix), n.full_range),
        None => (color.matrix, color.full_range),
    };
    Some(yuv::to_rgba(&color, alpha.as_ref(), matrix, full_range))
}

/// One decoded AV1 frame: planes widened to `u16`, chroma subsampling, bit depth and the
/// sequence header's colour description.
#[cfg(feature = "avif")]
pub(crate) struct Frame {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) bits: u32,
    /// Chroma subsampling shifts `(x, y)`; `None` for a monochrome (4:0:0) frame.
    pub(crate) chroma: Option<(usize, usize)>,
    pub(crate) planes: [Vec<u16>; 3],
    pub(crate) matrix: u32,
    pub(crate) full_range: bool,
}

#[cfg(feature = "avif")]
mod av1 {
    //! The `rav1d` C-ABI calls: open a two-frame-context decoder, feed one temporal unit,
    //! take the picture, copy its planes out, release everything.
    use std::ptr::NonNull;

    use rav1d::include::dav1d::data::Dav1dData;
    use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
    use rav1d::include::dav1d::headers::{
        DAV1D_PIXEL_LAYOUT_I400, DAV1D_PIXEL_LAYOUT_I420, DAV1D_PIXEL_LAYOUT_I422,
    };
    use rav1d::include::dav1d::picture::Dav1dPicture;
    use rav1d::src::lib::{
        dav1d_close, dav1d_data_create, dav1d_data_unref, dav1d_default_settings,
        dav1d_get_picture, dav1d_open, dav1d_picture_unref, dav1d_send_data,
    };

    use super::Frame;

    const AGAIN: i32 = -libc::EAGAIN;

    /// Decode one AV1 still frame (an AVIF item's OBU stream).
    pub(crate) fn decode(stream: &[u8]) -> Option<Frame> {
        if stream.is_empty() {
            return None;
        }
        let mut settings = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
        // SAFETY: `settings` is valid to write the default settings into.
        unsafe { dav1d_default_settings(NonNull::new(settings.as_mut_ptr())?) };
        // SAFETY: `dav1d_default_settings` initialized every field.
        let mut settings = unsafe { settings.assume_init() };
        // Two frame contexts (frame delay 2, two worker threads): with exactly one, rav1d
        // 1.1.0 tears a frame down twice on a decode error and panics inside its
        // `extern "C"` API, which aborts the process (miaskiewicz/turbo-dom#94, upstream
        // memorysafety/rav1d#1496, fixed after 1.1.0; 1.1.1 is unpublished). With two, a
        // decode error comes back as an error result. AV1 decoding is bit-exact, so the
        // pixels do not depend on the thread count.
        settings.n_threads = 2;
        settings.max_frame_delay = 2;
        // Bound the frame like every other format's decode (the `image` crate's 512 MiB
        // `max_alloc`, as RGBA): unbounded, a 1.8 KB 12000 x 12000 still peaks at ~2.8 GB.
        // An over-budget frame is a decode error: laid out at its size, painting nothing.
        settings.frame_size_limit = MAX_FRAME_PIXELS;
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: `ctx` is valid to write to and `settings` to read from.
        let opened =
            unsafe { dav1d_open(NonNull::new(&raw mut ctx), NonNull::new(&raw mut settings)) };
        // Opening fails only when rav1d cannot allocate the context (these settings pass its
        // validation); nothing decodes then, and closing the unopened (`None`) context is a
        // no-op.
        let frame = (opened.0 == 0).then(|| decode_with(ctx, stream)).flatten();
        // SAFETY: `ctx` came from `dav1d_open` (or stayed `None`) and is closed exactly once.
        unsafe { dav1d_close(NonNull::new(&raw mut ctx)) };
        frame
    }

    /// The largest frame (pixels) decoded: 512 MiB of RGBA8.
    const MAX_FRAME_PIXELS: u32 = 512 * 1024 * 1024 / 4;

    fn decode_with(ctx: Option<Dav1dContext>, stream: &[u8]) -> Option<Frame> {
        let mut data = Dav1dData::default();
        // SAFETY: `data` is valid to write the new buffer into. Null only when rav1d cannot
        // allocate it (a slice's length is always within its `usize::MAX / 2` bound).
        let buffer =
            NonNull::new(unsafe { dav1d_data_create(NonNull::new(&raw mut data), stream.len()) })?;
        // SAFETY: `dav1d_data_create` returned a writable buffer of `stream.len()` bytes.
        unsafe { std::ptr::copy_nonoverlapping(stream.as_ptr(), buffer.as_ptr(), stream.len()) };
        let mut picture = Dav1dPicture::default();
        let mut got = false;
        for _ in 0..16 {
            if data.sz > 0 {
                // SAFETY: `ctx` is open; `data` is a valid, owned data buffer.
                let sent = unsafe { dav1d_send_data(ctx, NonNull::new(&raw mut data)) };
                if sent.0 != 0 && sent.0 != AGAIN {
                    break;
                }
            }
            // SAFETY: `ctx` is open; `picture` is valid to write to.
            let res = unsafe { dav1d_get_picture(ctx, NonNull::new(&raw mut picture)) };
            // A picture (0) or a decode error ends the loop; `EAGAIN` asks for more input.
            if res.0 != AGAIN {
                got = res.0 == 0;
                break;
            }
        }
        if data.sz > 0 {
            // SAFETY: `data` still owns its (unconsumed) buffer.
            unsafe { dav1d_data_unref(NonNull::new(&raw mut data)) };
        }
        if !got {
            return None;
        }
        let frame = copy_frame(&picture);
        // SAFETY: `picture` was filled by `dav1d_get_picture` and is released once.
        unsafe { dav1d_picture_unref(NonNull::new(&raw mut picture)) };
        frame
    }

    /// Copy one plane out of a decoded picture as `u16` samples.
    fn copy_plane(
        picture: &Dav1dPicture,
        i: usize,
        w: usize,
        h: usize,
        wide: bool,
    ) -> Option<Vec<u16>> {
        let base = picture.data[i]?.as_ptr().cast::<u8>();
        let stride = picture.stride[usize::from(i > 0)];
        let row_bytes = w * if wide { 2 } else { 1 };
        let mut out = Vec::with_capacity(w * h);
        for y in 0..h {
            // SAFETY: rows `0..h` of `row_bytes` bytes lie inside the plane at `stride`.
            let row =
                unsafe { std::slice::from_raw_parts(base.offset(stride * y as isize), row_bytes) };
            if wide {
                out.extend(
                    row.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|&b| u16::from_ne_bytes(b)),
                );
            } else {
                out.extend(row.iter().map(|&b| u16::from(b)));
            }
        }
        Some(out)
    }

    fn chroma_subsampling(layout: u32) -> Option<(usize, usize)> {
        match layout {
            DAV1D_PIXEL_LAYOUT_I400 => None,
            DAV1D_PIXEL_LAYOUT_I420 => Some((1, 1)),
            DAV1D_PIXEL_LAYOUT_I422 => Some((1, 0)),
            _ => Some((0, 0)),
        }
    }

    fn copy_chroma_planes(
        picture: &Dav1dPicture,
        sub: Option<(usize, usize)>,
        w: usize,
        h: usize,
        wide: bool,
    ) -> Option<(Vec<u16>, Vec<u16>)> {
        let (sx, sy) = sub?;
        let (cw, ch) = ((w + sx) >> sx, (h + sy) >> sy);
        Some((
            copy_plane(picture, 1, cw, ch, wide)?,
            copy_plane(picture, 2, cw, ch, wide)?,
        ))
    }

    /// Copy a picture's planes out as `u16` samples.
    fn copy_frame(picture: &Dav1dPicture) -> Option<Frame> {
        let width = usize::try_from(picture.p.w).ok()?;
        let height = usize::try_from(picture.p.h).ok()?;
        let bits = u32::try_from(picture.p.bpc).ok()?;
        let chroma = chroma_subsampling(picture.p.layout);
        // SAFETY: a decoded picture carries its sequence header.
        let seq = unsafe { picture.seq_hdr?.as_ref() };
        let wide = bits > 8;
        let luma = copy_plane(picture, 0, width, height, wide)?;
        let (u, v) = copy_chroma_planes(picture, chroma, width, height, wide).unwrap_or_default();
        Some(Frame {
            width,
            height,
            bits,
            chroma,
            planes: [luma, u, v],
            matrix: seq.mtrx,
            full_range: seq.color_range != 0,
        })
    }
}

#[cfg(feature = "avif")]
pub(crate) mod yuv {
    //! YUV (ITU-T H.273 matrix coefficients) to straight RGBA8.
    use turbo_dom_layout::image::Rgba8Image;

    use super::Frame;

    /// The `(Kr, Kb)` luma weights of a matrix-coefficients code point (BT.709 = 1,
    /// BT.2020 = 9/10, FCC = 4, SMPTE 240M = 7; BT.601 for 5/6 and, as libavif and
    /// Chrome do, for unspecified 2 and anything else). `None` = identity (GBR, 0).
    pub(crate) fn weights(matrix: u32) -> Option<(f32, f32)> {
        match matrix {
            0 => None,
            1 => Some((0.2126, 0.0722)),
            4 => Some((0.30, 0.11)),
            7 => Some((0.212, 0.087)),
            9 | 10 => Some((0.2627, 0.0593)),
            _ => Some((0.299, 0.114)),
        }
    }

    /// Convert a decoded colour frame (plus an optional alpha frame of the same size) to
    /// straight RGBA8 (nearest-neighbour chroma upsampling).
    pub(crate) fn to_rgba(
        color: &Frame,
        alpha: Option<&Frame>,
        matrix: u32,
        full: bool,
    ) -> Rgba8Image {
        let max = ((1u32 << color.bits) - 1) as f32;
        // Limited ("video") range is the 8-bit range shifted to the depth (H.273).
        let shift = (1u32 << (color.bits - 8)) as f32;
        let (luma_off, luma_range, chroma_range) = if full {
            (0.0, max, max)
        } else {
            (16.0 * shift, 219.0 * shift, 224.0 * shift)
        };
        let mid = (1u32 << (color.bits - 1)) as f32;
        let weights = weights(matrix);
        let clamp = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
        let alpha_max = alpha.map_or(1.0, |a| ((1u32 << a.bits) - 1) as f32);
        let mut rgba = Vec::with_capacity(color.width * color.height * 4);
        for y in 0..color.height {
            for x in 0..color.width {
                let luma = f32::from(color.planes[0][y * color.width + x]);
                let (cb, cr) = color.chroma.map_or((mid, mid), |(sx, sy)| {
                    let cw = (color.width + sx) >> sx;
                    let at = (y >> sy) * cw + (x >> sx);
                    (
                        f32::from(color.planes[1][at]),
                        f32::from(color.planes[2][at]),
                    )
                });
                let yn = (luma - luma_off) / luma_range;
                let (u, v) = ((cb - mid) / chroma_range, (cr - mid) / chroma_range);
                let (r, g, b) = match weights {
                    // Identity (H.273 equations 41-43): every plane is quantized like
                    // luma, as libavif converts it.
                    None if color.chroma.is_some() => (
                        (cr - luma_off) / luma_range,
                        yn,
                        (cb - luma_off) / luma_range,
                    ),
                    None => (yn, yn, yn),
                    Some(_) if color.chroma.is_none() => (yn, yn, yn),
                    Some((kr, kb)) => {
                        let kg = 1.0 - kr - kb;
                        let r = yn + 2.0 * (1.0 - kr) * v;
                        let b = yn + 2.0 * (1.0 - kb) * u;
                        (r, (yn - kr * r - kb * b) / kg, b)
                    }
                };
                let a = alpha.map_or(255, |a| {
                    let s = f32::from(a.planes[0][y * a.width + x]);
                    clamp(s / alpha_max)
                });
                rgba.extend_from_slice(&[clamp(r), clamp(g), clamp(b), a]);
            }
        }
        Rgba8Image {
            width: color.width as u32,
            height: color.height as u32,
            rgba,
        }
    }
}

#[cfg(all(test, feature = "avif"))]
mod progress {
    //! Test-only last-mutation storage; recording must not change decoder timing with I/O.
    use std::io;
    use std::process::Command;

    #[cfg(unix)]
    const ENV: &str = "TURBO_DOM_AVIF_FUZZ_PROGRESS";

    pub(super) fn case_id(fixture: usize, plane: usize, variant: usize) -> u64 {
        (u64::from(u16::try_from(fixture).unwrap()) << 48)
            | (u64::from(u16::try_from(plane).unwrap()) << 32)
            | u64::from(u32::try_from(variant).unwrap().checked_add(1).unwrap())
    }

    pub(super) struct Breadcrumb {
        #[cfg(unix)]
        pointer: std::ptr::NonNull<std::sync::atomic::AtomicU64>,
        #[cfg(unix)]
        path: std::path::PathBuf,
        #[cfg(unix)]
        owner: bool,
    }

    #[cfg(unix)]
    impl Breadcrumb {
        pub(super) fn new() -> io::Result<Self> {
            use std::os::unix::fs::OpenOptionsExt;
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "turbo-dom-avif-{}-{nonce}.progress",
                std::process::id()
            ));
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)?;
            file.set_len(std::mem::size_of::<std::sync::atomic::AtomicU64>() as u64)?;
            Self::map(&file, path, true)
        }

        pub(super) fn child() -> io::Result<Self> {
            Self::child_path(std::env::var_os(ENV).map(std::path::PathBuf::from))
        }

        fn child_path(path: Option<std::path::PathBuf>) -> io::Result<Self> {
            match path {
                Some(path) => Self::open(path),
                None => Self::new(),
            }
        }

        fn open(path: std::path::PathBuf) -> io::Result<Self> {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)?;
            Self::map(&file, path, false)
        }

        fn map(file: &std::fs::File, path: std::path::PathBuf, owner: bool) -> io::Result<Self> {
            use std::os::fd::AsRawFd;
            if file.metadata()?.len() < std::mem::size_of::<std::sync::atomic::AtomicU64>() as u64 {
                return Err(io::Error::other("short AVIF progress file"));
            }
            // SAFETY: the file owns one zero-initialized atomic-sized slot. mmap
            // returns page-aligned storage, retained until this mapping is dropped.
            let memory = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    std::mem::size_of::<std::sync::atomic::AtomicU64>(),
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    file.as_raw_fd(),
                    0,
                )
            };
            if memory == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            let pointer = mapping_pointer(memory, unmap)?;
            Ok(Self {
                pointer,
                path,
                owner,
            })
        }

        pub(super) fn configure(&self, command: &mut Command) {
            command.env(ENV, &self.path);
        }

        pub(super) fn record(&self, fixture: usize, plane: usize, variant: usize) {
            // SAFETY: this mapping contains one aligned AtomicU64. The child is its
            // sole writer; the parent reads only after that process has exited.
            unsafe { self.pointer.as_ref() }.store(
                case_id(fixture, plane, variant),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        pub(super) fn read(&self, _stderr: &str) -> u64 {
            // SAFETY: the mapping is still owned by self and the child has exited.
            unsafe { self.pointer.as_ref() }.load(std::sync::atomic::Ordering::Relaxed)
        }
    }

    #[cfg(unix)]
    fn mapping_pointer(
        memory: *mut libc::c_void,
        release: impl FnOnce(*mut libc::c_void),
    ) -> io::Result<std::ptr::NonNull<std::sync::atomic::AtomicU64>> {
        let Some(pointer) = std::ptr::NonNull::new(memory.cast()) else {
            // A successful address-zero mapping must be released before rejection;
            // never form an AtomicU64 reference at that address.
            release(memory);
            return Err(io::Error::other("null AVIF progress mapping"));
        };
        Ok(pointer)
    }

    #[cfg(unix)]
    fn unmap(memory: *mut libc::c_void) {
        // SAFETY: callers supply exactly the live atomic-sized mapping from mmap.
        unsafe { libc::munmap(memory, std::mem::size_of::<std::sync::atomic::AtomicU64>()) };
    }

    #[cfg(unix)]
    impl Drop for Breadcrumb {
        fn drop(&mut self) {
            unmap(self.pointer.as_ptr().cast());
            if self.owner {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    #[cfg(not(unix))]
    impl Breadcrumb {
        pub(super) fn new() -> io::Result<Self> {
            Ok(Self {})
        }
        pub(super) fn child() -> io::Result<Self> {
            Self::new()
        }
        pub(super) fn configure(&self, _command: &mut Command) {}
        pub(super) fn record(&self, fixture: usize, plane: usize, variant: usize) {
            eprintln!("FUZZ-ID {}", case_id(fixture, plane, variant));
        }
        pub(super) fn read(&self, stderr: &str) -> u64 {
            stderr
                .lines()
                .rev()
                .find_map(|line| line.strip_prefix("FUZZ-ID ")?.parse().ok())
                .unwrap_or(0)
        }
    }

    #[cfg(unix)]
    #[test]
    fn independently_mapped_child_progress_survives_until_parent_reads_it() {
        let parent = Breadcrumb::new().unwrap();
        let path = parent.path.clone();
        assert_eq!(parent.read(""), 0);
        let child = Breadcrumb::open(path.clone()).unwrap();
        child.record(17, 2, 65537);
        drop(child);
        assert_eq!(parent.read(""), case_id(17, 2, 65537));
        drop(parent);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_nonzero_child_exit_retains_its_last_atomic_case_without_stderr() {
        const PROBE: &str = "TURBO_DOM_AVIF_PROGRESS_PROBE";
        if std::env::var_os(PROBE).is_some() {
            Breadcrumb::child().unwrap().record(7, 1, 1234);
            std::process::exit(23);
        }
        let parent = Breadcrumb::new().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        parent.configure(&mut command);
        let output = command
            .env(PROBE, "1")
            .args([
                "--exact",
                "avif::progress::a_nonzero_child_exit_retains_its_last_atomic_case_without_stderr",
                "--nocapture",
                "--test-threads=1",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(23));
        assert!(output.stderr.is_empty());
        assert_eq!(parent.read(""), case_id(7, 1, 1234));
    }

    /// Breadcrumbs made in the same clock tick (macOS clocks tick in microseconds, and the
    /// tests run in parallel) get distinct files instead of failing `create_new`.
    #[cfg(unix)]
    #[test]
    fn breadcrumbs_created_at_once_never_collide() {
        let start = std::sync::Barrier::new(16);
        let paths: Vec<_> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..16)
                .map(|_| {
                    scope.spawn(|| {
                        start.wait();
                        (0..64)
                            .map(|_| Breadcrumb::new().map(|c| c.path.clone()))
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            threads.into_iter().flat_map(|t| t.join().unwrap()).collect()
        });
        let created: Vec<_> = paths.into_iter().collect::<io::Result<_>>().unwrap();
        assert_eq!(created.len(), 16 * 64);
    }

    #[cfg(unix)]
    #[test]
    fn truncated_progress_storage_is_rejected_before_mapping() {
        let parent = Breadcrumb::new().unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(&parent.path)
            .unwrap()
            .set_len(0)
            .unwrap();
        assert!(Breadcrumb::open(parent.path.clone()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn absent_parent_progress_path_creates_owned_storage_without_changing_environment() {
        let child = Breadcrumb::child_path(None).unwrap();
        let path = child.path.clone();
        assert!(child.owner);
        child.record(3, 4, 5);
        assert_eq!(child.read(""), case_id(3, 4, 5));
        assert!(path.exists());
        drop(child);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn shared_writable_mapping_rejects_a_read_only_file_descriptor() {
        let parent = Breadcrumb::new().unwrap();
        let read_only = std::fs::File::open(&parent.path).unwrap();
        let error = Breadcrumb::map(&read_only, parent.path.clone(), false)
            .err()
            .expect("MAP_SHARED with PROT_WRITE requires a writable descriptor");
        assert_eq!(error.raw_os_error(), Some(libc::EACCES));
        assert_eq!(parent.read(""), 0);
        assert!(parent.path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn address_zero_mapping_is_released_once_before_rejection() {
        let releases = std::cell::Cell::new(0);
        let error = mapping_pointer(std::ptr::null_mut(), |memory| {
            assert!(memory.is_null());
            releases.set(releases.get() + 1);
        })
        .unwrap_err();
        assert_eq!(releases.get(), 1);
        assert_eq!(error.to_string(), "null AVIF progress mapping");
    }
}

#[cfg(all(test, feature = "avif"))]
mod tests {
    use super::progress::Breadcrumb;
    use super::yuv::{to_rgba, weights};
    use super::{av1, Frame};

    /// The colour stream of a turbo-dom-layout format fixture (`q.avif`: 64x40, 8-bit).
    fn stream() -> Vec<u8> {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../turbo-dom-layout/tests/fixtures/images/formats/q.avif"
        );
        let avif = std::fs::read(path).unwrap();
        turbo_dom_layout::image::avif_payload(&avif).unwrap().color
    }

    /// Every AVIF fixture of turbo-dom-layout and this crate.
    fn fixtures() -> Vec<(String, Vec<u8>)> {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut out = Vec::new();
        for dir in [
            format!("{root}/../turbo-dom-layout/tests/fixtures/images/formats"),
            format!("{root}/tests/fixtures/images/avif"),
        ] {
            let mut paths: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.extension().is_some_and(|e| e == "avif"))
                .collect();
            paths.sort();
            for p in paths {
                out.push((p.display().to_string(), std::fs::read(&p).unwrap()));
            }
        }
        out
    }

    /// The fuzz body [`corrupt_avifs_are_broken_images_never_a_process_abort`] runs in a
    /// child process (an abort inside the decoder takes the whole process down, which a
    /// test in this process could not observe): every AVIF fixture truncated at each
    /// sixteenth and with single bytes flipped across the file, decoded end to end, and its
    /// AV1 streams corrupted the same way and fed to rav1d directly.
    fn byte_flip_variants(base: &[u8], step: usize) -> Vec<Vec<u8>> {
        let mut variants: Vec<Vec<u8>> = (1..16)
            .map(|i| base[..base.len() * i / 16].to_vec())
            .collect();
        for at in (0..base.len()).step_by(step) {
            for mask in [0x01, 0x80, 0xff] {
                let mut v = base.to_vec();
                v[at] ^= mask;
                variants.push(v);
            }
        }
        variants
    }

    fn fuzz_container(
        label: &str,
        avif: &[u8],
        progress: &Breadcrumb,
        fixture: usize,
    ) -> (usize, usize) {
        let (mut decoded, mut rejected) = (0, 0);
        for (index, v) in byte_flip_variants(avif, 3).into_iter().enumerate() {
            progress.record(fixture, 0, index);
            match super::decode(&v) {
                Some(img) => {
                    assert_eq!(
                        img.rgba.len(),
                        img.width as usize * img.height as usize * 4,
                        "{label}"
                    );
                    decoded += 1;
                }
                None => rejected += 1,
            }
        }
        (decoded, rejected)
    }

    fn fuzz_stream(
        label: &str,
        stream: Vec<u8>,
        progress: &Breadcrumb,
        fixture: usize,
        plane: usize,
    ) -> (usize, usize) {
        let (mut decoded, mut rejected) = (0, 0);
        let n = stream.len();
        let mut variants = byte_flip_variants(&stream, 1);
        let mut ones = stream;
        ones[n / 2..].fill(0xff);
        variants.push(ones);
        for (index, v) in variants.into_iter().enumerate() {
            progress.record(fixture, plane + 1, index);
            match av1::decode(&v) {
                Some(frame) => {
                    assert_eq!(frame.planes[0].len(), frame.width * frame.height, "{label}");
                    decoded += 1;
                }
                None => rejected += 1,
            }
        }
        (decoded, rejected)
    }

    #[test]
    #[ignore = "run in a child process by corrupt_avifs_are_broken_images_never_a_process_abort"]
    fn corrupt_avif_fuzz_child() {
        let (mut decoded, mut rejected) = (0, 0);
        let progress = Breadcrumb::child().unwrap();
        for (fixture, (label, avif)) in fixtures().into_iter().enumerate() {
            let (d, r) = fuzz_container(&label, &avif, &progress, fixture);
            decoded += d;
            rejected += r;
            // Most container-level variants fail in the demuxer before rav1d sees them, so
            // the AV1 streams themselves are corrupted too: truncated at each sixteenth,
            // their second half overwritten with 0xFF (the issue's reproduction on q.avif's
            // colour stream) and every byte flipped with 0x01 / 0x80 / 0xff.
            let Some(payload) = turbo_dom_layout::image::avif_payload(&avif) else {
                continue;
            };
            for (index, stream) in std::iter::once(payload.color)
                .chain(payload.alpha)
                .enumerate()
            {
                let (d, r) = fuzz_stream(&label, stream, &progress, fixture, index);
                decoded += d;
                rejected += r;
            }
        }
        println!("FUZZ-DONE decoded={decoded} rejected={rejected}");
    }

    /// A corrupt AVIF decodes to `None` (the box paints as a broken image) — it never
    /// aborts the process (miaskiewicz/turbo-dom#94: rav1d 1.1.0 panicked inside its
    /// `extern "C"` API on a decode error with one frame context, aborting).
    #[test]
    fn corrupt_avifs_are_broken_images_never_a_process_abort() {
        let progress = Breadcrumb::new().unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        progress.configure(&mut command);
        let out = command
            .env("RUST_BACKTRACE", "1")
            .args([
                "--exact",
                "avif::tests::corrupt_avif_fuzz_child",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        if !out.status.success() || !stdout.contains("FUZZ-DONE") {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/avif-fuzz.stderr.log");
            let saved = std::fs::create_dir_all(path.parent().unwrap())
                .and_then(|()| std::fs::write(&path, &out.stderr));
            panic!(
                "child {:?}\n{stdout}\nfull stderr: {} ({saved:?})\n{}\n{}",
                out.status,
                path.display(),
                describe_case(progress.read(&stderr)),
                fuzz_failure_details(&stderr)
            );
        }
    }

    fn describe_case(id: u64) -> String {
        if id == 0 {
            return "last mutation unavailable".into();
        }
        let fixture = usize::try_from(id >> 48).unwrap();
        let plane = (id >> 32) & 0xffff;
        let variant = (id & 0xffff_ffff) - 1;
        let inputs = fixtures();
        let label = &inputs[fixture].0;
        let kind = if plane == 0 {
            "container".to_string()
        } else {
            format!("stream plane={}", plane - 1)
        };
        format!("FUZZ-CASE {kind} {label} variant={variant}")
    }

    #[test]
    fn packed_progress_identifies_container_color_and_alpha_mutations() {
        use super::progress::case_id;
        assert_eq!(describe_case(0), "last mutation unavailable");
        for (plane, expected) in [
            (0, "container"),
            (1, "stream plane=0"),
            (2, "stream plane=1"),
        ] {
            let description = describe_case(case_id(0, plane, 127));
            assert!(description.contains(expected));
            assert!(description.contains("q-alpha.avif variant=127"));
        }
    }

    fn fuzz_failure_details(stderr: &str) -> String {
        let panic = stderr
            .lines()
            .find(|line| line.contains("panicked at") || line.contains("memory allocation"))
            .unwrap_or("no Rust panic/allocation message");
        let tail: Vec<_> = stderr.lines().rev().take(120).collect();
        format!(
            "first failure: {panic}\nstderr tail:\n{}",
            tail.into_iter().rev().collect::<Vec<_>>().join("\n")
        )
    }

    #[test]
    fn fuzz_failure_diagnostics_preserve_the_case_and_late_panic() {
        let noise = "irrelevant decoder output\n".repeat(500);
        let stderr = format!("{noise}FUZZ-CASE stream fixture plane=0 variant=37 bytes=50\nthread worker panicked at decoder.rs:4997\nstack end");
        let details = fuzz_failure_details(&stderr);
        assert!(details.contains("variant=37"));
        assert!(details.contains("panicked at decoder.rs:4997"));
        assert!(details.ends_with("stack end"));
        assert!(details.len() < stderr.len());
    }

    /// The colour (and alpha) streams of an AVIF fixture of this crate or turbo-dom-layout.
    fn payload(path: &str) -> turbo_dom_layout::image::AvifPayload {
        let avif = std::fs::read(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        turbo_dom_layout::image::avif_payload(&avif).unwrap()
    }

    #[test]
    fn an_oversize_frame_is_refused_within_the_image_crate_budget() {
        // A 1.8 KB, 12000 x 12000 solid AVIF: its RGBA (576 MB) is over the 512 MiB budget
        // every other format decodes within; unbounded, rav1d alone peaked at ~2.8 GB.
        let avif = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/images/avif/oversize-12000.avif"
        ))
        .unwrap();
        assert!(super::decode(&avif).is_none());
        let color = payload("tests/fixtures/images/avif/oversize-12000.avif").color;
        assert!(
            av1::decode(&color).is_none(),
            "refused before rav1d allocates"
        );
    }

    #[test]
    fn frames_carry_each_layouts_planes_at_their_depth() {
        let raster = "tests/fixtures/images/avif";
        let layout = "../turbo-dom-layout/tests/fixtures/images/formats";
        // (fixture, bits, chroma shifts, chroma plane length, sample (Y, Cb, Cr) at (16, 10)).
        for (file, bits, chroma, uv_len, at) in [
            (
                format!("{raster}/q-10bit-limited.avif"),
                10,
                Some((0, 0)),
                64 * 40,
                (250, 409, 960),
            ),
            (
                format!("{raster}/q-12bit-full.avif"),
                12,
                Some((1, 1)),
                32 * 20,
                (1224, 1357, 4095),
            ),
            (
                format!("{raster}/q-422-bt601.avif"),
                8,
                Some((1, 0)),
                32 * 40,
                (0, 0, 0),
            ),
            (
                format!("{raster}/q-lossless.avif"),
                8,
                Some((0, 0)),
                64 * 40,
                (0, 0, 255),
            ),
        ] {
            let f = av1::decode(&payload(&file).color).expect(&file);
            assert_eq!(
                (f.width, f.height, f.bits, f.chroma),
                (64, 40, bits, chroma),
                "{file}"
            );
            assert_eq!(
                (f.planes[0].len(), f.planes[1].len(), f.planes[2].len()),
                (64 * 40, uv_len, uv_len),
                "{file}"
            );
            if at != (0, 0, 0) {
                let (sx, sy) = chroma.unwrap();
                let c = (10 >> sy) * ((64 + sx) >> sx) + (16 >> sx);
                assert_eq!(
                    (f.planes[0][10 * 64 + 16], f.planes[1][c], f.planes[2][c]),
                    at,
                    "{file}"
                );
            }
        }
        // An alpha auxiliary stream is monochrome: no chroma planes.
        let alpha =
            av1::decode(&payload(&format!("{layout}/q-alpha.avif")).alpha.unwrap()).unwrap();
        assert_eq!(
            (alpha.chroma, alpha.planes[1].len(), alpha.planes[2].len()),
            (None, 0, 0)
        );
        assert_eq!(alpha.planes[0].len(), 64 * 40);
    }

    #[test]
    fn a_colour_stream_decodes_to_its_frame() {
        let frame = av1::decode(&stream()).expect("decodes");
        assert_eq!((frame.width, frame.height, frame.bits), (64, 40, 8));
        assert_eq!(frame.planes[0].len(), 64 * 40);
    }

    #[test]
    fn malformed_and_incomplete_streams_decode_to_nothing() {
        let good = stream();
        assert!(av1::decode(&[]).is_none(), "empty");
        // OBUs rav1d cannot parse: all-ones bytes (a header whose size overruns the data),
        // zeros (an unknown OBU type), the stream cut in half, and its middle third
        // scrambled.
        let n = good.len();
        let mut scrambled = good.clone();
        scrambled[n / 3..n / 2].iter_mut().for_each(|b| *b ^= 0x5a);
        for (label, bad) in [
            ("ones", vec![0xff; 32]),
            ("zeros", vec![0; 32]),
            ("half", good[..n / 2].to_vec()),
            ("scrambled", scrambled),
        ] {
            assert!(av1::decode(&bad).is_none(), "{label}");
        }
        // Only the temporal delimiter and sequence header (no frame): no picture comes.
        assert!(av1::decode(&good[..n / 8]).is_none(), "header only");
    }

    /// A stream that makes rav1d panic is a broken image, not a process abort
    /// (miaskiewicz/turbo-dom#535). This ITU-T T.35 metadata OBU underflows rav1d 1.1.0's
    /// payload size (memorysafety/rav1d#1476): the panic used to cross its `extern "C"`
    /// API and abort the process.
    #[test]
    fn a_stream_that_panics_rav1d_is_a_broken_image_not_an_abort() {
        let itut_t35_underflow = [47, 47, 5, 4, 128, 0, 0, 0];
        assert!(av1::decode(&itut_t35_underflow).is_none());
        // The decoder still works afterwards.
        assert!(av1::decode(&stream()).is_some());
    }

    /// A one-row frame of `bits` depth with the given planes.
    fn frame(bits: u32, chroma: Option<(usize, usize)>, planes: [Vec<u16>; 3]) -> Frame {
        Frame {
            width: planes[0].len(),
            height: 1,
            bits,
            chroma,
            planes,
            matrix: 2,
            full_range: true,
        }
    }

    /// `rgb` (0..1) through the H.273 forward transform of `(kr, kb)` to full-range
    /// 8-bit `Y'CbCr`, then back through [`to_rgba`] for `matrix`.
    fn round_trip(matrix: u32, (kr, kb): (f32, f32), rgb: [f32; 3]) -> [u8; 4] {
        let [r, g, b] = rgb;
        let y = kr * r + (1.0 - kr - kb) * g + kb * b;
        let cb = (b - y) / (2.0 * (1.0 - kb));
        let cr = (r - y) / (2.0 * (1.0 - kr));
        let q = |v: f32| (v * 255.0).round() as u16;
        let f = frame(
            8,
            Some((0, 0)),
            [
                vec![q(y)],
                vec![q(cb + 128.0 / 255.0)],
                vec![q(cr + 128.0 / 255.0)],
            ],
        );
        let out = to_rgba(&f, None, matrix, true).rgba;
        [out[0], out[1], out[2], out[3]]
    }

    #[test]
    fn each_matrix_inverts_its_own_forward_transform() {
        // BT.709, FCC, SMPTE 240M, BT.2020 (NCL and CL weights), BT.601 (5, 6) and the
        // BT.601 default for unspecified (2) and any other code point.
        let cases = [
            (1, (0.2126, 0.0722)),
            (4, (0.30, 0.11)),
            (7, (0.212, 0.087)),
            (9, (0.2627, 0.0593)),
            (10, (0.2627, 0.0593)),
            (5, (0.299, 0.114)),
            (6, (0.299, 0.114)),
            (2, (0.299, 0.114)),
            (99, (0.299, 0.114)),
        ];
        for (matrix, k) in cases {
            assert_eq!(weights(matrix), Some(k), "{matrix}");
            for rgb in [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 1.0, 0.0],
                [0.5, 0.5, 0.5],
            ] {
                let got = round_trip(matrix, k, rgb);
                let want = rgb.map(|v| (v * 255.0).round() as u8);
                assert!(
                    got[..3].iter().zip(&want).all(|(g, w)| g.abs_diff(*w) <= 2) && got[3] == 255,
                    "matrix {matrix} {rgb:?}: {got:?}"
                );
            }
        }
        assert_eq!(weights(0), None, "identity");
    }

    #[test]
    fn monochrome_frames_are_grey_under_any_matrix() {
        // A monochrome frame under identity or any weighted matrix is grey from luma alone.
        let mono = frame(8, None, [vec![0, 128, 255], vec![], vec![]]);
        let grey = [0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255];
        assert_eq!(to_rgba(&mono, None, 0, true).rgba, grey);
        assert_eq!(to_rgba(&mono, None, 1, true).rgba, grey);
    }

    /// Identity (GBR, matrix 0; H.273 equations 41-43): G = Y, B = Cb, R = Cr, every plane
    /// quantized like luma — as libavif (Chrome's AVIF decoder) converts it.
    #[test]
    fn identity_gbr_maps_every_sample_exactly() {
        let all: Vec<u16> = (0..=255).collect();
        let rev: Vec<u16> = (0..=255).rev().collect();
        let mid: Vec<u16> = (0..=255).map(|v| (v + 77) % 256).collect();
        let full = frame(8, Some((0, 0)), [all.clone(), rev.clone(), mid.clone()]);
        let px = to_rgba(&full, None, 0, true).rgba;
        for i in 0..256 {
            let want = [mid[i] as u8, all[i] as u8, rev[i] as u8, 255];
            assert_eq!(px[i * 4..i * 4 + 4], want, "full range sample {i}");
        }
        // Limited range: 16..=235 on every plane is 0..=255.
        let limited = frame(
            8,
            Some((0, 0)),
            [vec![126, 16, 235], vec![16, 235, 126], vec![235, 126, 16]],
        );
        assert_eq!(
            to_rgba(&limited, None, 0, false).rgba,
            [255, 128, 0, 255, 128, 0, 255, 255, 0, 255, 128, 255]
        );
        // 10-bit full range: 1023 is 255, 0 is 0.
        let deep = frame(10, Some((0, 0)), [vec![1023], vec![0], vec![1023]]);
        assert_eq!(to_rgba(&deep, None, 0, true).rgba, [255, 255, 0, 255]);
    }

    /// Limited ("video") range at any depth is the 8-bit range shifted (H.273): luma
    /// 16..=235 and chroma 16..=240 times 2^(bits - 8) — 64..=940 / 64..=960 at 10 bits.
    #[test]
    fn deep_limited_range_is_the_eight_bit_range_shifted() {
        let luma = frame(10, None, [vec![64, 940, 502, 0, 1023], vec![], vec![]]);
        assert_eq!(
            to_rgba(&luma, None, 1, false).rgba,
            [
                0, 0, 0, 255, 255, 255, 255, 255, 128, 128, 128, 255, 0, 0, 0, 255, 255, 255, 255,
                255
            ]
        );
        // 12 bits: 256..=3760; a BT.601 red from limited 12-bit Y'CbCr.
        let deep = frame(12, None, [vec![256, 3760], vec![], vec![]]);
        assert_eq!(
            to_rgba(&deep, None, 6, false).rgba,
            [0, 0, 0, 255, 255, 255, 255, 255]
        );
        let (y, cb, cr) = (
            16.0 + 219.0 * 0.299,
            128.0 - 112.0 * 0.299 / 0.886,
            128.0 + 112.0,
        );
        let q = |v: f64| (v * 16.0).round() as u16;
        let red = frame(12, Some((0, 0)), [vec![q(y)], vec![q(cb)], vec![q(cr)]]);
        let px = to_rgba(&red, None, 6, false).rgba;
        assert!(px[0] >= 254 && px[1] <= 1 && px[2] <= 1, "{px:?}");
    }

    #[test]
    fn alpha_scales_by_its_own_depth() {
        let color = frame(8, None, [vec![255, 255, 255], vec![], vec![]]);
        let a8 = frame(8, None, [vec![0, 128, 255], vec![], vec![]]);
        let out = to_rgba(&color, Some(&a8), 1, true).rgba;
        assert_eq!((out[3], out[7], out[11]), (0, 128, 255));
        let a10 = frame(10, None, [vec![0, 512, 1023], vec![], vec![]]);
        let out = to_rgba(&color, Some(&a10), 1, true).rgba;
        assert_eq!((out[3], out[7], out[11]), (0, 128, 255));
    }

    #[test]
    fn limited_range_and_deep_samples_scale_to_eight_bits() {
        // Limited 8-bit: luma 16..235 is black..white; 10-bit full range: 1023 is white.
        let limited = frame(8, None, [vec![16, 235], vec![], vec![]]);
        assert_eq!(
            to_rgba(&limited, None, 1, false).rgba,
            [0, 0, 0, 255, 255, 255, 255, 255]
        );
        let deep = frame(10, None, [vec![1023, 512], vec![], vec![]]);
        assert_eq!(
            to_rgba(&deep, None, 1, true).rgba,
            [255, 255, 255, 255, 128, 128, 128, 255]
        );
        // A 10-bit alpha frame scales by its own depth.
        let alpha = frame(10, None, [vec![1023, 0], vec![], vec![]]);
        let out = to_rgba(&limited, Some(&alpha), 1, false).rgba;
        assert_eq!((out[3], out[7]), (255, 0));
    }
}
