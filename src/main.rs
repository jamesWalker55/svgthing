mod bounds;
mod cli;
mod map_colors;
mod parser;
mod render;

use std::{cell::OnceCell, collections::HashMap, fs, path::PathBuf};

use cli::{Options, RenderTask};
use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    cli::TileSetting,
    map_colors::{get_colors, map_colors},
    render::{render, render_upscaled, UpscaleMode},
};

fn cli_colors(paths: Vec<PathBuf>, print_count: bool) {
    let counts = paths
        .iter()
        .map(|path| {
            // read the input SVG into text
            let path = path.as_path();
            let text = fs::read_to_string(&path)
                .expect(format!("failed to read svg: {}", path.display()).as_str());

            // parse colors in the SVG and map them
            let original_colors = get_colors(&text)
                .expect(format!("failed to parse svg: {}", path.display()).as_str());

            original_colors
        })
        .fold(HashMap::<Color, u32>::new(), |mut acc, colors| {
            for color in colors {
                match acc.get(&color) {
                    Some(count) => acc.insert(color, count + 1),
                    None => acc.insert(color, 1),
                };
            }
            acc
        });

    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by_key(|(_, count)| *count);

    for (color, count) in counts.iter().rev() {
        if print_count {
            println!("{} {}", count, color.to_rgb_string());
        } else {
            println!("{}", color.to_rgb_string());
        }
    }
}

fn cli_render(tasks: Vec<RenderTask>, fonts_dir: Option<PathBuf>, strict: bool) {
    let fontdb = {
        let mut db = resvg::usvg::fontdb::Database::new();
        if let Some(path) = fonts_dir {
            db.load_fonts_dir(path);
        }
        db
    };

    for task in tasks.iter() {
        // read the input SVG into text
        let path = task.input.as_path();
        let mut text = fs::read_to_string(&path)
            .expect(format!("failed to read svg: {}", path.display()).as_str());

        // parse colors in the SVG and map them
        {
            let original_colors = get_colors(&text)
                .expect(format!("failed to parse svg: {}", path.display()).as_str());

            let mut color_map: HashMap<Color, Color> = HashMap::new();

            for cm in &task.color_mappings {
                if !original_colors.contains(&cm.old) {
                    todo!("throw error here!");
                }
                color_map.insert(cm.old.clone(), cm.new.clone());
            }
            text = map_colors(&text, &color_map, strict)
                .expect(format!("failed to map colors: {}", path.display()).as_str());
        }

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
            .expect("failed to parse svg");

        let scale_1_pixmap = render(&tree).unwrap();
        let detected_bounds = OnceCell::new();

        for output in &task.outputs {
            if output.scale == 1.0 {
                // no scaling, just save the image
                scale_1_pixmap.save_png(output.output.as_path()).unwrap();
                continue;
            }

            if output.scale < 1.0 {
                todo!("throw error, not supported");
            }

            let output_path = output.output.as_path();

            let detected_bounds =
                detected_bounds.get_or_init(|| detect_reaper_bounds(&scale_1_pixmap));
            let (yellow_bounds, pink_bounds) = detected_bounds
                .as_ref()
                .map(|(a, b)| (Some(a), Some(b)))
                .unwrap_or((None, None));

            // there are bounds, preprocess then upscale
            let upscale_mode = match &task.tile_setting {
                Some(ts) => match &ts {
                    TileSetting::HorizontalButton => UpscaleMode::HORIZONTAL_BUTTON,
                    TileSetting::VerticalButton => UpscaleMode::VERTICAL_BUTTON,
                    TileSetting::Grid { tx, ty } => UpscaleMode::Grid {
                        x: (*tx).into(),
                        y: (*ty).into(),
                    },
                    TileSetting::HorizontalTiles { tx } => {
                        UpscaleMode::HorizontalTiles((*tx).into())
                    }
                    TileSetting::VerticalTiles { ty } => UpscaleMode::VerticalTiles((*ty).into()),
                },
                None => UpscaleMode::Normal,
            };

            let pixmap = render_upscaled(
                &tree,
                output.scale,
                &upscale_mode,
                pink_bounds,
                yellow_bounds,
            )
            .unwrap();

            pixmap.save_png(&output_path).unwrap();
        }
    }
}

fn main() {
    let opt = cli::options().run();

    match opt {
        Options::Render {
            fonts,
            tasks,
            strict,
        } => cli_render(tasks, fonts, strict),
        Options::Colors { paths, count } => cli_colors(paths, count),
    }
}
