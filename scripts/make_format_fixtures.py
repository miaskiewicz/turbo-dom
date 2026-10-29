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
- `q.tiff` — an uncompressed TIFF, which Chrome does NOT decode (the error control);
- `q-irot90.avif`, `q-irot180.avif`, `q-irot270.avif`, `q-imir0.avif`, `q-imir1.avif`,
  `q-irot90-imir0.avif` — `q.avif` with HEIF transform properties added to its primary
  item (miaskiewicz/turbo-dom#45): `irot` (angle x 90 degrees anti-clockwise) and/or
  `imir` (axis 0: top and bottom exchanged, 1: left and right — libavif's and Chrome's
  reading), in that order. No installed
  encoder writes them, so they are spliced into the container: appended to `ipco`,
  associated (essential) in `ipma`, the box sizes and `iloc` offsets moved accordingly.

The binaries are committed; re-run this (needs `magick`) only to regenerate.
`IMAGES.sha256` pins them. Usage: python3 scripts/make_format_fixtures.py
(`--avif-orientation`: only re-derive the transformed AVIFs from the committed `q.avif`).
"""
import hashlib
import struct
import sys
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


def box(kind: bytes, body: bytes) -> bytes:
    return struct.pack(">I", 8 + len(body)) + kind + body


def children(data: bytes, start: int, end: int) -> list[tuple[bytes, int, int]]:
    """The `(type, offset, size)` of the boxes in `data[start:end]` (32-bit sizes)."""
    out = []
    while start < end:
        size, kind = struct.unpack(">I4s", data[start : start + 8])
        out.append((kind, start, size))
        start += size
    return out


def find(boxes: list[tuple[bytes, int, int]], kind: bytes) -> tuple[bytes, int, int]:
    return next(b for b in boxes if b[0] == kind)


def with_transforms(avif: bytes, props: list[bytes]) -> bytes:
    """`avif` (single-item, version-0 `ipma`/`iloc`, `mdat` after `meta`, as ImageMagick
    writes it) with the property boxes `props` appended to `ipco` and associated, as
    essential properties in that order, with the primary item."""
    top = children(avif, 0, len(avif))
    _, meta_at, meta_size = find(top, b"meta")
    meta = children(avif, meta_at + 12, meta_at + meta_size)
    _, iprp_at, iprp_size = find(meta, b"iprp")
    iprp = children(avif, iprp_at + 8, iprp_at + iprp_size)
    _, ipco_at, ipco_size = find(iprp, b"ipco")
    _, ipma_at, ipma_size = find(iprp, b"ipma")
    _, iloc_at, iloc_size = find(meta, b"iloc")
    first = len(children(avif, ipco_at + 8, ipco_at + ipco_size)) + 1
    ipco = avif[ipco_at : ipco_at + ipco_size] + b"".join(props)
    ipco = struct.pack(">I", len(ipco)) + ipco[4:]
    # ipma v0, flags 0: entry_count u32, then item_ID u16, association_count u8, u8 each.
    body = bytearray(avif[ipma_at + 8 : ipma_at + ipma_size])
    assert body[0] == 0 and body[3] & 1 == 0 and struct.unpack(">I", body[4:8])[0] == 1
    count = body[10]
    body[10] = count + len(props)
    body[11 + count : 11 + count] = bytes(0x80 | (first + i) for i in range(len(props)))
    ipma = box(b"ipma", bytes(body))
    delta = len(ipco) - ipco_size + len(ipma) - ipma_size
    # iloc v0, 4-byte offsets/lengths/base offsets, one item: shift its base offset (the
    # `mdat` data after `meta`) by the bytes added.
    iloc = bytearray(avif[iloc_at : iloc_at + iloc_size])
    assert iloc[8] == 0 and iloc[12] == 0x44 and iloc[13] == 0x40
    base = 20  # box header 8, version/flags 4, sizes 2, item_count 2, item_ID 2, dri 2
    old = struct.unpack(">I", iloc[base : base + 4])[0]
    iloc[base : base + 4] = struct.pack(">I", old + delta)
    out = bytearray(avif)
    # Replace back to front so the earlier offsets stay valid (iloc < iprp/ipco < ipma).
    out[ipma_at : ipma_at + ipma_size] = ipma
    out[ipco_at : ipco_at + ipco_size] = ipco
    out[iloc_at : iloc_at + iloc_size] = iloc
    for at in (iprp_at, meta_at):
        out[at : at + 4] = struct.pack(">I", struct.unpack(">I", out[at : at + 4])[0] + delta)
    return bytes(out)


def irot(angle: int) -> bytes:
    return box(b"irot", bytes([angle]))


def imir(axis: int) -> bytes:
    return box(b"imir", bytes([axis]))


def oriented_avifs(avif: bytes) -> dict[str, bytes]:
    return {
        "q-irot90.avif": with_transforms(avif, [irot(1)]),
        "q-irot180.avif": with_transforms(avif, [irot(2)]),
        "q-irot270.avif": with_transforms(avif, [irot(3)]),
        "q-imir0.avif": with_transforms(avif, [imir(0)]),
        "q-imir1.avif": with_transforms(avif, [imir(1)]),
        "q-irot90-imir0.avif": with_transforms(avif, [irot(1), imir(0)]),
    }


def write(files: dict[str, bytes]) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name in sorted(files):
        (OUT / name).write_bytes(files[name])
    names = sorted(p.name for p in OUT.iterdir() if p.name != "IMAGES.sha256")
    lines = [f"{hashlib.sha256((OUT / n).read_bytes()).hexdigest()}  {n}" for n in names]
    (OUT / "IMAGES.sha256").write_text("\n".join(lines) + "\n")
    print(f"{len(files)} images -> {OUT}")


def main() -> None:
    if "--avif-orientation" in sys.argv[1:]:
        write(oriented_avifs((OUT / "q.avif").read_bytes()))
        return
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
    files.update(oriented_avifs(files["q.avif"]))
    write(files)


if __name__ == "__main__":
    main()
