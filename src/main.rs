mod bounds;
mod map_colors;
mod parser;

use std::{collections::HashMap, fs, path::PathBuf};

use bounds::Bounds;
use parser::Color;
use resvg::tiny_skia::{self, Pixmap};
use thiserror::Error;

use crate::{
    bounds::detect_reaper_bounds,
    map_colors::{get_colors, map_colors},
};

enum UpscaleMode {
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
    const VERTICAL_BUTTON: Self = Self::VerticalTiles(3);
    const HORIZONTAL_BUTTON: Self = Self::HorizontalTiles(3);
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
enum UpscaleError {
    #[error("input SVG has fractional resolution of {0} x {1}")]
    FractionalInputResolution(f32, f32),
    #[error("scale amount {0} is invalid")]
    InvalidScale(f32),
    #[error("input image of size {w}x{h} cannot be cleanly divided into {tx} by {ty} tiles")]
    NotDivisibleIntoTiles { w: u32, h: u32, tx: u32, ty: u32 },
    #[error("output image of size {0}x{1} cannot be created")]
    InvalidOutputResolution(u32, u32),
}

fn render(tree: &resvg::usvg::Tree) -> Result<Pixmap, UpscaleError> {
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

fn render_upscaled(
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
        let final_outer_width = final_outer_width as f32;
        let final_outer_height = final_outer_height as f32;

        let actual_scale = (final_inner_width as f32 / inner_width as f32)
            .max(final_inner_height as f32 / inner_height as f32);

        let pink_bounds = pink_bounds.unwrap().scale(actual_scale);
        let yellow_bounds = yellow_bounds.unwrap().scale(actual_scale);

        let eraser_paint = {
            let mut paint = tiny_skia::Paint::default();
            paint.anti_alias = false;
            paint.blend_mode = tiny_skia::BlendMode::Clear;
            paint
        };
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, final_outer_width, 1.0).unwrap(),
            &eraser_paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 1.0, final_outer_height).unwrap(),
            &eraser_paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, final_outer_height - 1.0, final_outer_width, 1.0)
                .unwrap(),
            &eraser_paint,
            tiny_skia::Transform::identity(),
            None,
        );
        pixmap.fill_rect(
            tiny_skia::Rect::from_xywh(final_outer_width - 1.0, 0.0, 1.0, final_outer_height)
                .unwrap(),
            &eraser_paint,
            tiny_skia::Transform::identity(),
            None,
        );

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
            pink_bounds.paint(&mut pixmap_mut, &pink_paint);
            yellow_bounds.paint(&mut pixmap_mut, &yellow_paint);
        }
    }

    Ok(pixmap)
}

fn main() {
    let paths = fs::read_dir("svg").expect("failed to list paths in folder ./svg");

    let fontdb = {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_fonts_dir("fonts");
        db
    };

    // render to image
    let mut colors_count: HashMap<Color, u32> = HashMap::new();

    for entry in paths {
        let path = entry.unwrap().path();
        if !path
            .extension()
            .is_some_and(|ext| ext.to_ascii_lowercase() == "svg")
        {
            continue;
        }
        let output_path = PathBuf::from("temp")
            .join(path.file_name().expect("file has no filename"))
            .with_extension("png");

        let text = fs::read_to_string(&path)
            .expect(format!("failed to read svg: {}", path.display()).as_str());

        // // parse colors and map them
        // {
        //     let colors = get_colors(&text)
        //         .expect(format!("failed to parse svg: {}", path.display()).as_str());
        //     let mut new_colors: HashMap<Color, Color> = HashMap::with_capacity(colors.len());
        //     for c in colors {
        //         new_colors.insert(c.clone(), Color(255, 0, 0));
        //         match colors_count.get(&c) {
        //             Some(count) => colors_count.insert(c, count + 1),
        //             None => colors_count.insert(c, 1),
        //         };
        //     }
        //     text = map_colors(&text, &new_colors)
        //         .expect(format!("failed to map colors: {}", path.display()).as_str());
        // }

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
            .expect("failed to parse svg");

        let pixmap_1 = render(&tree).unwrap();

        const SCALE: f32 = 1.5;

        if let Some((yellow_bounds, pink_bounds)) = detect_reaper_bounds(&pixmap_1) {
            // there are bounds, preprocess then upscale
            let upscale_mode = if path
                .file_stem()
                .is_some_and(|name| name.to_string_lossy() == "mcp_fxparm_empty")
            {
                UpscaleMode::VERTICAL_BUTTON
            } else {
                UpscaleMode::Normal
            };

            let pixmap = render_upscaled(
                &tree,
                SCALE,
                &upscale_mode,
                Some(&pink_bounds),
                Some(&yellow_bounds),
            )
            .unwrap();

            println!("{}", path.display());
            pixmap.save_png(&output_path).unwrap();
        } else {
            // no bounds, just upscale
            // println!("{}", path.display());
        };
    }

    println!("");

    let mut colors_count: Vec<(u32, Color)> =
        colors_count.into_iter().map(|(k, v)| (v, k)).collect();
    colors_count.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    for (count, color) in colors_count {
        println!("{} {}", count, color.to_rgb_string());
    }
}
