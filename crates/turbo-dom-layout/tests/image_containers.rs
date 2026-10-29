//! Container edge cases of the image layer's format probes, built byte by byte from the
//! format specifications: ISOBMFF/HEIF/AVIF boxes (ISO/IEC 14496-12 §4.2 box sizes,
//! §8.11.3 `iloc` versions 0-2 and construction methods, §8.11.4 `pitm`, §4.3 `ftyp`
//! compatible brands; ISO/IEC 23008-12 `ispe`/`ipma`/`auxC`; the `colr` `nclx` body),
//! ICO directory entries (a 0 width/height byte means 256) and a JPEG Exif block whose
//! TIFF header is not a byte-order mark.
use turbo_dom_layout::image::{avif_payload, ico_entry, probe, Nclx, Orientation};

fn bx(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(body.len() + 8)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(body);
    out
}

fn full(kind: [u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut b = vec![version];
    b.extend_from_slice(&flags.to_be_bytes()[1..]);
    b.extend_from_slice(body);
    bx(kind, &b)
}

fn ftyp(major: [u8; 4], compatible: &[&[u8; 4]]) -> Vec<u8> {
    let mut body = major.to_vec();
    body.extend_from_slice(&0u32.to_be_bytes());
    for brand in compatible {
        body.extend_from_slice(*brand);
    }
    bx(*b"ftyp", &body)
}

fn ispe(w: u32, h: u32) -> Vec<u8> {
    let mut body = w.to_be_bytes().to_vec();
    body.extend_from_slice(&h.to_be_bytes());
    full(*b"ispe", 0, 0, &body)
}

fn colr(kind: [u8; 4], codes: [u16; 3], flags: u8) -> Vec<u8> {
    let mut body = kind.to_vec();
    for c in codes {
        body.extend_from_slice(&c.to_be_bytes());
    }
    body.push(flags);
    bx(*b"colr", &body)
}

fn alpha_aux() -> Vec<u8> {
    full(
        *b"auxC",
        0,
        0,
        b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha\0",
    )
}

/// `ipma` version 0, 7-bit indices: `(item, [1-based property index])`.
fn ipma(entries: &[(u16, &[u8])]) -> Vec<u8> {
    let mut body = u32::try_from(entries.len()).unwrap().to_be_bytes().to_vec();
    for (item, props) in entries {
        body.extend_from_slice(&item.to_be_bytes());
        body.push(u8::try_from(props.len()).unwrap());
        body.extend(props.iter().map(|p| p | 0x80));
    }
    full(*b"ipma", 0, 0, &body)
}

/// One `iloc` item: `(id, construction method, [(offset, length)])`.
type Item<'a> = (u32, u16, &'a [(u32, u32)]);

/// `iloc` with 4-byte offsets and lengths, no base offset / extent index.
fn iloc(version: u8, items: &[Item]) -> Vec<u8> {
    let mut body = vec![0x44, 0x00];
    let wide = version >= 2;
    let push_id = |body: &mut Vec<u8>, v: u32| {
        if wide {
            body.extend_from_slice(&v.to_be_bytes());
        } else {
            body.extend_from_slice(&u16::try_from(v).unwrap().to_be_bytes());
        }
    };
    push_id(&mut body, u32::try_from(items.len()).unwrap());
    for (id, method, extents) in items {
        push_id(&mut body, *id);
        if version >= 1 {
            body.extend_from_slice(&method.to_be_bytes());
        }
        body.extend_from_slice(&0u16.to_be_bytes()); // data reference index
        body.extend_from_slice(&u16::try_from(extents.len()).unwrap().to_be_bytes());
        for (offset, length) in *extents {
            body.extend_from_slice(&offset.to_be_bytes());
            body.extend_from_slice(&length.to_be_bytes());
        }
    }
    full(*b"iloc", version, 0, &body)
}

fn pitm(version: u8, item: u32) -> Vec<u8> {
    if version == 0 {
        full(*b"pitm", 0, 0, &u16::try_from(item).unwrap().to_be_bytes())
    } else {
        full(*b"pitm", version, 0, &item.to_be_bytes())
    }
}

fn meta(children: &[Vec<u8>]) -> Vec<u8> {
    full(*b"meta", 0, 0, &children.concat())
}

fn iprp(properties: &[Vec<u8>], associations: &[u8]) -> Vec<u8> {
    let mut body = bx(*b"ipco", &properties.concat());
    body.extend_from_slice(associations);
    bx(*b"iprp", &body)
}

