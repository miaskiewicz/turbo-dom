//! Parse a CSS `@font-face` `unicode-range` descriptor into `[u32]` ranges
//! (miaskiewicz/turbo-dom#539, CSS Fonts 4 §4.5). Google Fonts / Next.js split
//! their webfonts across several `@font-face` rules keyed by this descriptor;
//! without it every face advertises coverage of the whole BMP and a Latin-Extended
//! subset ends up serving `A-Z` as NO GLYPH.

use std::ops::RangeInclusive;

const UNICODE_MAX: u32 = 0x10_FFFF;

/// Parse a `unicode-range` descriptor value into ordered ranges. Returns `None`
/// for a malformed input; a browser treats a malformed descriptor as the default
/// (full Unicode), but a caller that already stores "no declared range" as absent
/// can collapse the two cases cheaply.
///
/// Accepts single codepoints (`U+26`), explicit ranges (`U+0100-02BA`), the wildcard
/// shorthand (`U+??` ≡ `U+00-FF`, `U+4??` ≡ `U+400-4FF`, `U+????` ≡ full BMP; wildcards
/// expand to the smallest/largest hex digit for each `?`) and comma-separated lists
/// of any of these. The hex is ASCII case-insensitive, and the leading `U+` must be
/// present (lower-case `u+` is accepted too).
#[must_use]
pub fn parse_unicode_range(css: &str) -> Option<Vec<RangeInclusive<u32>>> {
    let mut out = Vec::new();
    for token in css.split(',') {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return None;
        }
        out.push(parse_token(trimmed)?);
    }
    (!out.is_empty()).then_some(out)
}

fn parse_token(token: &str) -> Option<RangeInclusive<u32>> {
    let rest = strip_prefix_ci(token, "U+")?;
    if rest.is_empty() {
        return None;
    }
    if rest.contains('?') {
        parse_wildcard(rest)
    } else if let Some((start, end)) = rest.split_once('-') {
        let start = hex(start)?;
        let end = hex(end)?;
        guard(start, end)
    } else {
        let value = hex(rest)?;
        guard(value, value)
    }
}

fn strip_prefix_ci<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
    let head = input.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        return input.get(prefix.len()..);
    }
    None
}

fn parse_wildcard(hex_digits: &str) -> Option<RangeInclusive<u32>> {
    // The wildcard form is one block: every `?` stands for any hex digit, and
    // every `?` must be at the trailing end of the hex digits (CSS Fonts 4 §4.5:
    // "all remaining digits after the first `?` must also be `?`"). The expansion
    // is `[prefix||'0'*n, prefix||'F'*n]`.
    if hex_digits.len() > 6 {
        return None;
    }
    let bytes = hex_digits.as_bytes();
    let first_q = bytes.iter().position(|b| *b == b'?')?;
    if bytes[first_q..].iter().any(|b| *b != b'?') {
        return None;
    }
    let (prefix, wild) = hex_digits.split_at(first_q);
    let prefix_value = if prefix.is_empty() {
        0u32
    } else if !prefix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    } else {
        u32::from_str_radix(prefix, 16).ok()?
    };
    let shift = 4 * wild.len() as u32;
    let start = prefix_value << shift;
    let span = (1u32 << shift).checked_sub(1)?;
    let end = start.checked_add(span)?;
    guard(start, end)
}

fn hex(digits: &str) -> Option<u32> {
    let digits = digits.trim();
    if digits.is_empty() || digits.len() > 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

fn guard(start: u32, end: u32) -> Option<RangeInclusive<u32>> {
    (start <= end && end <= UNICODE_MAX).then_some(start..=end)
}

/// Whether `ranges` covers `ch`. An empty slice is the browser default "every
/// codepoint", so a face without a declared range keeps its pre-#539 behaviour.
#[must_use]
pub(super) fn covers(ranges: &[RangeInclusive<u32>], ch: char) -> bool {
    if ranges.is_empty() {
        return true;
    }
    let value = ch as u32;
    ranges.iter().any(|range| range.contains(&value))
}
