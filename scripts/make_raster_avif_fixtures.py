#!/usr/bin/env python3
"""Generate turbo-dom-raster's AVIF decode fixtures (P5-cov).

Writes crates/turbo-dom-raster/tests/fixtures/images/avif/ — the four-colour 64x40 quadrant
image of turbo-dom-layout's orientation fixtures (top-left red, top-right green, bottom-left
blue, bottom-right yellow) as AVIFs that exercise the raster's YUV -> RGB paths
(src/avif.rs): every chroma layout and each matrix-coefficients family it weights, in both
ranges, plus a file without an `nclx` colour box (the AV1 sequence header's colour
description applies):

- `q-lossless.avif`  — `heif-enc -L`: 4:4:4, identity matrix (GBR, code 0), full range;
- `q-444-bt709-limited.avif` — 4:4:4, BT.709 (1), limited range;
- `q-422-bt601.avif` — 4:2:2, BT.601 (6), full range;
- `q-420-fcc.avif`   — 4:2:0, FCC (4), full range;
- `q-420-smpte240.avif` — 4:2:0, SMPTE 240M (7), full range;
- `q-420-bt2020.avif` — 4:2:0, BT.2020 non-constant luminance (9), limited range;
- `q-nocolr.avif`    — `q-444-bt709-limited.avif` with its `colr` box retyped `free` (an
  unknown property), so only the sequence header describes the colour;
- `q-10bit-limited.avif` / `q-12bit-full.avif` — 4:4:4 BT.709 at 10 bits limited range and
  4:2:0 BT.601 at 12 bits full range, from a 16-bit copy of the image.

Colour primaries stay BT.709 and the transfer sRGB, so Chrome shows the colours without
gamut mapping. The binaries are committed; re-run this (needs libheif's `heif-enc` with the
aom encoder, and ImageMagick for the 16-bit source) only to regenerate. `IMAGES.sha256` pins them. The Chrome 156 pixels are recorded
by scripts/raster-avif-chrome-control.mjs. Usage: python3 scripts/make_raster_avif_fixtures.py
"""
import hashlib
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
QUAD = ROOT / "crates/turbo-dom-layout/tests/fixtures/images/orientation/quad.png"
OUT = ROOT / "crates/turbo-dom-raster/tests/fixtures/images/avif"

COLOUR = ["--colour_primaries", "1", "--transfer_characteristic", "13"]
JOBS = {
    "q-lossless.avif": ["-L"],
    "q-444-bt709-limited.avif": ["-q", "90", "-p", "chroma=444", "--matrix_coefficients", "1", "--full_range_flag", "0", *COLOUR],
    "q-422-bt601.avif": ["-q", "90", "-p", "chroma=422", "--matrix_coefficients", "6", "--full_range_flag", "1", *COLOUR],
    "q-420-fcc.avif": ["-q", "90", "-p", "chroma=420", "--matrix_coefficients", "4", "--full_range_flag", "1", *COLOUR],
    "q-420-smpte240.avif": ["-q", "90", "-p", "chroma=420", "--matrix_coefficients", "7", "--full_range_flag", "1", *COLOUR],
    "q-420-bt2020.avif": ["-q", "90", "-p", "chroma=420", "--matrix_coefficients", "9", "--full_range_flag", "0", *COLOUR],
}


DEEP_JOBS = {
    "q-10bit-limited.avif": ["-b", "10", "-q", "90", "-p", "chroma=444", "--matrix_coefficients", "1", "--full_range_flag", "0", *COLOUR],
    "q-12bit-full.avif": ["-b", "12", "-q", "90", "-p", "chroma=420", "--matrix_coefficients", "6", "--full_range_flag", "1", *COLOUR],
}


def heif_enc(args: list[str], src: Path, dest: Path) -> bytes:
    subprocess.run(["heif-enc", "--avif", "-e", "aom", "--no-thumb-alpha", *args, "-o", str(dest), str(src)], check=True)
    return dest.read_bytes()


def without_colr(avif: bytes) -> bytes:
    """`avif` with its one `colr` property box retyped `free` (same size, so no offsets move)."""
    assert avif.count(b"colr") == 1, "expected exactly one colr box"
    return avif.replace(b"colr", b"free")


def main() -> None:
    files: dict[str, bytes] = {}
    with tempfile.TemporaryDirectory() as tmp:
        for name, args in JOBS.items():
            files[name] = heif_enc(args, QUAD, Path(tmp) / name)
        quad16 = Path(tmp) / "quad16.png"
        subprocess.run(["magick", str(QUAD), "-depth", "16", f"PNG48:{quad16}"], check=True)
        for name, args in DEEP_JOBS.items():
            files[name] = heif_enc(args, quad16, Path(tmp) / name)
    files["q-nocolr.avif"] = without_colr(files["q-444-bt709-limited.avif"])
    OUT.mkdir(parents=True, exist_ok=True)
    for name in sorted(files):
        (OUT / name).write_bytes(files[name])
    names = sorted(p.name for p in OUT.iterdir() if p.name != "IMAGES.sha256")
    lines = [f"{hashlib.sha256((OUT / n).read_bytes()).hexdigest()}  {n}" for n in names]
    (OUT / "IMAGES.sha256").write_text("\n".join(lines) + "\n")
    print(f"{len(files)} images -> {OUT}")


if __name__ == "__main__":
    main()
