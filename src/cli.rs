use std::{num::NonZeroU32, path::PathBuf};

use crate::parser::Color;
use bpaf::Bpaf;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    #[bpaf(external(task), some("at least one task must be specified"))]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(adjacent)]
pub struct Task {
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
