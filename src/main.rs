mod parser;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
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

fn main() {
    let mut text =
        fs::read_to_string("svg/gen_env_write.svg").expect("failed to read svg into text");

    // parse colors and map them
    {
        let colors = get_colors(&text).expect("failed to parse xml");
        dbg!(&colors);
        let mut new_colors: HashMap<Color, Color> = HashMap::with_capacity(colors.len());
        for c in colors {
            new_colors.insert(c, Color(255, 0, 0));
        }
        text = map_colors(&text, &new_colors).expect("failed to map colors");
    }

    // render to image
    {
        let mut fontdb = resvg::usvg::fontdb::Database::new();
        fontdb.load_fonts_dir("fonts");

        let tree = resvg::usvg::Tree::from_str(&text, &resvg::usvg::Options::default(), &fontdb)
            .expect("failed to parse svg");
        let (width, height) = {
            let size = tree.size();
            (size.width(), size.height())
        };
        dbg!(tree.view_box());

        const SCALE: u32 = 3;

        let mut pixmap = resvg::tiny_skia::Pixmap::new(
            (width.ceil() as u32) * SCALE,
            (height.ceil() as u32) * SCALE,
        )
        .unwrap();
        pixmap.fill(resvg::tiny_skia::Color::TRANSPARENT);

        let transform = resvg::tiny_skia::Transform::from_scale(SCALE as f32, SCALE as f32);

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        pixmap.save_png("temp.png").unwrap();
    }
}
