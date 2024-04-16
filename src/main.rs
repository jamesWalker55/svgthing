mod bounds;
mod map_colors;
mod parser;

use std::{collections::HashMap, fs, path::PathBuf};

use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    map_colors::{get_colors, map_colors},
};

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
        let mut fontdb = resvg::usvg::fontdb::Database::new();
        fontdb.load_fonts_dir("fonts");

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
            .expect("failed to parse svg");
        let (outer_width, outer_height) = {
            let size = tree.size();
            let width = size.width();
            let height = size.height();
            if width.trunc() != width || height.trunc() != height {
                panic!(
                    "svg dimensions is fractional ({}, {}): {}",
                    width,
                    height,
                    path.display()
                );
            }
            (width as u32, height as u32)
        };

        // render normally first at scale 1
        let pixmap_1 = {
            let mut pixmap = resvg::tiny_skia::Pixmap::new(outer_width, outer_height).unwrap();

            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut pixmap.as_mut(),
            );

            pixmap
        };

        const SCALE: f64 = 1.5;

        if let Some((yellow_bounds, pink_bounds)) = detect_reaper_bounds(&pixmap_1) {
            // there are bounds, preprocess then upscale
            let inner_width = outer_width - 2;
            let inner_height = outer_height - 2;
            let final_inner_width = ((inner_width as f64) * SCALE).ceil() as u32;
            let final_inner_height = ((inner_height as f64) * SCALE).ceil() as u32;
            let final_outer_width = final_inner_width + 2;
            let final_outer_height = final_inner_height + 2;

            let mut pixmap =
                resvg::tiny_skia::Pixmap::new(final_outer_width, final_outer_height).unwrap();
            let transform = {
                let mut t = resvg::tiny_skia::Transform::identity();
                t = t.post_translate(-1.0, -1.0);
                t = t.post_scale(
                    final_inner_width as f32 / inner_width as f32,
                    final_inner_height as f32 / inner_height as f32,
                );
                t = t.post_translate(1.0, 1.0);
                t
            };

            resvg::render(&tree, transform, &mut pixmap.as_mut());

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
