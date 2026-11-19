//! Device-pixel replacement for `ImageData`, independent of canvas drawing state.
use serde_json::Value;
use turbo_dom_layout::Rgba;

use crate::canvas_ops::CanvasBackend;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageData {
    width: u32,
    pixels: Vec<u8>,
    destination: [i64; 2],
    dirty: [i64; 4],
}

impl ImageData {
    pub(crate) fn parse(args: &[Value]) -> Option<Self> {
        let image = args.first()?;
        let (width, height) = dimensions(image)?;
        if !supported_format(image) {
            return None;
        }
        let pixels = pixels(image, width, height)?;
        let destination = [integer(args, 1)?, integer(args, 2)?];
        let dirty = dirty_rectangle(args, width, height)?;
        Some(Self {
            width,
            pixels,
            destination,
            dirty,
        })
    }

    pub(crate) fn paint(&self, backend: &mut dyn CanvasBackend) {
        let [left, top, right, bottom] = self.visible_rectangle(backend.bitmap_size());
        for y in top..bottom {
            for x in left..right {
                let at = ((y as u64 * u64::from(self.width) + x as u64) * 4) as usize;
                let [r, g, b, a] = self.pixels[at..at + 4]
                    .try_into()
                    .expect("validated RGBA data");
                backend.replace_pixel(
                    x + self.destination[0],
                    y + self.destination[1],
                    Rgba::new(r, g, b, a),
                );
            }
        }
    }
    fn visible_rectangle(&self, (width, height): (u32, u32)) -> [i64; 4] {
        let [x, y] = self.destination;
        let [left, top, right, bottom] = self.dirty;
        [
            left.max(-x),
            top.max(-y),
            right.min(i64::from(width) - x),
            bottom.min(i64::from(height) - y),
        ]
    }
}

fn dimensions(image: &Value) -> Option<(u32, u32)> {
    let width = dimension(image, "width")?;
    let height = dimension(image, "height")?;
    (!crate::canvas_too_large(width, height)).then_some((width, height))
}

fn dimension(image: &Value, name: &str) -> Option<u32> {
    u32::try_from(image.get(name)?.as_u64()?)
        .ok()
        .filter(|v| *v != 0)
}

fn pixels(image: &Value, width: u32, height: u32) -> Option<Vec<u8>> {
    let bytes = image.get("data")?.as_array()?;
    if bytes.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return None;
    }
    bytes
        .iter()
        .map(|v| u8::try_from(v.as_u64()?).ok())
        .collect()
}

fn supported_format(image: &Value) -> bool {
    image
        .get("colorSpace")
        .is_none_or(|v| v.as_str() == Some("srgb"))
        && image
            .get("pixelFormat")
            .is_none_or(|v| v.as_str() == Some("rgba-unorm8"))
}

// The producer already performs WebIDL signed-long conversion. Keep integer arithmetic
// here: float coordinates would lose pixel offsets near the signed-long limits.
fn integer(args: &[Value], index: usize) -> Option<i64> {
    args.get(index)?
        .as_i64()
        .filter(|v| i32::try_from(*v).is_ok())
}

fn dirty_rectangle(args: &[Value], width: u32, height: u32) -> Option<[i64; 4]> {
    let [x, y, w, h] = if args.len() == 3 {
        [0, 0, i64::from(width), i64::from(height)]
    } else {
        [
            integer(args, 3)?,
            integer(args, 4)?,
            integer(args, 5)?,
            integer(args, 6)?,
        ]
    };
    Some([
        x.min(x + w).clamp(0, i64::from(width)),
        y.min(y + h).clamp(0, i64::from(height)),
        x.max(x + w).clamp(0, i64::from(width)),
        y.max(y + h).clamp(0, i64::from(height)),
    ])
}
