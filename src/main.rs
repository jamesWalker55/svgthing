mod bounds;
mod cli;
mod map_colors;
mod parser;
mod render;

use std::{cell::OnceCell, collections::HashMap, fs, path::PathBuf};

use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    cli::TileSetting,
    map_colors::{get_colors, map_colors},
    render::{render, render_upscaled, UpscaleMode},
};

fn main() {
    let opt = cli::options().run();

    let fontdb = {
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_fonts_dir("fonts");
        db
    };

    for task in opt.tasks {
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
            text = map_colors(&text, &color_map)
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
