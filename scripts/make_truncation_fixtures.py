#!/usr/bin/env python3
"""Generate the truncation fixtures (P5b-X7, miaskiewicz/turbo-dom#59).

Writes crates/turbo-dom-layout/tests/fixtures/images/truncation/ — small complete images
whose every byte prefix `scripts/image-truncation-chrome-control.mjs` loads in Chrome 156
(as `data:` `<img>`s) to record where Blink's decoders first report a size:

- `baseline.jpg` (5x4 baseline JPEG), `progressive.jpg` (5x4 progressive JPEG);
- `still.gif` (6x5), `animated.gif` (6x5, two frames, the second a 2x2 sub-image);
- `lossy.webp` (7x6 `VP8 `), `lossless.webp` (7x6 `VP8L`), `alpha.webp` (7x6 `VP8X` +
  `ALPH` + `VP8 `), `animated.webp` (7x6 `VP8X` + `ANIM` + two `ANMF` frames);
- `still.png` (3x2 RGBA; the PNG rule — the first `IDAT` — already matched Chrome).

ImageMagick / libwebp output is not byte-stable across versions: the binaries are
committed and `IMAGES.sha256` pins them; re-run this (needs `magick`, `cwebp`, `img2webp`)
only to regenerate, then recapture the control.
Usage: python3 scripts/make_truncation_fixtures.py
"""
import hashlib
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "crates/turbo-dom-layout/tests/fixtures/images/truncation"


def run(*args: str) -> None:
    subprocess.run(list(args), check=True)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        t = Path(tmp)
        # A gradient so the entropy-coded data is more than a handful of bytes.
        run("magick", "-size", "5x4", "gradient:#ff0000-#0000ff", "-strip", "-quality", "90",
            str(OUT / "baseline.jpg"))
        run("magick", "-size", "5x4", "gradient:#ff0000-#0000ff", "-strip", "-quality", "90",
            "-interlace", "JPEG", str(OUT / "progressive.jpg"))
        run("magick", "-size", "6x5", "gradient:#ff0000-#00ff00", "-strip", str(OUT / "still.gif"))
        run("magick", "-size", "6x5", "xc:#ff0000", "-strip",
            "(", "-size", "2x2", "xc:#0000ff", ")", "-set", "delay", "10", "-loop", "0",
            str(t / "animated.gif"))
        (OUT / "animated.gif").write_bytes((t / "animated.gif").read_bytes())
        run("magick", "-size", "7x6", "gradient:#ff0000-#00ff00", str(t / "rgb.png"))
        run("magick", "-size", "7x6", "gradient:#ff000080-#00ff00ff", str(t / "rgba.png"))
        run("magick", "-size", "7x6", "xc:#0000ff", str(t / "blue.png"))
        run("cwebp", "-quiet", "-q", "80", "-metadata", "none", str(t / "rgb.png"), "-o",
            str(OUT / "lossy.webp"))
        run("cwebp", "-quiet", "-lossless", "-metadata", "none", str(t / "rgb.png"), "-o",
            str(OUT / "lossless.webp"))
        run("cwebp", "-quiet", "-q", "80", "-metadata", "none", str(t / "rgba.png"), "-o",
            str(OUT / "alpha.webp"))
        run("img2webp", "-loop", "0", "-lossy", "-d", "100", str(t / "rgb.png"), str(t / "blue.png"),
            "-o", str(OUT / "animated.webp"))
        run("magick", "-size", "3x2", "xc:#ff0000ff", "-strip", "PNG32:" + str(OUT / "still.png"))
    lines = [
        f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
        for p in sorted(OUT.iterdir())
        if p.name != "IMAGES.sha256"
    ]
    (OUT / "IMAGES.sha256").write_text("".join(lines))
    for p in sorted(OUT.iterdir()):
        print(p.name, p.stat().st_size)


if __name__ == "__main__":
    main()
