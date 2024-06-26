use std::str::FromStr;

use nom::{
    branch::alt,
    bytes::complete::{tag, take},
    character::complete::{char, none_of, one_of, space0, u8},
    combinator::{all_consuming, cut, eof, not, recognize},
    multi::{many0, many1},
    sequence::{delimited, preceded, tuple},
    Finish, IResult, Parser,
};

type Input = str;

type Result<'a, O = &'a Input> = IResult<&'a Input, O>;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub fn to_rgb_string(&self) -> String {
        format!("rgb({}, {}, {})", self.0, self.1, self.2)
    }

    pub fn is_reaper_reserved(&self) -> bool {
        let is_yellow = self.0 == 255 && self.1 == 255 && self.2 == 0;
        let is_pink = self.0 == 255 && self.1 == 0 && self.2 == 255;
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

fn color_numeric(input: &Input) -> Result<Color> {
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
    .map(|(r, _, g, _, b)| Color(r, g, b))
    .parse(input)
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
        )),
        alt((
            eof.map(|_| ()),
            none_of("0123456789abcdefABCDEF").map(|_| ()),
        )),
    )
    .map(|(r, g, b)| {
        let r =
            u8::from_str_radix(r, 16).expect(format!("failed to convert {r} to number").as_str());
        let g =
            u8::from_str_radix(g, 16).expect(format!("failed to convert {g} to number").as_str());
        let b =
            u8::from_str_radix(b, 16).expect(format!("failed to convert {b} to number").as_str());
        Color(r, g, b)
    })
    .parse(input)
}

fn color_hex_short(input: &Input) -> Result<Color> {
    delimited(
        alt((tag("#"), tag("0x"))),
        tuple((
            recognize(one_of("0123456789abcdefABCDEF")),
            recognize(one_of("0123456789abcdefABCDEF")),
            recognize(one_of("0123456789abcdefABCDEF")),
        )),
        alt((
            eof.map(|_| ()),
            none_of("0123456789abcdefABCDEF").map(|_| ()),
        )),
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
        Color(r, g, b)
    })
    .parse(input)
}

#[derive(PartialEq, Eq, Debug)]
pub enum TextElement<'a> {
    Text(&'a Input),
    Color(Color),
}

fn color(input: &Input) -> Result<Color> {
    alt((color_hex, color_hex_short, color_numeric))(input)
}

fn non_color_text(input: &Input) -> Result {
    recognize(many1(preceded(not(color), take(1usize))))(input)
}

fn text_with_colors(input: &Input) -> Result<Vec<TextElement>> {
    many0(alt((
        color.map(|x| TextElement::Color(x)),
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
        assert_eq!(color_numeric("rgb(1,2,3)").unwrap().1, Color(1, 2, 3));
        assert_eq!(color_numeric("rgb(1, 2, 3)").unwrap().1, Color(1, 2, 3));
        assert_eq!(
            color_numeric("rgb(  1  ,  2  ,  3  )").unwrap().1,
            Color(1, 2, 3)
        );
        assert!(color_numeric("rgb(  1  ,  2  ,  3  ,)").is_err());
        assert!(color_numeric("rgb (1, 2, 3)").is_err());

        // valid numbers
        assert_eq!(
            color_numeric("rgb(0, 100, 255)").unwrap().1,
            Color(0, 100, 255)
        );
        assert!(color_numeric("rgb(0, 256, 0)").is_err());
        assert!(color_numeric("rgb(-1, 0, 0)").is_err());
    }

    #[test]
    fn test_color_hex() {
        assert_eq!(color_hex("#000000").unwrap().1, Color(0, 0, 0));
        assert_eq!(color_hex("#112233").unwrap().1, Color(0x11, 0x22, 0x33));
        assert_eq!(color_hex("0x000000").unwrap().1, Color(0, 0, 0));
        assert_eq!(color_hex("0x112233").unwrap().1, Color(0x11, 0x22, 0x33));
    }

    #[test]
    fn test_color_hex_short() {
        assert_eq!(color_hex_short("#000").unwrap().1, Color(0, 0, 0));
        assert_eq!(color_hex_short("#123").unwrap().1, Color(0x11, 0x22, 0x33));
        assert_eq!(color_hex_short("0x000").unwrap().1, Color(0, 0, 0));
        assert_eq!(color_hex_short("0x123").unwrap().1, Color(0x11, 0x22, 0x33));
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
                TextElement::Color(Color(0, 0, 0))
            ]
        );
        assert_eq!(
            text_with_colors("apple 0x000000").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color(0, 0, 0))
            ]
        );
        assert_eq!(
            text_with_colors("apple rgb(1,2,3) apple").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color(1, 2, 3)),
                TextElement::Text(" apple")
            ]
        );
    }
}