/// An AVIF whose major brand is `mif1` (`avif` only among the compatible brands), the
/// primary item 1 (32-bit `pitm`) stored in `idat` (`iloc` v1, method 1) with an `nclx`
/// colour description, and the alpha item 2 in a trailing `mdat` whose size field is 0
/// (it runs to the end of the file), located by a zero-length extent (to the end too).
fn idat_avif() -> Vec<u8> {
    let head = |mdat_at: u32| {
        let mut file = ftyp(*b"mif1", &[b"miaf", b"avif"]);
        file.extend(meta(&[
            pitm(1, 1),
            iloc(
                1,
                &[
                    (1, 1, &[(2, 3)]),
                    (3, 1, &[(0, 2)]),
                    (2, 0, &[(mdat_at + 8, 0)]),
                ],
            ),
            // Item 3 (associated before the alpha item) is not an alpha auxiliary.
            iprp(
                &[ispe(64, 40), colr(*b"nclx", [1, 13, 6], 0x80), alpha_aux()],
                &ipma(&[(1, &[1, 2]), (3, &[1]), (2, &[3])]),
            ),
            bx(*b"idat", b"\0\0COL\0"),
        ]));
        file
    };
    let at = u32::try_from(head(0).len()).unwrap();
    let mut file = head(at);
    // `mdat` with size 0: to the end of the file.
    file.extend_from_slice(&0u32.to_be_bytes());
    file.extend_from_slice(b"mdat");
    file.extend_from_slice(b"ALPHA-BYTES");
    file
}

#[test]
fn avif_items_in_idat_and_a_to_end_mdat_demux_with_their_nclx() {
    let file = idat_avif();
    let intrinsic = probe(&file).expect("an AVIF");
    assert_eq!(
        (intrinsic.width, intrinsic.height, intrinsic.has_alpha),
        (64, 40, true)
    );
    assert_eq!(intrinsic.orientation, Orientation::IDENTITY);
    let payload = avif_payload(&file).expect("demuxes");
    assert_eq!(payload.color, b"COL");
    assert_eq!(payload.alpha.as_deref(), Some(&b"ALPHA-BYTES"[..]));
    assert_eq!(
        payload.nclx,
        Some(Nclx {
            primaries: 1,
            transfer: 13,
            matrix: 6,
            full_range: true,
        })
    );
}

/// A box written with a 64-bit `largesize` (its `size` field 1).
fn large(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = 1u32.to_be_bytes().to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(&(u64::try_from(body.len()).unwrap() + 16).to_be_bytes());
    out.extend_from_slice(body);
    out
}

/// An AVIF (major brand `avif`) with a 16-bit `pitm`, `iloc` version 2 (32-bit item ids
/// and count), its `meta` and the colour item's `mdat` written with a 64-bit
/// `largesize`, and the given `colr` property.
fn largesize_avif(colour: &[u8], method: u16) -> Vec<u8> {
    let head = |mdat_at: u32| {
        let mut file = ftyp(*b"avif", &[]);
        let mut body = vec![0; 4]; // full box version + flags
        for child in [
            pitm(0, 7),
            iloc(2, &[(7, method, &[(mdat_at + 16, 4)])]),
            iprp(&[ispe(3, 2), colour.to_vec()], &ipma(&[(7, &[1, 2])])),
        ] {
            body.extend(child);
        }
        file.extend(large(*b"meta", &body));
        file
    };
    let at = u32::try_from(head(0).len()).unwrap();
    let mut file = head(at);
    file.extend_from_slice(&1u32.to_be_bytes());
    file.extend_from_slice(b"mdat");
    file.extend_from_slice(&(16u64 + 4).to_be_bytes());
    file.extend_from_slice(b"AV1!");
    file
}

#[test]
fn a_last_meta_box_with_size_zero_runs_to_the_end_of_the_file() {
    // ISO/IEC 14496-12 §4.2: `size` 0 = the box extends to the end of the file. Here the
    // `meta` box is last (after the `mdat`), with an `iloc` version 0 item (16-bit ids,
    // file offsets).
    let mut file = ftyp(*b"avif", &[]);
    let data_at = u32::try_from(file.len()).unwrap() + 8;
    file.extend(bx(*b"mdat", b"AV1!"));
    file.extend_from_slice(&0u32.to_be_bytes());
    file.extend_from_slice(b"meta");
    file.extend_from_slice(&[0; 4]);
    for child in [
        pitm(0, 1),
        iloc(0, &[(1, 0, &[(data_at, 4)])]),
        iprp(&[ispe(5, 6)], &ipma(&[(1, &[1])])),
    ] {
        file.extend(child);
    }
    let intrinsic = probe(&file).expect("an AVIF");
    assert_eq!((intrinsic.width, intrinsic.height), (5, 6));
    assert_eq!(avif_payload(&file).unwrap().color, b"AV1!");
}

