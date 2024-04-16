mod bounds;
mod cli;
mod map_colors;
mod parser;
mod render;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    render::{render, render_upscaled, UpscaleMode},
};

fn main() {
    cli::options().check_invariants(false);

    let opt = cli::options().run();

    let mut input_paths_count: HashMap<&Path, u32> = HashMap::new();

    // group the tasks by their input paths
    for mut task in opt.tasks {
        // canonicalize paths to ensure paths that point to the same place (relative or absolute) will end up as the same PathBuf
        task.input = match task.input.canonicalize() {
            Ok(x) => {
                if !x.is_file() {
                    println!("Input file `{}` is not a file", task.input.display(),);
                    return;
                }
                x
            }
            Err(err) => {
                println!(
                    "Failed to locate input file `{}`: {}",
                    task.input.display(),
                    err
                );
                return;
            }
        };

        let input_path = task.input.as_path();
        match input_paths_count.get(&input_path) {
            Some(count) => input_paths_count.insert(&input_path, count + 1),
            None => input_paths_count.insert(&input_path, 1),
        };
    }

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
            let upscale_mode = if path.file_stem().is_some_and(|name| {
                let name = name.to_string_lossy();
                name == "mcp_fxparm_empty" || name == "mcp_sendlist_empty"
            }) {
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
