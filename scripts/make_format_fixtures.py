#!/usr/bin/env python3
"""Generate the image-format fixtures (P5b, miaskiewicz/turbo-surf#30).

Writes crates/turbo-dom-layout/tests/fixtures/images/formats/ — the four-colour 64x40
quadrant image of make_orientation_fixtures.py (top-left red, top-right green,
bottom-left blue, bottom-right yellow) in the formats Chrome 156 decodes beyond
PNG/JPEG/GIF/WebP, plus a transparent-quadrant variant for the alpha paths:

- `q.avif`, `q-alpha.avif` — AVIF (ImageMagick + libheif/aom; 8-bit 4:2:0, lossless-ish);
- `q.bmp` (BMP3, 24-bit BGR), `q-alpha.bmp` (BMP4 32-bit with an alpha mask),
  `q-8bit.bmp` (8-bit palette);
- `q.ico` (one 64x40 BMP-encoded entry), `q-sizes.ico` (16, 32 and 48 px square
  entries of the image, BMP-encoded), `q-png.ico` (one PNG-encoded 64x40 entry);
- `q.cur` — `q.ico` as a cursor (header type 2, hotspot 0,0);
- `q.svg` — the quadrants as an SVG document (width/height 64x40);
- `q.tiff` — an uncompressed TIFF, which Chrome does NOT decode (the error control).

The binaries are committed; re-run this (needs `magick`) only to regenerate.
`IMAGES.sha256` pins them. Usage: python3 scripts/make_format_fixtures.py
"""
import hashlib
import struct
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ORIENTATION = ROOT / "crates/turbo-dom-layout/tests/fixtures/images/orientation/quad.png"
OUT = ROOT / "crates/turbo-dom-layout/tests/fixtures/images/formats"

SVG = (
    '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="40">'
    '<rect width="32" height="20" fill="#ff0000"/><rect x="32" width="32" height="20" fill="#00ff00"/>'
    '<rect y="20" width="32" height="20" fill="#0000ff"/><rect x="32" y="20" width="32" height="20" fill="#ffff00"/>'
    "</svg>\n"
)


def magick(*args: str) -> None:
    subprocess.run(["magick", *args], check=True)


def as_cursor(ico: bytes) -> bytes:
    """An ICO file as a CUR: header type 2, each entry's planes/bit-count fields become a
    (0, 0) hotspot (the embedded BMP header still carries the bit depth)."""
    count = struct.unpack("<H", ico[4:6])[0]
    out = bytearray(ico)
    out[2:4] = struct.pack("<H", 2)
    for i in range(count):
        entry = 6 + 16 * i
        out[entry + 4 : entry + 8] = struct.pack("<HH", 0, 0)
    return bytes(out)


def png_icon(png: bytes, width: int, height: int) -> bytes:
    """A one-entry ICO whose image is a PNG stream (the Vista+ encoding)."""
    header = struct.pack("<HHH", 0, 1, 1)
    entry = struct.pack("<BBBBHHII", width, height, 0, 0, 1, 32, len(png), 6 + 16)
    return header + entry + png


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    files: dict[str, bytes] = {}
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        quad = t / "quad.png"
        quad.write_bytes(ORIENTATION.read_bytes())
        alpha = t / "alpha.png"
        # The bottom-right quadrant fully transparent.
        magick(str(quad), "-alpha", "set", "-region", "32x20+32+20", "-alpha", "transparent", str(alpha))
        jobs = {
            "q.avif": [str(quad), "-quality", "95", "-define", "heic:chroma=444"],
            "q-alpha.avif": [str(alpha), "-quality", "95", "-define", "heic:chroma=444"],
            "q.bmp": [str(quad), "-strip", "BMP3:"],
            "q-alpha.bmp": [str(alpha), "-strip", "-define", "bmp:format=bmp4", "BMP:"],
            "q-8bit.bmp": [str(quad), "-strip", "-type", "Palette", "BMP3:"],
            "q.tiff": [str(quad), "-strip", "-compress", "none", "TIFF:"],
            "q.ico": [str(quad), "-strip", "-define", "icon:format=bmp", "ICO:"],
            "q-sizes.ico": [str(quad), "-strip", "-resize", "48x48!", "-define", "icon:auto-resize=48,32,16", "-define", "icon:format=bmp", "ICO:"],
        }
        for name, args in jobs.items():
            dest = t / name
            *head, fmt = args
            if fmt.endswith(":"):
                magick(*head, f"{fmt}{dest}")
            else:
                magick(*args, str(dest))
            files[name] = dest.read_bytes()
    files["q.cur"] = as_cursor(files["q.ico"])
    files["q-png.ico"] = png_icon(ORIENTATION.read_bytes(), 64, 40)
    files["q.svg"] = SVG.encode()
    lines = []
    for name in sorted(files):
        (OUT / name).write_bytes(files[name])
        lines.append(f"{hashlib.sha256(files[name]).hexdigest()}  {name}")
    (OUT / "IMAGES.sha256").write_text("\n".join(lines) + "\n")
    print(f"{len(files)} images -> {OUT}")


if __name__ == "__main__":
    main()
