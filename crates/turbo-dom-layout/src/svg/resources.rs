//! Admission before resvg's per-occurrence raster decoders allocate output buffers.
use super::{usvg, MAX_DEPTH, MAX_DIM};
use crate::image::{self, Format};

/// A decoded-raster admission failure in an SVG image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SvgResourceError {
    /// An embedded raster exceeds the SVG renderer's existing per-axis limit.
    RasterDimensions,
    /// Raster occurrences exceed one maximum SVG canvas worth of RGBA decoding.
    RasterBytes,
    /// The converted resource graph exceeds the SVG renderer's depth limit.
    ResourceDepth,
    /// A custom-property computation exceeds the existing variable nesting bound.
    CustomPropertyNesting,
    /// Computed custom tokens or XML growth exceed the source-derived bound.
    CustomPropertyBytes,
    /// This variable-dependent cascade/property path is not certified.
    UnsupportedCustomProperties,
}

impl std::fmt::Display for SvgResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::RasterDimensions => "SVG embedded raster exceeds the dimension budget",
            Self::RasterBytes => "SVG embedded rasters exceed the decoded-byte budget",
            Self::ResourceDepth => "SVG resource graph exceeds the depth budget",
            Self::CustomPropertyNesting => "SVG custom properties exceed the nesting budget",
            Self::CustomPropertyBytes => {
                "SVG custom properties exceed the source-derived token budget"
            }
            Self::UnsupportedCustomProperties => {
                "SVG variable-dependent paint is not certified for this cascade or property"
            }
        })
    }
}

impl std::error::Error for SvgResourceError {}

const MAX_RASTER_BYTES: u64 = MAX_DIM as u64 * MAX_DIM as u64 * 4;

fn frame_bytes(width: u32, height: u32) -> Result<u64, SvgResourceError> {
    if width > MAX_DIM || height > MAX_DIM {
        return Err(SvgResourceError::RasterDimensions);
    }
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(SvgResourceError::RasterBytes)
}

/// `None` denotes an invalid header, which retains ordinary missing-image behavior.
pub(super) fn raster_bytes(data: &[u8]) -> Result<Option<u64>, SvgResourceError> {
    let Some(size) = image::probe(data) else {
        return Ok(None);
    };
    frame_bytes(size.width, size.height)?;
    let dimensions = match image::sniff(data) {
        Some(Format::Gif) => image::gif_frame_size(data),
        _ => Some((size.width, size.height)),
    };
    dimensions
        .filter(|&(width, height)| width > 0 && height > 0)
        .map(|(width, height)| frame_bytes(width, height))
        .transpose()
}

#[derive(Default)]
struct Budget(u64);

impl Budget {
    fn charge(&mut self, bytes: u64) -> Result<(), SvgResourceError> {
        self.0 = self
            .0
            .checked_add(bytes)
            .filter(|&total| total <= MAX_RASTER_BYTES)
            .ok_or(SvgResourceError::RasterBytes)?;
        Ok(())
    }
}

enum Visit<'a> {
    Group(&'a usvg::Group),
    Clip(&'a usvg::ClipPath),
    Mask(&'a usvg::Mask),
}

type Pending<'a> = Vec<(Visit<'a>, usize)>;

/// Counts occurrences, not encoded resources: resvg decodes on every render call.
/// This is a conservative bound on decode work, not a claim about peak memory.
pub(super) fn audit(tree: &usvg::Tree) -> Result<(), SvgResourceError> {
    let mut pending = vec![(Visit::Group(tree.root()), 0)];
    let mut budget = Budget::default();
    while let Some((visit, depth)) = pending.pop() {
        if depth > MAX_DEPTH {
            return Err(SvgResourceError::ResourceDepth);
        }
        match visit {
            Visit::Group(group) => group_resources(group, depth, &mut pending, &mut budget)?,
            Visit::Clip(clip) => {
                pending.push((Visit::Group(clip.root()), depth + 1));
                if let Some(clip) = clip.clip_path() {
                    pending.push((Visit::Clip(clip), depth + 1));
                }
            }
            Visit::Mask(mask) => {
                pending.push((Visit::Group(mask.root()), depth + 1));
                if let Some(mask) = mask.mask() {
                    pending.push((Visit::Mask(mask), depth + 1));
                }
            }
        }
    }
    Ok(())
}

fn group_resources<'a>(
    group: &'a usvg::Group,
    depth: usize,
    pending: &mut Pending<'a>,
    budget: &mut Budget,
) -> Result<(), SvgResourceError> {
    if let Some(clip) = group.clip_path() {
        pending.push((Visit::Clip(clip), depth + 1));
    }
    if let Some(mask) = group.mask() {
        pending.push((Visit::Mask(mask), depth + 1));
    }
    for filter in group.filters() {
        for primitive in filter.primitives() {
            if let usvg::filter::Kind::Image(image) = primitive.kind() {
                pending.push((Visit::Group(image.root()), depth + 1));
            }
        }
    }
    for node in group.children() {
        node_resources(node, depth + 1, pending, budget)?;
    }
    Ok(())
}

