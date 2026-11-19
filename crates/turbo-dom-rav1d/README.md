# turbo-dom-rav1d

turbo-dom's fork of [rav1d](https://github.com/memorysafety/rav1d) 1.1.0, the BSD-2-Clause
Rust port of the dav1d AV1 decoder, published to crates.io as `turbo-dom-rav1d`. It is the
AVIF decoder behind turbo-dom-raster's `avif` feature.

**Generated, not vendored.** No rav1d source is committed here. `scripts/generate.py`
takes the checksum-pinned upstream `rav1d-1.1.0.crate` (`pins.json`) and applies:

1. `patches/on-error-frame-hdr.patch`: upstream commit 6c6f46378 (memorysafety/rav1d#1497,
   on rav1d `main`, in no published release). With one frame context, any decode error
   made `on_error` unwrap a frame header that `rav1d_decode_frame_exit` had already taken,
   so it panicked.
2. Every `extern "C"` ABI becomes `extern "C-unwind"` (159 sites: the 19 `dav1d_*` entry
   points and the DSP function pointers they call). Behind `extern "C"` a panic cannot
   unwind and aborts the process. Behind `C-unwind` the caller can `catch_unwind` it
   (memorysafety/rav1d#1521).
3. The manifest is renamed to `turbo-dom-rav1d`. The library is still named `rav1d`, so
   `rav1d::src::lib::dav1d_*` paths do not change. Upstream's packaging records and nightly
   `rust-toolchain.toml` are dropped.

That's why a corrupt AVIF is a broken image instead of a process abort
(miaskiewicz/turbo-dom#535). turbo-dom-raster decodes on the calling thread
(`n_threads = 1`, `max_frame_delay = 1`) inside `catch_unwind`. See
`crates/turbo-dom-raster/src/avif.rs`.

```bash
python3 crates/turbo-dom-rav1d/scripts/generate.py --write            # -> crates/turbo-dom-rav1d/crate (gitignored)
python3 crates/turbo-dom-rav1d/scripts/generate.py --download --check # CI: reproduces the pinned tree
python3 -m unittest discover -s crates/turbo-dom-rav1d/scripts -p 'test_*.py'
```

A reviewed change to the patches or the transform is accepted with
`generate.py --write --update-pins`, then published as a new version. The step-by-step
procedure (dry run, gates, token, lockfile) is in [PUBLISHING.md](../../PUBLISHING.md#turbo-dom-rav1d);
1.1.0 was published on 2026-11-19. The crate is outside
the workspace (`exclude` in the root `Cargo.toml`). In a git worktree, cargo walks up to
the main checkout's workspace, so build or publish it from `--output` outside the
repository: `cargo publish --manifest-path <out>/Cargo.toml`. turbo-dom-raster takes it
from crates.io (`rav1d = { package = "turbo-dom-rav1d", version = "=1.1.0" }`).

Licence: BSD-2-Clause, as upstream (`COPYING` ships in the crate).
