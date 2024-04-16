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
