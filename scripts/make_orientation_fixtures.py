#!/usr/bin/env python3
"""Generate the EXIF-orientation image fixtures (P5b, miaskiewicz/turbo-html2pdf#30).

Writes crates/turbo-dom-layout/tests/fixtures/images/orientation/:

- `quad.png`: a 64x40 stored image in four solid quadrants (top-left red, top-right
  green, bottom-left blue, bottom-right yellow), so the displayed orientation is
  readable from four pixel samples;
- `jpeg-o{1..8}.jpg`: the quadrant image as a baseline JPEG (ImageMagick, quality 95,
  metadata stripped) plus an EXIF APP1 segment (big-endian TIFF) with Orientation=N,
  inserted after the JFIF APP0 — `jpeg-o6-le.jpg` is the little-endian (`II`) form and
  `jpeg-o6-first.jpg` puts APP1 directly after SOI (no JFIF segment);
- `png-o6.png`: the PNG with an `eXIf` chunk (Orientation=6) before IDAT;
- `webp-o6.webp`: a lossless WebP (cwebp) rewrapped as extended `VP8X` with an `EXIF`
  chunk (Orientation=6); `webp-o6-exifhdr.webp` the same with an `Exif\0\0`-prefixed
  chunk body (Chrome 156 applies neither: WebP EXIF orientation is ignored).

The binaries are committed; re-run this (needs `magick` + `cwebp`) only to regenerate.
`IMAGES.sha256` pins them. Usage: python3 scripts/make_orientation_fixtures.py
"""
import hashlib
import struct
import subprocess
import tempfile
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/turbo-dom-layout/tests/fixtures/images/orientation"
W, H = 64, 40
QUADS = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0)]  # TL, TR, BL, BR


def png_chunk(kind: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def quad_png(exif: bytes | None = None) -> bytes:
    rows = b""
    for y in range(H):
        row = b"\x00"
        for x in range(W):
            row += bytes(QUADS[(2 if y >= H // 2 else 0) + (1 if x >= W // 2 else 0)])
        rows += row
    ihdr = struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)
    chunks = png_chunk(b"IHDR", ihdr)
    if exif is not None:
        chunks += png_chunk(b"eXIf", exif)
    chunks += png_chunk(b"IDAT", zlib.compress(rows, 9)) + png_chunk(b"IEND", b"")
    return b"\x89PNG\r\n\x1a\n" + chunks


def tiff(orientation: int, little: bool = False) -> bytes:
    e = "<" if little else ">"
    head = (b"II*\x00" if little else b"MM\x00*") + struct.pack(e + "I", 8)
    ifd = struct.pack(e + "H", 1) + struct.pack(e + "HHIHH", 0x0112, 3, 1, orientation, 0)
    return head + ifd + struct.pack(e + "I", 0)


def app1(orientation: int, little: bool = False) -> bytes:
    body = b"Exif\x00\x00" + tiff(orientation, little)
    return b"\xff\xe1" + struct.pack(">H", len(body) + 2) + body


def jpeg_with(base: bytes, segment: bytes, first: bool) -> bytes:
    assert base[:2] == b"\xff\xd8"
    if first:
        return base[:2] + segment + base[2:]
    assert base[2:4] == b"\xff\xe0", "JFIF APP0 expected after SOI"
    app0_len = struct.unpack(">H", base[4:6])[0]
    at = 4 + app0_len
    return base[:at] + segment + base[at:]


def riff_chunk(kind: bytes, data: bytes) -> bytes:
    pad = b"\x00" if len(data) % 2 else b""
    return kind + struct.pack("<I", len(data)) + data + pad


def webp_with_exif(lossless: bytes, orientation: int, exif_header: bool = False) -> bytes:
    assert lossless[:4] == b"RIFF" and lossless[8:16] == b"WEBPVP8L"
    vp8l_len = struct.unpack("<I", lossless[16:20])[0]
    vp8l = lossless[20 : 20 + vp8l_len]
    flags = 0x08  # EXIF present
    canvas = struct.pack("<I", W - 1)[:3] + struct.pack("<I", H - 1)[:3]
    vp8x = riff_chunk(b"VP8X", struct.pack("<I", flags) + canvas)
    exif = (b"Exif\x00\x00" if exif_header else b"") + tiff(orientation)
    body = b"WEBP" + vp8x + riff_chunk(b"VP8L", vp8l) + riff_chunk(b"EXIF", exif)
    return b"RIFF" + struct.pack("<I", len(body)) + body


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    files: dict[str, bytes] = {"quad.png": quad_png()}
    with tempfile.TemporaryDirectory() as tmp:
        src = Path(tmp) / "quad.png"
        src.write_bytes(files["quad.png"])
        jpg = Path(tmp) / "quad.jpg"
        subprocess.run(["magick", str(src), "-strip", "-quality", "95", str(jpg)], check=True)
        base = jpg.read_bytes()
        webp = Path(tmp) / "quad.webp"
        subprocess.run(["cwebp", "-quiet", "-lossless", "-exact", str(src), "-o", str(webp)], check=True)
        lossless = webp.read_bytes()
    for o in range(1, 9):
        files[f"jpeg-o{o}.jpg"] = jpeg_with(base, app1(o), first=False)
    files["jpeg-o6-le.jpg"] = jpeg_with(base, app1(6, little=True), first=False)
    files["jpeg-o6-first.jpg"] = jpeg_with(base, app1(6), first=True)
    files["png-o6.png"] = quad_png(exif=tiff(6))
    files["webp-o6.webp"] = webp_with_exif(lossless, 6)
    files["webp-o6-exifhdr.webp"] = webp_with_exif(lossless, 6, exif_header=True)
    lines = []
    for name in sorted(files):
        (OUT / name).write_bytes(files[name])
        lines.append(f"{hashlib.sha256(files[name]).hexdigest()}  {name}")
    (OUT / "IMAGES.sha256").write_text("\n".join(lines) + "\n")
    print(f"{len(files)} images -> {OUT}")


if __name__ == "__main__":
    main()
