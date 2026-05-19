//! Generates PPTX fixture files into `fixtures/` at the workspace root.
//! Run with: cargo run -p intern-core --example gen_fixtures

use ppt_rs::generator::{Shape, ShapeType, SlideContent, create_pptx_with_content};
use std::fs;
use std::path::Path;

const MARGIN: u32 = 457_200; // 0.5 inch in EMU
const SLIDE_W: u32 = 9_144_000;

fn rect(name: &str, x: u32, y: u32, w: u32, h: u32) -> Shape {
    Shape::new(ShapeType::Rectangle, x, y, w, h).with_text(name)
}

fn save(filename: &str, title: &str, slides: Vec<SlideContent>) {
    let bytes = create_pptx_with_content(title, slides).expect("generate pptx");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent")
        .join("fixtures");
    fs::create_dir_all(&dir).expect("create fixtures/");
    let path = dir.join(filename);
    fs::write(&path, &bytes).expect("write fixture");
    println!("  {}", path.display());
}

fn main() {
    println!("Generating fixtures...");
    clean_deck();
    println!("Done.");
}

/// 3-slide deck with no violations - happy-path baseline.
fn clean_deck() {
    let col_w: u32 = 3_800_000;
    let col_h: u32 = 4_000_000;
    let left_x = MARGIN;
    let right_x = SLIDE_W - MARGIN - col_w;
    let y: u32 = 1_600_000;

    let slide = |n: u8| {
        SlideContent::new(&format!("Slide {n}")).with_shapes(vec![
            rect("Left", left_x, y, col_w, col_h),
            rect("Right", right_x, y, col_w, col_h),
        ])
    };

    save(
        "clean.pptx",
        "Clean Deck",
        vec![slide(1), slide(2), slide(3)],
    );
}
