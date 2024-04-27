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

pub fn map_colors(
    xml: &str,
    color_map: &HashMap<Color, Color>,
    strict: bool,
) -> Result<String, String> {
    let mut unused_colors: HashSet<Color> = color_map.keys().cloned().collect();
    let result: Result<String, String> = parser::xml_text(xml.into())
        .map_err(|x| format!("{}", x))?
        .iter()
        .map(|part| match part {
            parser::TextElement::Text(text) => Ok(Cow::from(*text)),
            parser::TextElement::Color(color) => {
                if color.is_reaper_reserved() {
                    return Ok(color.to_rgb_string().into());
                }

                match color_map.get(color) {
                    Some(new_color) => {
                        unused_colors.remove(new_color);
                        Ok(new_color.to_rgb_string().into())
                    }
                    None => {
                        if strict {
                            Err(format!(
                                "failed to map colors {:?} - colors not found in svg",
                                unused_colors
                            ))
                        } else {
                            Ok(color.to_rgb_string().into())
                        }
                    }
                }
            }
        })
        .collect();
    if unused_colors.len() != 0 {
        return Err(format!(
            "failed to map colors {:?} - colors not found in svg",
            unused_colors
        ));
    }
    result
}
