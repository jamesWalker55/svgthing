use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use crate::parser::{self, Color};

pub fn get_colors(xml: &str) -> Result<HashSet<Color>, String> {
    let mut result = HashSet::new();
    for part in parser::xml_text(xml.into()).map_err(|x| format!("{}", x))? {
        let parser::TextElement::Color(color) = part else {
            continue;
        };
        result.insert(color);
    }
    Ok(result)
}

pub fn map_colors(xml: &str, color_map: &HashMap<Color, Color>) -> Result<String, String> {
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
