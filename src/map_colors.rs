use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use crate::{
    parser::{self, Color},
    RenderOptions,
};

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
    opt: &RenderOptions,
) -> Result<String, String> {
    let mut unused_colors: HashSet<Color> = color_map.keys().cloned().collect();
    let result: Result<String, String> = parser::xml_text(xml.into())
        .map_err(|x| format!("{}", x))?
        .iter()
        .map(|part| match part {
            parser::TextElement::Text(text) => Ok(Cow::from(*text)),
            parser::TextElement::Color(old_color) => {
                if old_color.is_reaper_reserved() {
                    return Ok(old_color.to_string().into());
                }

                match color_map.get(old_color) {
                    Some(new_color) => {
                        unused_colors.remove(old_color);
                        Ok(new_color.to_string().into())
                    }
                    None => {
                        if opt.all_svg_colors {
                            Err(format!(
                                "failed to map colors {:?} - svg color not in map",
                                old_color
                            ))
                        } else {
                            Ok(old_color.to_string().into())
                        }
                    }
                }
            }
        })
        .collect();
    let result = result?;
    if opt.all_input_colors && unused_colors.len() != 0 {
        return Err(format!(
            "failed to map colors {:?} - colors not found in svg",
            unused_colors
        ));
    }
    Ok(result)
}
