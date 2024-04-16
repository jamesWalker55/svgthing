mod parser;

use std::fs;

fn main() {
    let text = fs::read_to_string("svg/gen_env_write.svg").expect("failed to read svg into text");

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
