use resvg::tiny_skia::{self, Pixmap};
use thiserror::Error;

use crate::bounds::{self, Bounds};

pub enum UpscaleMode {
    /// No special assurance. Just upscale the entire contents
    Normal,
    /// Multiple images stacked vertically, ensure all slices are upscaled pixel-perfectly
    VerticalTiles(u32),
    /// Multiple images stacked horizontally, ensure all slices are upscaled pixel-perfectly
    HorizontalTiles(u32),
    /// Grid, ensure all tiles are upscaled pixel-perfectly
    Grid { x: u32, y: u32 },
}

impl UpscaleMode {
    pub const VERTICAL_BUTTON: Self = Self::VerticalTiles(3);
    pub const HORIZONTAL_BUTTON: Self = Self::HorizontalTiles(3);
}

/// Divide 2 integers. Only return the result if it has no remainder.
fn divide_no_remainder(a: u32, b: u32) -> Option<u32> {
    let remainder = a % b;
    if remainder != 0 {
        return None;
    }

    Some(a / b)
}

#[derive(Error, Debug)]
pub enum UpscaleError {
    #[error("input SVG has fractional resolution of {0} x {1}")]
    FractionalInputResolution(f32, f32),
    #[error("scale amount {0} is invalid")]
    InvalidScale(f32),
    #[error("input image of size {w}x{h} cannot be cleanly divided into {tx} by {ty} tiles")]
    NotDivisibleIntoTiles { w: u32, h: u32, tx: u32, ty: u32 },
    #[error("output image of size {0}x{1} cannot be created")]
    InvalidOutputResolution(u32, u32),
}

/// Render a Tree normally at its normal scale
pub fn render(tree: &resvg::usvg::Tree) -> Result<Pixmap, UpscaleError> {
    let (outer_width, outer_height) = {
        let size = tree.size();
        let width = size.width();
        let height = size.height();
        if width.trunc() != width || height.trunc() != height {
            panic!("svg dimensions is fractional ({} x {})", width, height);
        }
        (width as u32, height as u32)
    };

    let mut pixmap = Pixmap::new(outer_width, outer_height).ok_or(
        UpscaleError::InvalidOutputResolution(outer_width, outer_height),
    )?;

    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );

    Ok(pixmap)
}

/// Render a Tree, upscaling it. This allows specifying 'tile size' to ensure that
/// each inner tile is upscaled to an integer resolution, not a fractional resolution.
pub fn render_upscaled(
    tree: &resvg::usvg::Tree,
    scale: f32,
    mode: &UpscaleMode,
    pink_bounds: Option<&Bounds>,
    yellow_bounds: Option<&Bounds>,
) -> Result<Pixmap, UpscaleError> {
    if scale <= 0.0 {
        return Err(UpscaleError::InvalidScale(scale));
    }

    let has_bounds = pink_bounds.is_some() || yellow_bounds.is_some();

    // calculate the target output size, given the upscale mode
    let (outer_width, outer_height) = {
        let size = tree.size();
        let width = size.width();
        let height = size.height();
        if width.trunc() != width || height.trunc() != height {
            return Err(UpscaleError::FractionalInputResolution(width, height));
        }
        (width as u32, height as u32)
    };
    let (inner_width, inner_height) = if has_bounds {
        (outer_width - 2, outer_height - 2)
    } else {
        (outer_width, outer_height)
    };

    let (tiles_x, tiles_y) = match mode {
        UpscaleMode::Normal => (1, 1),
        UpscaleMode::VerticalTiles(y) => (1, *y),
        UpscaleMode::HorizontalTiles(x) => (*x, 1),
        UpscaleMode::Grid { x, y } => (*x, *y),
    };
    let tile_width =
        divide_no_remainder(inner_width, tiles_x).ok_or(UpscaleError::NotDivisibleIntoTiles {
            w: inner_width,
            h: inner_height,
            tx: tiles_x,
            ty: tiles_y,
        })?;
    let tile_height =
        divide_no_remainder(inner_height, tiles_y).ok_or(UpscaleError::NotDivisibleIntoTiles {
            w: inner_width,
            h: inner_height,
            tx: tiles_x,
            ty: tiles_y,
        })?;

    let final_tile_width = ((tile_width as f32) * scale).ceil() as u32;
    let final_tile_height = ((tile_height as f32) * scale).ceil() as u32;
    let final_inner_width = final_tile_width * tiles_x;
    let final_inner_height = final_tile_height * tiles_y;
    let (final_outer_width, final_outer_height) = if has_bounds {
        (final_inner_width + 2, final_inner_height + 2)
    } else {
        (final_inner_width, final_inner_height)
    };

    // render the SVG to the target size
    let mut pixmap = Pixmap::new(final_outer_width, final_outer_height).ok_or(
        UpscaleError::InvalidOutputResolution(final_outer_width, final_outer_height),
    )?;
    let transform = if has_bounds {
        tiny_skia::Transform::from_scale(
            final_inner_width as f32 / inner_width as f32,
            final_inner_height as f32 / inner_height as f32,
        )
        .pre_translate(-1.0, -1.0)
        .post_translate(1.0, 1.0)
    } else {
        tiny_skia::Transform::from_scale(
            final_outer_width as f32 / outer_width as f32,
            final_outer_height as f32 / outer_height as f32,
        )
    };

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // clear existing bounds and redraw them
    if has_bounds {
        // upscale the bounds
        let actual_scale = (final_inner_width as f32 / inner_width as f32)
            .max(final_inner_height as f32 / inner_height as f32);

        let pink_bounds = pink_bounds.unwrap().scale(actual_scale);
        let yellow_bounds = yellow_bounds.unwrap().scale(actual_scale);

        // redraw the bounds
        let pink_paint = {
            let mut paint = tiny_skia::Paint::default();
            paint.anti_alias = false;
            paint.blend_mode = tiny_skia::BlendMode::Source;
            paint.set_color(tiny_skia::Color::from_rgba8(255, 0, 255, 255));
            paint
        };
        let yellow_paint = {
            let mut paint = tiny_skia::Paint::default();
            paint.anti_alias = false;
            paint.blend_mode = tiny_skia::BlendMode::Source;
            paint.set_color(tiny_skia::Color::from_rgba8(255, 255, 0, 255));
            paint
        };

        {
            let mut pixmap_mut = pixmap.as_mut();
            bounds::erase_bounds(&mut pixmap_mut);
            pink_bounds.paint(&mut pixmap_mut, &pink_paint);
            yellow_bounds.paint(&mut pixmap_mut, &yellow_paint);
        }
    }

    Ok(pixmap)
}
