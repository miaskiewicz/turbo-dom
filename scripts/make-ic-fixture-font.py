#!/usr/bin/env python3
"""Write crates/turbo-dom-layout/assets/fonts/ic-fixture/IcFixture.ttf (turbo-dom#237).

A tiny TrueType face authored here (no third-party outlines, so it carries the repo's
own licence) whose only purpose is the CSS `ic` unit: it maps U+6C34 (the glyph `ic`
measures) to a glyph with a 900/1000 em advance, distinct from the 1em fallback, plus
`a`, `b` and space. Every glyph is a plain box. Deterministic: rerunning writes the
same bytes. Usage: python3 scripts/make-ic-fixture-font.py
"""
import os
import struct

UPEM = 1000
# (codepoint or None for .notdef, advance, box (xmin, ymin, xmax, ymax) or None)
GLYPHS = [
    (None, 500, (50, 0, 450, 700)),
    (0x20, 250, None),
    (0x61, 520, (40, 0, 480, 500)),
    (0x62, 540, (50, 0, 500, 720)),
    (0x6C34, 900, (60, -80, 840, 800)),
]


def box_glyph(box):
    xmin, ymin, xmax, ymax = box
    points = [(xmin, ymin), (xmin, ymax), (xmax, ymax), (xmax, ymin)]
    data = struct.pack(">hhhhh", 1, xmin, ymin, xmax, ymax)
    data += struct.pack(">H", 3)  # endPtsOfContours
    data += struct.pack(">H", 0)  # instructionLength
    data += bytes([0x01] * 4)  # on-curve, word coordinates
    prev = 0
    for x, _ in points:
        data += struct.pack(">h", x - prev)
        prev = x
    prev = 0
    for _, y in points:
        data += struct.pack(">h", y - prev)
        prev = y
    return data + b"\0" * (-len(data) % 4)


def cmap_table():
    mapped = sorted((cp, gid) for gid, (cp, _, _) in enumerate(GLYPHS) if cp is not None)
    segments = [(cp, cp, gid) for cp, gid in mapped] + [(0xFFFF, 0xFFFF, 0)]
    n = len(segments)
    search = 2 ** (n.bit_length() - 1)
    sub = struct.pack(">HHHH", n * 2, search * 2, search.bit_length() - 1, n * 2 - search * 2)
    sub += b"".join(struct.pack(">H", end) for _, end, _ in segments) + b"\0\0"
    sub += b"".join(struct.pack(">H", start) for start, _, _ in segments)
    sub += b"".join(struct.pack(">h", (gid - start) if gid else 1) for start, _, gid in segments)
    sub += b"\0\0" * n  # idRangeOffset
    sub = struct.pack(">HHH", 4, 6 + len(sub), 0) + sub
    return struct.pack(">HHHHI", 0, 1, 3, 1, 12) + sub


def name_table():
    names = [(1, "Ic Fixture"), (2, "Regular"), (3, "IcFixture-Regular"), (4, "Ic Fixture"),
             (5, "Version 1.000"), (6, "IcFixture-Regular")]
    strings = b""
    records = b""
    for name_id, text in names:
        encoded = text.encode("utf-16-be")
        records += struct.pack(">HHHHHH", 3, 1, 0x409, name_id, len(encoded), len(strings))
        strings += encoded
    return struct.pack(">HHH", 0, len(names), 6 + 12 * len(names)) + records + strings


def tables():
    glyf = b""
    loca = []
    for _, _, box in GLYPHS:
        loca.append(len(glyf))
        if box:
            glyf += box_glyph(box)
    loca.append(len(glyf))
    boxes = [b for _, _, b in GLYPHS if b]
    xmin = min(b[0] for b in boxes)
    ymin = min(b[1] for b in boxes)
    xmax = max(b[2] for b in boxes)
    ymax = max(b[3] for b in boxes)
    advances = [a for _, a, _ in GLYPHS]
    head = struct.pack(">HHIIIHHqqhhhhHHhhh", 1, 0, 0x00010000, 0, 0x5F0F3CF5, 0x000B, UPEM,
                       0, 0, xmin, ymin, xmax, ymax, 0, 8, 2, 1, 0)
    hhea = struct.pack(">HHhhhHhhhhhhhhhhhH", 1, 0, 880, -120, 0, max(advances), xmin, 0,
                       xmax, 1, 0, 0, 0, 0, 0, 0, 0, len(GLYPHS))
    hmtx = b"".join(struct.pack(">Hh", a, (b or (0,))[0]) for _, a, b in GLYPHS)
    maxp = struct.pack(">IHHHHHHHHHHHHHH", 0x00010000, len(GLYPHS), 4, 1, 0, 0, 2, 0, 0, 0,
                       0, 0, 0, 0, 0)
    os2 = struct.pack(">HhHHHhhhhhhhhhhh", 4, 600, 400, 5, 0, 650, 600, 0, 75, 650, 600, 0, 350,
                      50, 250, 0)
    os2 += b"\0" * 10  # panose
    os2 += struct.pack(">IIII", 1, 0, 0x00040000, 0)  # ulUnicodeRange (Latin, CJK)
    os2 += b"NONE"
    os2 += struct.pack(">HHH", 0x40, 0x20, 0x6C34)
    os2 += struct.pack(">hhhHH", 880, -120, 0, 880, 120)
    os2 += struct.pack(">II", 1 | (1 << 18), 0)  # ulCodePageRange (Latin 1, Chinese simplified)
    os2 += struct.pack(">hhHHH", 500, 700, 0, 0x20, 1)
    post = struct.pack(">IIhhIIIII", 0x00030000, 0, -100, 50, 0, 0, 0, 0, 0)
    return {
        b"OS/2": os2, b"cmap": cmap_table(), b"glyf": glyf, b"head": head, b"hhea": hhea,
        b"hmtx": hmtx, b"loca": b"".join(struct.pack(">I", o) for o in loca), b"maxp": maxp,
        b"name": name_table(), b"post": post,
    }


def checksum(data):
    data += b"\0" * (-len(data) % 4)
    return sum(struct.unpack(f">{len(data) // 4}I", data)) & 0xFFFFFFFF


def font():
    tabs = sorted(tables().items())
    n = len(tabs)
    search = 2 ** (n.bit_length() - 1)
    header = struct.pack(">IHHHH", 0x00010000, n, search * 16, search.bit_length() - 1,
                         n * 16 - search * 16)
    offset = 12 + 16 * n
    directory = b""
    body = b""
    for tag, data in tabs:
        directory += struct.pack(">4sIII", tag, checksum(data), offset + len(body), len(data))
        body += data + b"\0" * (-len(data) % 4)
    out = bytearray(header + directory + body)
    head_offset = 12 + 16 * n + body.index(dict(tabs)[b"head"])
    adjust = (0xB1B0AFBA - checksum(bytes(out))) & 0xFFFFFFFF
    out[head_offset + 8:head_offset + 12] = struct.pack(">I", adjust)
    return bytes(out)


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    target = os.path.join(here, "..", "crates", "turbo-dom-layout", "assets", "fonts", "ic-fixture")
    os.makedirs(target, exist_ok=True)
    with open(os.path.join(target, "IcFixture.ttf"), "wb") as f:
        f.write(font())
