mod bounds;
mod map_colors;
mod parser;
mod render;

use std::{collections::HashMap, fs, num::NonZeroU32, path::PathBuf};

use bpaf::Bpaf;
use parser::Color;

use crate::{
    bounds::detect_reaper_bounds,
    render::{render, render_upscaled, UpscaleMode},
};

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    #[bpaf(external(task), some("at least one task must be specified"))]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
struct Task {
    /// Scale to render the image
    #[bpaf(short, long, fallback(1.0), argument("SCALE"))]
    scale: f32,
    #[bpaf(external(tile_setting), optional, group_help("Tiled upscaling"))]
    tile_setting: Option<TileSetting>,
    /// Output path to save the rendered image (should be PNG format)
    #[bpaf(short, long, argument("OUTPUT"))]
    output: PathBuf,
    /// Input path of the SVG to be rendered
    #[bpaf(positional("INPUT"))]
    input: PathBuf,
}

#[derive(Debug, Clone, Bpaf)]
enum TileSetting {
    /// Image contains 3 equal-sized tiles placed horizontally, i.e. a horizontally-sliced button.
    #[bpaf(short('h'), long("hb"))]
    HorizontalButton,
    /// Image contains 3 equal-sized tiles placed vertically, i.e. a vertically-sliced button.
    #[bpaf(short('v'), long("vb"))]
    VerticalButton,
    Grid {
        /// Divide the image into arbitrary number of tiles horizontally
        #[bpaf(short('x'), long("tx"))]
        tx: NonZeroU32,
        /// Divide the image into arbitrary number of tiles vertically
        #[bpaf(short('y'), long("ty"))]
        ty: NonZeroU32,
    },
    HorizontalTiles {
        /// Divide the image into arbitrary number of tiles horizontally
        #[bpaf(short('x'), long("tx"))]
        tx: NonZeroU32,
    },
    VerticalTiles {
        /// Divide the image into arbitrary number of tiles vertically
        #[bpaf(short('y'), long("ty"))]
        ty: NonZeroU32,
    },
}

fn main() {
    let opt = options().run();

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
