use std::iter;

use resvg::tiny_skia::{self};

#[derive(Debug)]
pub struct Bounds {
    pub l: u32,
    pub r: u32,
    pub t: u32,
    pub b: u32,
}

impl Bounds {
    pub fn is_empty(&self) -> bool {
        self.l == 0 && self.r == 0 && self.t == 0 && self.b == 0
    }

    fn scale_value(value: u32, amount: f32) -> u32 {
        if value == 0 {
            return 0;
        }

        // prefer ceil over round
        // rationale:
        //   if a range of 5 is fixed size, then a range of 6 might be fixed size, while 4 will likely be too small
        //   prefer larger values rather than rounding to nearest value
        (value as f32 * amount).ceil().max(1.0) as u32
    }

    pub fn scale(&self, amount: f32) -> Self {
        Self {
            l: Self::scale_value(self.l, amount),
            r: Self::scale_value(self.r, amount),
            t: Self::scale_value(self.t, amount),
            b: Self::scale_value(self.b, amount),
        }
    }

    pub fn paint(&self, pixmap: &mut tiny_skia::PixmapMut, paint: &tiny_skia::Paint) {
        if self.is_empty() {
            return;
        }

        let img_width = pixmap.width() as f32;
        let img_height = pixmap.height() as f32;

        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, (self.l + 1) as f32, 1.0).unwrap(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, (self.t + 1) as f32).unwrap(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(
                img_width - (self.r + 1) as f32,
                img_height - 1.0,
                (self.r + 1) as f32,
                1.0,
            )
            .unwrap(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(
                img_width - 1.0,
                img_height - (self.b + 1) as f32,
                1.0,
                (self.b + 1) as f32,
            )
            .unwrap(),
            &paint,
            tiny_skia::Transform::identity(),
            None,
        );
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            l: Default::default(),
            r: Default::default(),
            t: Default::default(),
            b: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BoundPixel {
    Yellow,
    Pink,
    Transparent,
}

/// Return (yellow, pink) bound widths (subtracted by 2 to ignore the 1px border)
/// `Some` means it has a 1px border. `None` means it has no border.
fn parse_bound_side(
    img: &resvg::tiny_skia::Pixmap,
    x_iter: impl Iterator<Item = u32>,
    y_iter: impl Iterator<Item = u32>,
) -> Option<(u32, u32)> {
    let x_iter: Vec<_> = x_iter.collect();
    let y_iter: Vec<_> = y_iter.collect();

    let mut result = Vec::new();

    for x in x_iter.iter() {
        for y in y_iter.iter() {
            let pixel = img
                .pixel(*x, *y)
                .expect(format!("pixel out of bounds ({x}, {y})").as_str());
            let is_empty = pixel.alpha() == 0;
            if is_empty {
                result.push(BoundPixel::Transparent);
                continue;
            }

            let is_yellow = pixel.alpha() == 255
                && pixel.red() == 255
                && pixel.green() == 255
                && pixel.blue() == 0;
            if is_yellow {
                result.push(BoundPixel::Yellow);
                continue;
            }

            let is_pink = pixel.alpha() == 255
                && pixel.red() == 255
                && pixel.green() == 0
                && pixel.blue() == 255;
            if is_pink {
                result.push(BoundPixel::Pink);
                continue;
            }

            // encountered invalid pixel, therefore this is not a valid REAPER bound border
            return None;
        }
    }

    // image must be minimum of 3 pixels in width / height
    if result.len() < 3 {
        return None;
    }

    // find the semantic width of the yellow/pink lines
    // e.g. if a pink line is 3px long, it represents a 2px region
    let mut yellow_width: u32 = 0;
    let mut pink_width: u32 = 0;
    let mut prev_pixel: Option<BoundPixel> = None;
    for (i, pixel) in result.iter().enumerate() {
        match prev_pixel {
            None => match pixel {
                BoundPixel::Yellow => {
                    prev_pixel = Some(BoundPixel::Yellow);
                    yellow_width = i as u32;
                }
                BoundPixel::Pink => {
                    prev_pixel = Some(BoundPixel::Pink);
                    pink_width = i as u32;
                }
                BoundPixel::Transparent => return None,
            },
            Some(BoundPixel::Yellow) => match pixel {
                BoundPixel::Yellow => {
                    yellow_width = i as u32;
                }
                BoundPixel::Pink => {
                    prev_pixel = Some(BoundPixel::Pink);
                    pink_width = i as u32;
                }
                BoundPixel::Transparent => {
                    prev_pixel = Some(BoundPixel::Transparent);
                }
            },
            Some(BoundPixel::Pink) => match pixel {
                BoundPixel::Pink => pink_width = i as u32,
                BoundPixel::Transparent => prev_pixel = Some(BoundPixel::Transparent),
                // invalid sequence, pink -> yellow
                BoundPixel::Yellow => return None,
            },
            Some(BoundPixel::Transparent) => match pixel {
                BoundPixel::Transparent => continue,
                // invalid sequences, transparent -> yellow/pink
                BoundPixel::Yellow => return None,
                BoundPixel::Pink => return None,
            },
        }
    }

    let max_width = (result.len() - 2) as u32;

    Some((yellow_width.min(max_width), pink_width.min(max_width)))
}

pub fn detect_reaper_bounds(img: &resvg::tiny_skia::Pixmap) -> Option<(Bounds, Bounds)> {
    if img.width() == 0 || img.height() == 0 {
        return None;
    }

    // from top left->right
    let left = parse_bound_side(&img, 0..img.width(), iter::once(0))?;
    // from left top->bottom
    let top = parse_bound_side(&img, iter::once(0), 0..img.height())?;
    // from bottom right->left
    let right = parse_bound_side(&img, (0..img.width()).rev(), iter::once(img.height() - 1))?;
    // from right bottom->top
    let bottom = parse_bound_side(&img, iter::once(img.width() - 1), (0..img.height()).rev())?;

    let yellow_bounds = Bounds {
        t: top.0,
        l: left.0,
        b: bottom.0,
        r: right.0,
    };
    let pink_bounds = Bounds {
        t: top.1,
        l: left.1,
        b: bottom.1,
        r: right.1,
    };

    Some((yellow_bounds, pink_bounds))
}