fn node_resources<'a>(
    node: &'a usvg::Node,
    depth: usize,
    pending: &mut Pending<'a>,
    budget: &mut Budget,
) -> Result<(), SvgResourceError> {
    match node {
        usvg::Node::Group(group) => pending.push((Visit::Group(group), depth)),
        usvg::Node::Image(image) => image_resources(image.kind(), depth, pending, budget)?,
        usvg::Node::Path(path) => {
            if let Some(fill) = path.fill() {
                paint_resources(fill.paint(), depth, pending);
            }
            if let Some(stroke) = path.stroke() {
                paint_resources(stroke.paint(), depth, pending);
            }
        }
        usvg::Node::Text(text) => pending.push((Visit::Group(text.flattened()), depth)),
    }
    Ok(())
}

fn paint_resources<'a>(paint: &'a usvg::Paint, depth: usize, pending: &mut Pending<'a>) {
    if let usvg::Paint::Pattern(pattern) = paint {
        pending.push((Visit::Group(pattern.root()), depth));
    }
}

fn image_resources<'a>(
    image: &'a usvg::ImageKind,
    depth: usize,
    pending: &mut Pending<'a>,
    budget: &mut Budget,
) -> Result<(), SvgResourceError> {
    let data = match image {
        usvg::ImageKind::SVG(tree) => {
            pending.push((Visit::Group(tree.root()), depth));
            return Ok(());
        }
        usvg::ImageKind::PNG(data)
        | usvg::ImageKind::JPEG(data)
        | usvg::ImageKind::GIF(data)
        | usvg::ImageKind::WEBP(data) => data,
    };
    if let Some(bytes) = raster_bytes(data)? {
        budget.charge(bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_and_cumulative_limits_derive_from_existing_canvas_limit() {
        assert_eq!(frame_bytes(MAX_DIM, MAX_DIM), Ok(MAX_RASTER_BYTES));
        assert_eq!(
            frame_bytes(MAX_DIM + 1, 1),
            Err(SvgResourceError::RasterDimensions)
        );
        let mut budget = Budget::default();
        assert_eq!(budget.charge(MAX_RASTER_BYTES), Ok(()));
        assert_eq!(budget.charge(1), Err(SvgResourceError::RasterBytes));
        assert_eq!(
            Budget(u64::MAX).charge(1),
            Err(SvgResourceError::RasterBytes)
        );
    }

    #[test]
    fn gif_admission_uses_actual_frame_not_only_logical_screen() {
        // Tiny logical screen, but a first frame above the existing SVG axis cap.
        let mut gif = b"GIF89a\x01\0\x01\0\0\0\0\x2c\0\0\0\0".to_vec();
        gif.extend_from_slice(&(MAX_DIM as u16 + 1).to_le_bytes());
        gif.extend_from_slice(&1u16.to_le_bytes());
        gif.push(0);
        assert_eq!(raster_bytes(&gif), Err(SvgResourceError::RasterDimensions));
        assert_eq!(raster_bytes(&gif[..gif.len() - 3]), Ok(None));
    }
}
