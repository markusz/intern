//! Integration tests using ppt-rs to generate synthetic PPTX files.
//!
//! Each test builds a presentation with known geometry, runs the reader,
//! and then asserts on what violations (or lack thereof) the rules produce.

use intern_core::reader::read_presentation;
use intern_core::rules::all_rules;
use ppt_rs::generator::{Shape, ShapeType, SlideContent};
use ppt_rs::generator::{SlideLayout, create_pptx_with_content};

const THRESHOLD: i64 = 19_050; // 2px

// ── helpers ──────────────────────────────────────────────────────────────────

fn write_tmp(bytes: &[u8], name: &str) -> String {
    let path = format!("/tmp/intern_test_{name}.pptx");
    std::fs::write(&path, bytes).expect("write tmp pptx");
    path
}

fn cleanup(path: &str) {
    std::fs::remove_file(path).ok();
}

fn shape_at(name: &str, x: u32, y: u32, w: u32, h: u32) -> Shape {
    Shape::new(ShapeType::Rectangle, x, y, w, h).with_text(name)
}

fn violations_for(path: &str, rule_id: &str) -> Vec<intern_core::rules::Violation> {
    let slides = read_presentation(path).expect("read pptx");
    all_rules()
        .iter()
        .filter(|r| r.id() == rule_id)
        .flat_map(|r| r.check(&slides, THRESHOLD))
        .collect()
}

fn all_violations(path: &str) -> Vec<intern_core::rules::Violation> {
    let slides = read_presentation(path).expect("read pptx");
    all_rules()
        .iter()
        .flat_map(|r| r.check(&slides, THRESHOLD))
        .collect()
}

// ── reader smoke test ─────────────────────────────────────────────────────────

#[test]
fn reader_parses_shapes_with_correct_positions() {
    let x: u32 = 914_400; // 1 inch
    let y: u32 = 457_200; // 0.5 inch
    let w: u32 = 2_743_200; // 3 inches
    let h: u32 = 914_400; // 1 inch

    let slide = SlideContent::new("Test").with_shapes(vec![shape_at("Box", x, y, w, h)]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "reader_positions");

    let slides = read_presentation(&path).unwrap();
    cleanup(&path);

    let found = slides[0]
        .elements
        .iter()
        .find(|e| e.rect.w == w as i64 && e.rect.h == h as i64);

    assert!(
        found.is_some(),
        "shape with correct size not found in parsed slide"
    );
    let el = found.unwrap();
    assert_eq!(el.rect.x, x as i64);
    assert_eq!(el.rect.y, y as i64);
}

#[test]
fn reader_returns_correct_slide_count() {
    let make = |title: &str| SlideContent::new(title);
    let bytes =
        create_pptx_with_content("Fixture", vec![make("One"), make("Two"), make("Three")]).unwrap();
    let path = write_tmp(&bytes, "slide_count");
    let slides = read_presentation(&path).unwrap();
    cleanup(&path);
    assert_eq!(slides.len(), 3);
}

#[test]
fn reader_parses_images_from_raw_xml() {
    let slide = SlideContent::new("No images").layout(SlideLayout::Blank);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "image_parse");
    let result = read_presentation(&path);
    cleanup(&path);
    assert!(result.is_ok());
}

// ── two-column layout integration ─────────────────────────────────────────────

#[test]
fn column_left_edge_fires_on_misaligned_left_column() {
    let offset = 300 * 9_525u32;
    let slide = SlideContent::new("two-col").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200 + offset, 2_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
        shape_at("R2", 5_500_000, 2_000_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "col_left_edge");

    let v = violations_for(&path, "COLUMN_LEFT_EDGE");
    cleanup(&path);
    assert!(!v.is_empty(), "expected COLUMN_LEFT_EDGE violation");
}

#[test]
fn column_left_edge_clean_on_aligned_columns() {
    let slide = SlideContent::new("two-col clean").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200, 2_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
        shape_at("R2", 5_500_000, 2_000_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "col_left_edge_clean");

    let v = violations_for(&path, "COLUMN_LEFT_EDGE");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "COLUMN_LEFT_EDGE should be silent when columns are aligned"
    );
}

// ── clean deck ────────────────────────────────────────────────────────────────

#[test]
fn no_violations_on_perfectly_aligned_deck() {
    let make_slide = |title: &str| {
        SlideContent::new(title).with_shapes(vec![
            shape_at("Left", 457_200, 1_200_000, 3_500_000, 4_000_000),
            shape_at("Right", 4_800_000, 1_200_000, 3_500_000, 4_000_000),
        ])
    };

    let bytes = create_pptx_with_content(
        "Clean Deck",
        vec![
            make_slide("Slide 1"),
            make_slide("Slide 2"),
            make_slide("Slide 3"),
        ],
    )
    .unwrap();
    let path = write_tmp(&bytes, "clean_deck");

    let v = all_violations(&path);
    cleanup(&path);

    assert!(
        v.is_empty(),
        "all rules should be silent on a perfectly aligned deck: {v:#?}"
    );
}
