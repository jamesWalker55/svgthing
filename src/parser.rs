use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, one_of, space0, u8},
    combinator::{cut, opt, recognize},
    multi::count,
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
}
