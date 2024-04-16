use nom::{
    bytes::complete::tag,
    character::complete::{char, space0, u8},
    combinator::cut,
    sequence::{delimited, tuple},
    IResult, Parser,
};

type Input = str;

type Result<'a, O = &'a Input> = IResult<&'a Input, O>;

fn color_rgb_value(input: &Input) -> Result<u8> {
    delimited(space0, u8, space0)(input)
}

fn color_rgb(input: &Input) -> Result<(u8, u8, u8)> {
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
    .map(|(r, _, g, _, b)| (r, g, b))
    .parse(input)
}

fn color_rgba(input: &Input) -> Result<(u8, u8, u8, u8)> {
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
    .map(|(r, _, g, _, b, _, a)| (r, g, b, a))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgb() {
        // spacing
        assert_eq!(color_rgb("rgb(1,2,3)").unwrap().1, (1, 2, 3));
        assert_eq!(color_rgb("rgb(1, 2, 3)").unwrap().1, (1, 2, 3));
        assert_eq!(color_rgb("rgb(  1  ,  2  ,  3  )").unwrap().1, (1, 2, 3));
        assert!(color_rgb("rgb(  1  ,  2  ,  3  ,)").is_err());
        assert!(color_rgb("rgb (1, 2, 3)").is_err());

        // valid numbers
        assert_eq!(color_rgb("rgb(0, 100, 255)").unwrap().1, (0, 100, 255));
        assert!(color_rgb("rgb(0, 256, 0)").is_err());
        assert!(color_rgb("rgb(-1, 0, 0)").is_err());
    }
}
