use std::{num::NonZeroU32, path::PathBuf};

use crate::parser::Color;
use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub enum Options {
    /// Scan SVG files for colors and list them
    #[bpaf(command)]
    Colors {
        count: bool,
        /// Parse alpha values when parsing the SVG
        include_alpha: bool,
        #[bpaf(positional("PATH"))]
        paths: Vec<PathBuf>,
    },
    /// Render SVG files and upscale them, while preserving REAPER's pink/yellow borders
    #[bpaf(command)]
    Render {
        fonts: Option<PathBuf>,
        /// Assert that all input colors are used in the SVG
        all_input_colors: bool,
        /// Assert that all SVG colors appear in the input colors
        all_svg_colors: bool,
        /// Parse alpha values when parsing the SVG
        include_alpha: bool,
        #[bpaf(external(render_task), some("at least one task must be specified"))]
        tasks: Vec<RenderTask>,
    },
    /// Render a single task, but pass your SVG through stdin
    #[bpaf(command)]
    RenderStdin {
        fonts: Option<PathBuf>,
        /// Assert that all input colors are used in the SVG
        all_input_colors: bool,
        /// Assert that all SVG colors appear in the input colors
        all_svg_colors: bool,
        /// Parse alpha values when parsing the SVG
        include_alpha: bool,
        #[bpaf(external(stdin_render_task))]
        task: StdinRenderTask,
    },
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct RenderTasks {
    #[bpaf(external(render_task), some("at least one task must be specified"))]
    pub tasks: Vec<RenderTask>,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
pub struct RenderTask {
    /// Input path of the SVG to be rendered
    #[bpaf(short, long, argument("SVG"))]
    pub input: PathBuf,
    /// Replace colors in the input SVG with new colors
    #[bpaf(external(color_mapping), many)]
    pub color_mappings: Vec<ColorMapping>,
    #[bpaf(external(tile_setting), optional, group_help("Tiled upscaling"))]
    pub tile_setting: Option<TileSetting>,
    /// The output PNBs to render
    #[bpaf(external(output), some("at least one output must be specified"))]
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
pub struct StdinRenderTask {
    /// Replace colors in the input SVG with new colors
    #[bpaf(external(color_mapping), many)]
    pub color_mappings: Vec<ColorMapping>,
    #[bpaf(external(tile_setting), optional, group_help("Tiled upscaling"))]
    pub tile_setting: Option<TileSetting>,
    /// The output PNBs to render
    #[bpaf(external(output), some("at least one output must be specified"))]
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
pub struct ColorMapping {
    /// Map a color to a new color
    #[bpaf(short, long)]
    pub map: (),
    /// Color to map from. If this color isn't found in the SVG, this will raise an error
    #[bpaf(positional("FROM_COLOR"))]
    pub old: Color,
    /// The new color to use
    #[bpaf(positional("TO_COLOR"))]
    pub new: Color,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
pub struct Output {
    /// Output path to save the rendered image (should be PNG format)
    #[bpaf(short, long, argument("OUTPUT"))]
    pub output: PathBuf,
    /// Scale to render the image
    #[bpaf(short, long, fallback(1.0), argument("SCALE"))]
    pub scale: f32,
}

#[derive(Debug, Clone, Bpaf)]
pub enum TileSetting {
    /// Image contains 3 equal-sized tiles placed horizontally, i.e. a horizontally-sliced button.
    #[bpaf(long("hb"))]
    HorizontalButton,
    /// Image contains 3 equal-sized tiles placed vertically, i.e. a vertically-sliced button.
    #[bpaf(long("vb"))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_cli() {
        options().check_invariants(false);
    }
}
