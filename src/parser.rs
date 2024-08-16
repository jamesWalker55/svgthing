use std::str::FromStr;

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{char, none_of, one_of, space0, u8},
    combinator::{all_consuming, cut, eof, not, opt, peek, recognize},
    multi::{many0, many1},
    number::complete::float,
    sequence::{delimited, preceded, tuple},
    Finish, IResult, Parser,
};

type Input = str;

type Result<'a, O = &'a Input> = IResult<&'a Input, O>;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub enum Color {
    RGB(u8, u8, u8),
    RGBA(u8, u8, u8, u8),
}

impl ToString for Color {
    fn to_string(&self) -> String {
        match self {
            Color::RGB(r, g, b) => format!("rgb({}, {}, {})", r, g, b),
            Color::RGBA(r, g, b, a) => format!(
                "rgb({}, {}, {});fill-opacity:{}",
                r,
                g,
                b,
                *a as f32 / 255.0
            ),
        }
    }
}

impl Color {
    pub fn r(&self) -> u8 {
        match self {
            Color::RGB(r, g, b) => *r,
            Color::RGBA(r, g, b, a) => *r,
        }
    }

    pub fn g(&self) -> u8 {
        match self {
            Color::RGB(r, g, b) => *g,
            Color::RGBA(r, g, b, a) => *g,
        }
    }

    pub fn b(&self) -> u8 {
        match self {
            Color::RGB(r, g, b) => *b,
            Color::RGBA(r, g, b, a) => *b,
        }
    }

    pub fn a(&self) -> Option<u8> {
        match self {
            Color::RGB(r, g, b) => None,
            Color::RGBA(r, g, b, a) => Some(*a),
        }
    }

    pub fn with_a(&self, a: u8) -> Self {
        match self {
            Color::RGB(r, g, b) => Self::RGBA(*r, *g, *b, a),
            Color::RGBA(r, g, b, a) => Self::RGBA(*r, *g, *b, *a),
        }
    }

    pub fn with_opacity(&self, opacity: f32) -> Self {
        self.with_a((255.0 * opacity).round() as u8)
    }

    pub fn is_reaper_reserved(&self) -> bool {
        if self.a().unwrap_or(255) != 255 {
            return false;
        }

        let r = self.r();
        let g = self.g();
        let b = self.b();
        let is_yellow = r == 255 && g == 255 && b == 0;
        let is_pink = r == 255 && g == 0 && b == 255;
        is_yellow || is_pink
    }
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        all_consuming(color)(&s)
            .finish()
            .map(|(_, o)| o)
            .map_err(|x| x.to_string())
    }
}

fn color_rgb_value(input: &Input) -> Result<u8> {
    delimited(space0, u8, space0)(input)
}

fn rgb_numeric(input: &Input) -> Result<Color> {
    delimited(
        tag("rgb("),
        cut(tuple((
            color_rgb_value,
            char(','),
            color_rgb_value,
            char(','),
            color_rgb_value,
        ))),
        cut(char(')')),
    )
    .map(|(r, _, g, _, b)| Color::RGB(r, g, b))
    .parse(input)
}

fn rgba_numeric(input: &Input) -> Result<Color> {
    delimited(
        tag("rgba("),
        cut(tuple((
            color_rgb_value,
            char(','),
            color_rgb_value,
            char(','),
            color_rgb_value,
            char(','),
            color_rgb_value,
        ))),
        cut(char(')')),
    )
    .map(|(r, _, g, _, b, _, a)| Color::RGBA(r, g, b, a))
    .parse(input)
}

fn color_numeric(input: &Input) -> Result<Color> {
    alt((rgb_numeric, rgba_numeric))(input)
}

fn color_hex(input: &Input) -> Result<Color> {
    delimited(
        alt((tag("#"), tag("0x"))),
        tuple((
            recognize(tuple((
                one_of("0123456789abcdefABCDEF"),
                one_of("0123456789abcdefABCDEF"),
            ))),
            recognize(tuple((
                one_of("0123456789abcdefABCDEF"),
                one_of("0123456789abcdefABCDEF"),
            ))),
            recognize(tuple((
                one_of("0123456789abcdefABCDEF"),
                one_of("0123456789abcdefABCDEF"),
            ))),
            opt(recognize(tuple((
                one_of("0123456789abcdefABCDEF"),
                one_of("0123456789abcdefABCDEF"),
            )))),
        )),
        peek(alt((
            eof.map(|_| ()),
            none_of("0123456789abcdefABCDEF").map(|_| ()),
        ))),
    )
    .map(|(r, g, b, a)| {
        let r =
            u8::from_str_radix(r, 16).expect(format!("failed to convert {r} to number").as_str());
        let g =
            u8::from_str_radix(g, 16).expect(format!("failed to convert {g} to number").as_str());
        let b =
            u8::from_str_radix(b, 16).expect(format!("failed to convert {b} to number").as_str());
        match a {
            Some(a) => {
                let a = u8::from_str_radix(a, 16)
                    .expect(format!("failed to convert {a} to number").as_str());
                Color::RGBA(r, g, b, a)
            }
            None => Color::RGB(r, g, b),
        }
    })
    .parse(input)
}