#[test]
fn avif_needs_an_avif_or_avis_brand() {
    // `ftyp` brands are 4 bytes: replacing the box keeps every offset.
    let file = idat_avif();
    let rebrand = |major: [u8; 4], compatible: &[&[u8; 4]]| {
        let mut bytes = ftyp(major, compatible);
        assert_eq!(bytes.len(), 24);
        bytes.extend_from_slice(&file[24..]);
        bytes
    };
    assert!(avif_payload(&rebrand(*b"heic", &[b"miaf", b"mif1"])).is_none());
    assert!(avif_payload(&rebrand(*b"avis", &[b"miaf", b"mif1"])).is_some());
    assert!(avif_payload(&rebrand(*b"mif1", &[b"miaf", b"avis"])).is_some());
}

#[test]
fn avif_version_2_iloc_and_a_largesize_mdat() {
    let file = largesize_avif(&colr(*b"nclx", [9, 16, 9], 0x00), 0);
    let intrinsic = probe(&file).expect("an AVIF");
    assert_eq!(
        (intrinsic.width, intrinsic.height, intrinsic.has_alpha),
        (3, 2, false)
    );
    let payload = avif_payload(&file).expect("demuxes");
    assert_eq!(payload.color, b"AV1!");
    assert_eq!(payload.alpha, None);
    assert_eq!(
        payload.nclx,
        Some(Nclx {
            primaries: 9,
            transfer: 16,
            matrix: 9,
            full_range: false,
        })
    );
    // An ICC `colr` is no `nclx` description.
    let icc = largesize_avif(&colr(*b"rICC", [9, 16, 9], 0x80), 0);
    assert_eq!(avif_payload(&icc).unwrap().nclx, None);
    // A truncated `nclx` body has no full-range byte.
    let short = largesize_avif(&bx(*b"colr", b"nclx\0\x01\0\x02\0\x03"), 0);
    assert_eq!(avif_payload(&short).unwrap().nclx, None);
}

#[test]
fn avif_items_with_an_unknown_construction_method_have_no_data() {
    // Construction method 2 (item offsets) is not supported: the primary item has no
    // data, so the file neither probes nor demuxes (Chrome rejects it too).
    let file = largesize_avif(&colr(*b"nclx", [1, 1, 1], 0), 2);
    assert!(probe(&file).is_none());
    assert!(avif_payload(&file).is_none());
    // Method 1 without an `idat` box: nothing to index either.
    let file = largesize_avif(&colr(*b"nclx", [1, 1, 1], 0), 1);
    assert!(avif_payload(&file).is_none());
}

#[test]
fn ico_entries_read_zero_dimension_bytes_as_256() {
    let mut ico = vec![0, 0, 1, 0, 1, 0];
    // width 0, height 0 (= 256), 0 colours, reserved, 1 plane, 32 bpp, size 4, offset 22.
    ico.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0, 4, 0, 0, 0, 22, 0, 0, 0]);
    ico.extend_from_slice(b"DIB!");
    let entry = ico_entry(&ico).expect("an ICO");
    assert_eq!((entry.width, entry.height, entry.png), (256, 256, false));
    assert_eq!(entry.data, b"DIB!");
    let i = probe(&ico).unwrap();
    assert_eq!((i.width, i.height, i.has_alpha), (256, 256, true));
    assert!(ico_entry(b"\x89PNG\r\n\x1a\n").is_none(), "not an ICO");
    // Two entries: the larger AREA wins (10x10 = 100 over 30x3 = 90), whichever has the
    // larger width or the larger width/height sum.
    let mut two = vec![0, 0, 1, 0, 2, 0];
    two.extend_from_slice(&[30, 3, 0, 0, 1, 0, 32, 0, 1, 0, 0, 0, 38, 0, 0, 0]);
    two.extend_from_slice(&[10, 10, 0, 0, 1, 0, 32, 0, 1, 0, 0, 0, 39, 0, 0, 0]);
    two.extend_from_slice(b"BA");
    let entry = ico_entry(&two).unwrap();
    assert_eq!((entry.width, entry.height, entry.data), (10, 10, &b"A"[..]));
    assert!(
        ico_entry(&[0, 0, 1, 0, 0, 0]).is_none(),
        "an empty directory"
    );
}

#[test]
fn a_jpeg_exif_block_without_a_byte_order_mark_is_identity() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/images/orientation/jpeg-o6.jpg"
    );
    let mut bytes = std::fs::read(path).unwrap();
    assert_eq!(probe(&bytes).unwrap().orientation.exif(), 6);
    let at = bytes
        .windows(6)
        .position(|w| w == b"Exif\0\0")
        .expect("an Exif APP1")
        + 6;
    bytes[at..at + 2].copy_from_slice(b"XX");
    let i = probe(&bytes).unwrap();
    assert_eq!(i.orientation, Orientation::IDENTITY);
}
