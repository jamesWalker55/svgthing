mod parser;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs, iter,
    path::PathBuf,
};

use parser::Color;

fn get_colors(xml: &str) -> Result<HashSet<Color>, String> {
    let mut result = HashSet::new();
    for part in parser::xml_text(xml.into()).map_err(|x| format!("{}", x))? {
        let parser::TextElement::Color(color) = part else {
            continue;
        };
        result.insert(color);
    }
    Ok(result)
}

fn map_colors(xml: &str, color_map: &HashMap<Color, Color>) -> Result<String, String> {
    let result: String = parser::xml_text(xml.into())
        .map_err(|x| format!("{}", x))?
        .iter()
        .map(|part| match part {
            parser::TextElement::Text(text) => Cow::from(*text),
            parser::TextElement::Color(color) => match color_map.get(color) {
                Some(new_color) => new_color.to_rgb_string().into(),
                None => color.to_rgb_string().into(),
            },
        })
        .collect();
    Ok(result)
}

#[derive(Debug)]
struct Bounds {
    l: u32,
    r: u32,
    t: u32,
    b: u32,
}

impl Bounds {
    fn is_empty(&self) -> bool {
        self.l == 0 && self.r == 0 && self.t == 0 && self.b == 0
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

fn detect_reaper_bounds(img: &resvg::tiny_skia::Pixmap) -> Option<(Bounds, Bounds)> {
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

fn main() {
    let paths = fs::read_dir("svg").expect("failed to list paths in folder ./svg");

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

        // render to image
        {
            let mut fontdb = resvg::usvg::fontdb::Database::new();
            fontdb.load_fonts_dir("fonts");

            let tree =
                resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
                    .expect("failed to parse svg");
            let (width, height) = {
                let size = tree.size();
                (size.width(), size.height())
            };

            const SCALE: u32 = 1;

            let mut pixmap = resvg::tiny_skia::Pixmap::new(
                (width.ceil() as u32) * SCALE,
                (height.ceil() as u32) * SCALE,
            )
            .unwrap();
            pixmap.fill(resvg::tiny_skia::Color::TRANSPARENT);

            // no transformation
            let transform = resvg::tiny_skia::Transform::from_scale(SCALE as f32, SCALE as f32);

            // move everything to top-left by 1px
            // let mut transform = resvg::tiny_skia::Transform::from_scale(SCALE as f32, SCALE as f32);
            // transform = transform.pre_translate(-1.0, -1.0);

            resvg::render(&tree, transform, &mut pixmap.as_mut());

            pixmap.save_png(&output_path).unwrap();

            // check for bounds
            if let Some((yellow_bounds, pink_bounds)) = detect_reaper_bounds(&pixmap) {
                println!("{} {:?} {:?}", path.display(), yellow_bounds, pink_bounds);
            } else {
                println!("{}", path.display());
            };
        }

        // std::io::stdout().flush().unwrap();
    }

    println!("");

    let mut colors_count: Vec<(u32, Color)> =
        colors_count.into_iter().map(|(k, v)| (v, k)).collect();
    colors_count.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    for (count, color) in colors_count {
        println!("{} {}", count, color.to_rgb_string());
    }
}