fn rgb_hex_short(input: &Input) -> Result<Color> {
    delimited(
        alt((tag("#"), tag("0x"))),
        tuple((
            recognize(one_of("0123456789abcdefABCDEF")),
            recognize(one_of("0123456789abcdefABCDEF")),
            recognize(one_of("0123456789abcdefABCDEF")),
        )),
        peek(alt((
            eof.map(|_| ()),
            none_of("0123456789abcdefABCDEF").map(|_| ()),
        ))),
    )
    .map(|(r, g, b)| {
        let r = u8::from_str_radix(r, 16)
            .expect(format!("failed to convert {r} to number").as_str())
            * 0x11;
        let g = u8::from_str_radix(g, 16)
            .expect(format!("failed to convert {g} to number").as_str())
            * 0x11;
        let b = u8::from_str_radix(b, 16)
            .expect(format!("failed to convert {b} to number").as_str())
            * 0x11;
        Color::RGB(r, g, b)
    })
    .parse(input)
}

fn color(input: &Input) -> Result<Color> {
    alt((color_hex, rgb_hex_short, color_numeric))(input)
}

#[derive(PartialEq, Debug)]
pub enum TextElement<'a> {
    Text(&'a Input),
    Color(Color),
}

fn fill_opacity(input: &Input) -> Result<f32> {
    preceded(tag(";fill-opacity:"), float)(input)
}

fn color_with_fill_opacity(input: &Input) -> Result<Color> {
    color(input).map(|(input, color)| match fill_opacity(input) {
        Ok((input, opacity)) => (input, color.with_opacity(opacity)),
        Err(_) => (input, color),
    })
}

fn non_color_text(input: &Input) -> Result {
    recognize(many1(preceded(not(color_with_fill_opacity), take(1usize))))(input)
}

fn text_with_colors(input: &Input) -> Result<Vec<TextElement>> {
    many0(alt((
        color_with_fill_opacity.map(|x| TextElement::Color(x)),
        non_color_text.map(|x| TextElement::Text(x)),
    )))
    .parse(input)
}

pub fn xml_text(input: &Input) -> std::result::Result<Vec<TextElement>, nom::error::Error<&Input>> {
    all_consuming(text_with_colors)(input)
        .finish()
        .map(|(_rest, vec)| vec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_numeric() {
        // spacing
        assert_eq!(rgb_numeric("rgb(1,2,3)").unwrap().1, Color::RGB(1, 2, 3));
        assert_eq!(rgb_numeric("rgb(1, 2, 3)").unwrap().1, Color::RGB(1, 2, 3));
        assert_eq!(
            rgb_numeric("rgb(  1  ,  2  ,  3  )").unwrap().1,
            Color::RGB(1, 2, 3)
        );
        assert!(rgb_numeric("rgb(  1  ,  2  ,  3  ,)").is_err());
        assert!(rgb_numeric("rgb (1, 2, 3)").is_err());

        // valid numbers
        assert_eq!(
            rgb_numeric("rgb(0, 100, 255)").unwrap().1,
            Color::RGB(0, 100, 255)
        );
        assert!(rgb_numeric("rgb(0, 256, 0)").is_err());
        assert!(rgb_numeric("rgb(-1, 0, 0)").is_err());
    }

    #[test]
    fn test_color_hex() {
        assert_eq!(color_hex("#000000").unwrap().1, Color::RGB(0, 0, 0));
        assert_eq!(
            color_hex("#112233").unwrap().1,
            Color::RGB(0x11, 0x22, 0x33)
        );
        assert_eq!(color_hex("0x000000").unwrap().1, Color::RGB(0, 0, 0));
        assert_eq!(
            color_hex("0x112233").unwrap().1,
            Color::RGB(0x11, 0x22, 0x33)
        );
    }

    #[test]
    fn test_color_hex_short() {
        assert_eq!(rgb_hex_short("#000").unwrap().1, Color::RGB(0, 0, 0));
        assert_eq!(
            rgb_hex_short("#123").unwrap().1,
            Color::RGB(0x11, 0x22, 0x33)
        );
        assert_eq!(rgb_hex_short("0x000").unwrap().1, Color::RGB(0, 0, 0));
        assert_eq!(
            rgb_hex_short("0x123").unwrap().1,
            Color::RGB(0x11, 0x22, 0x33)
        );
    }

    #[test]
    fn test_text_no_color() {
        assert_eq!(non_color_text("apple #000000").unwrap().1, "apple ");
        assert_eq!(non_color_text("apple 0x000000").unwrap().1, "apple ");
        assert_eq!(non_color_text("apple rgb(1,2,3)").unwrap().1, "apple ");
        assert!(non_color_text("rgb(1, 2, 3)").is_err());
    }

    #[test]
    fn test_text() {
        assert_eq!(
            text_with_colors("apple #000000").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color::RGB(0, 0, 0))
            ]
        );
        assert_eq!(
            text_with_colors("apple 0x000000").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color::RGB(0, 0, 0))
            ]
        );
        assert_eq!(
            text_with_colors("apple rgb(1,2,3) apple").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color::RGB(1, 2, 3)),
                TextElement::Text(" apple")
            ]
        );
    }
}
