mod parser;

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
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

        let mut text = fs::read_to_string(&path)
            .expect(format!("failed to read svg: {}", path.display()).as_str());

        // parse colors and map them
        {
            let colors = get_colors(&text)
                .expect(format!("failed to parse svg: {}", path.display()).as_str());
            let mut new_colors: HashMap<Color, Color> = HashMap::with_capacity(colors.len());
            for c in colors {
                new_colors.insert(c.clone(), Color(255, 0, 0));
                match colors_count.get(&c) {
                    Some(count) => colors_count.insert(c, count + 1),
                    None => colors_count.insert(c, 1),
                };
            }
            text = map_colors(&text, &new_colors)
                .expect(format!("failed to map colors: {}", path.display()).as_str());
        }

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

            const SCALE: u32 = 3;

            let mut pixmap = resvg::tiny_skia::Pixmap::new(
                (width.ceil() as u32) * SCALE,
                (height.ceil() as u32) * SCALE,
            )
            .unwrap();
            pixmap.fill(resvg::tiny_skia::Color::TRANSPARENT);

            let transform = resvg::tiny_skia::Transform::from_scale(SCALE as f32, SCALE as f32);

            resvg::render(&tree, transform, &mut pixmap.as_mut());

            pixmap.save_png(&output_path).unwrap();
        }
    }

    let mut colors_count: Vec<(u32, Color)> =
        colors_count.into_iter().map(|(k, v)| (v, k)).collect();
    colors_count.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    for (count, color) in colors_count {
        println!("{} {}", count, color.to_rgb_string());
    }
}
