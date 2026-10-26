# JPEG XL test images

Three files from the jxl-rs test suite (libjxl/jxl-rs v0.7.4 `jxl/tests/testdata/`, the version
Chromium 156 vendors as `third_party/rust/chromium_crates_io/vendor/jxl-v0_7`), used unmodified as
test data for the JPEG XL probe/decode (miaskiewicz/turbo-dom#45 item 2). Copyright (c) the JPEG XL
Project Authors, BSD-3-Clause — see [LICENSE](LICENSE) (the jxl-rs license, copied verbatim).

| file | form | size | Chrome 156 |
|---|---|---|---|
| `exif.jxl` | ISOBMFF container (`jxlc`) | 256x128 | loads, solid black |
| `multiple_aux.jxl` | ISOBMFF container | 256x128 | loads, solid black |
| `modular_rle_fast_path.jxl` | bare codestream (`FF 0A`) | 8x8 | loads, solid black |

No JPEG XL encoder is installed to make the four-colour fixtures the other formats use.
`IMAGES.sha256` pins the files.
