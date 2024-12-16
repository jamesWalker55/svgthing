mod bounds;
mod cli;
mod map_colors;
mod parser;
mod render;

use std::{
    cell::OnceCell,
    collections::HashMap,
    fs,
    io::{self, Read},
    path::PathBuf,
};

use cli::{Options, RenderTask};
use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    cli::TileSetting,
    map_colors::{get_colors, map_colors},
    render::{render, render_upscaled, UpscaleMode},
};

fn cli_colors(paths: Vec<PathBuf>, print_count: bool, include_alpha: bool) {
    let counts = paths
        .iter()
        .map(|path| {
            // read the input SVG into text
            let path = path.as_path();
            let text = fs::read_to_string(&path)
                .expect(format!("failed to read svg: {}", path.display()).as_str());

            // parse colors in the SVG and map them
            let original_colors = get_colors(&text, include_alpha)
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
            println!("{} {}", count, color.to_string());
        } else {
            println!("{}", color.to_string());
        }
    }
}

pub(crate) struct RenderOptions {
    pub(crate) all_input_colors: bool,
    pub(crate) all_svg_colors: bool,
    pub(crate) include_alpha: bool,
}

fn cli_render(tasks: Vec<RenderTask>, fonts_dir: Option<PathBuf>, opt: &RenderOptions) {
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
            let mut color_map: HashMap<Color, Color> = HashMap::new();

            for cm in &task.color_mappings {
                color_map.insert(cm.old.clone(), cm.new.clone());
            }

            text = match map_colors(&text, &color_map, opt) {
                Ok(x) => x,
                Err(err) => {
                    println!("failed to map colors: {}: {}", path.display(), err);
                    continue;
                }
            }
        }

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
            .or_else(|x| {
                fs::write("error.svg", text).unwrap();
                Err(x)
            })
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
            all_input_colors,
            all_svg_colors,
            include_alpha,
        } => cli_render(
            tasks,
            fonts,
            &RenderOptions {
                all_input_colors,
                all_svg_colors,
                include_alpha,
            },
        ),
        Options::RenderStdin {
            fonts,
            all_input_colors,
            all_svg_colors,
            include_alpha,
        } => {
            let input: String = {
                let stdin = io::stdin();
                let mut buf = Vec::new();
                stdin
                    .lock()
                    .read_to_end(&mut buf)
                    .expect("failed to read stdin");
                String::from_utf8(buf).expect("input is invalid utf8")
            };
            let input_split = shell_words::split(input.as_str())
                .expect("failed to parse stdin as UNIX arguments");
            let tasks = cli::render_tasks()
                .run_inner(input_split.as_slice())
                .expect("failed to parse stdin as render tasks");

            cli_render(
                tasks.tasks,
                fonts,
                &RenderOptions {
                    all_input_colors,
                    all_svg_colors,
                    include_alpha,
                },
            );
        }
        Options::Colors {
            paths,
            count,
            include_alpha,
        } => cli_colors(paths, count, include_alpha),
    }
}
