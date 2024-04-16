use nom::{
    branch::alt,
    bytes::complete::{tag, take, take_while},
    character::complete::{char, one_of, space0, u8},
    combinator::{cut, not, opt, recognize},
    multi::{count, many0, many1},
    sequence::{delimited, preceded, tuple},
    IResult, Parser,
};

type Input = str;

type Result<'a, O = &'a Input> = IResult<&'a Input, O>;

#[derive(PartialEq, Eq, Debug)]
enum Color {
    RGB(u8, u8, u8),
    RGBA(u8, u8, u8, u8),
}

fn color_rgb_value(input: &Input) -> Result<u8> {
    delimited(space0, u8, space0)(input)
}

fn color_numeric(input: &Input) -> Result<Color> {
    alt((
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
        .map(|(r, _, g, _, b, _, a)| Color::RGBA(r, g, b, a)),
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
        .map(|(r, _, g, _, b)| Color::RGB(r, g, b)),
    ))(input)
}

fn color_hex(input: &Input) -> Result<Color> {
    preceded(
        char('#'),
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
    )
    .map(|(r, g, b, a)| {
        let r =
            u8::from_str_radix(r, 16).expect(format!("failed to convert {r} to number").as_str());
        let g =
            u8::from_str_radix(g, 16).expect(format!("failed to convert {g} to number").as_str());
        let b =
            u8::from_str_radix(b, 16).expect(format!("failed to convert {b} to number").as_str());
        let a = a.map(|x| {
            u8::from_str_radix(x, 16).expect(format!("failed to convert {b} to number").as_str())
        });
        if let Some(a) = a {
            Color::RGBA(r, g, b, a)
        } else {
            Color::RGB(r, g, b)
        }
    })
    .parse(input)
}

#[derive(PartialEq, Eq, Debug)]
enum TextElement<'a> {
    Text(&'a Input),
    Color(Color),
}

fn color(input: &Input) -> Result<Color> {
    alt((color_hex, color_numeric))(input)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_numeric() {
        // spacing
        assert_eq!(color_numeric("rgb(1,2,3)").unwrap().1, Color::RGB(1, 2, 3));
        assert_eq!(
            color_numeric("rgb(1, 2, 3)").unwrap().1,
            Color::RGB(1, 2, 3)
        );
        assert_eq!(
            color_numeric("rgb(  1  ,  2  ,  3  )").unwrap().1,
            Color::RGB(1, 2, 3)
        );
        assert!(color_numeric("rgb(  1  ,  2  ,  3  ,)").is_err());
        assert!(color_numeric("rgb (1, 2, 3)").is_err());

        assert_eq!(
            color_numeric("rgba(1,2,3,4)").unwrap().1,
            Color::RGBA(1, 2, 3, 4)
        );
        assert_eq!(
            color_numeric("rgba(1, 2, 3, 4)").unwrap().1,
            Color::RGBA(1, 2, 3, 4)
        );
        assert_eq!(
            color_numeric("rgba(  1  ,  2  ,  3  ,  4  )").unwrap().1,
            Color::RGBA(1, 2, 3, 4)
        );
        assert!(color_numeric("rgba(  1  ,  2  ,  3  ,  4  ,)").is_err());
        assert!(color_numeric("rgba (1, 2, 3,4)").is_err());

        // valid numbers
        assert_eq!(
            color_numeric("rgb(0, 100, 255)").unwrap().1,
            Color::RGB(0, 100, 255)
        );
        assert!(color_numeric("rgb(0, 256, 0)").is_err());
        assert!(color_numeric("rgb(-1, 0, 0)").is_err());
    }

    #[test]
    fn test_color_hex() {
        assert_eq!(color_hex("#000000").unwrap().1, Color::RGB(0, 0, 0));
        assert_eq!(
            color_hex("#112233").unwrap().1,
            Color::RGB(0x11, 0x22, 0x33)
        );
        assert_eq!(color_hex("#00000000").unwrap().1, Color::RGBA(0, 0, 0, 0));
        assert_eq!(
            color_hex("#11223344").unwrap().1,
            Color::RGBA(0x11, 0x22, 0x33, 0x44)
        );
    }

    #[test]
    fn test_text_no_color() {
        assert_eq!(non_color_text("apple #000000").unwrap().1, "apple ");
        assert_eq!(non_color_text("apple rgb(1,2,3)").unwrap().1, "apple ");
        assert!(non_color_text("rgba(1, 2, 3,4)").is_err());
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
            text_with_colors("apple rgb(1,2,3) apple").unwrap().1,
            vec![
                TextElement::Text("apple "),
                TextElement::Color(Color::RGB(1, 2, 3)),
                TextElement::Text(" apple")
            ]
        );
    }
}
