//! Integration tests using ppt-rs to generate synthetic PPTX files.
//!
//! Each test builds a presentation with known geometry, runs the reader,
//! and then asserts on what violations (or lack thereof) the rules produce.

use intern_core::model::Presentation;
use intern_core::reader::{read_presentation, slide_exclusions};
use intern_core::rules::{Fix, Limits, RuleContext, all_rules};
use intern_core::writer::{append_notes_directive, apply_fixes};
use ppt_rs::generator::{Shape, ShapeType, SlideContent};
use ppt_rs::generator::{SlideLayout, create_pptx_with_content};

const THRESHOLD: i64 = 19_050; // 2px

fn write_tmp(bytes: &[u8], name: &str) -> String {
    let path = format!("/tmp/intern_test_{name}.pptx");
    std::fs::write(&path, bytes).expect("write tmp pptx");
    path
}

fn cleanup(path: &str) {
    std::fs::remove_file(path).ok();
    std::fs::remove_file(format!("{path}.bak")).ok();
}

fn ctx_for(pres: &Presentation) -> RuleContext {
    RuleContext {
        threshold: THRESHOLD,
        slide_width: pres.slide_width,
        slide_height: pres.slide_height,
    }
}

fn fixes_for(path: &str) -> Vec<Fix> {
    let pres = read_presentation(path).expect("read pptx");
    let ctx = ctx_for(&pres);
    all_rules(&Limits::default())
        .iter()
        .flat_map(|r| r.check(&pres.slides, &ctx))
        .filter_map(|v| v.fix)
        .collect()
}

fn shape_at(name: &str, x: u32, y: u32, w: u32, h: u32) -> Shape {
    Shape::new(ShapeType::Rectangle, x, y, w, h).with_text(name)
}

fn violations_for(path: &str, rule_id: &str) -> Vec<intern_core::rules::Violation> {
    let pres = read_presentation(path).expect("read pptx");
    let ctx = ctx_for(&pres);
    all_rules(&Limits::default())
        .iter()
        .filter(|r| r.id() == rule_id)
        .flat_map(|r| r.check(&pres.slides, &ctx))
        .collect()
}

fn all_violations(path: &str) -> Vec<intern_core::rules::Violation> {
    let pres = read_presentation(path).expect("read pptx");
    let ctx = ctx_for(&pres);
    all_rules(&Limits::default())
        .iter()
        .flat_map(|r| r.check(&pres.slides, &ctx))
        .collect()
}

#[test]
fn reader_parses_shapes_with_correct_positions() {
    let x: u32 = 914_400; // 1 inch
    let y: u32 = 457_200; // 0.5 inch
    let w: u32 = 2_743_200; // 3 inches
    let h: u32 = 914_400; // 1 inch

    let slide = SlideContent::new("Test").with_shapes(vec![shape_at("Box", x, y, w, h)]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "reader_positions");

    let pres = read_presentation(&path).unwrap();
    cleanup(&path);

    let found = pres.slides[0]
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
    let pres = read_presentation(&path).unwrap();
    cleanup(&path);
    assert_eq!(pres.slides.len(), 3);
}

// Image XML parsing itself is covered by unit tests in reader.rs; this only
// checks that a slide without any pictures still reads cleanly.
#[test]
fn reader_handles_slide_without_images() {
    let slide = SlideContent::new("No images").layout(SlideLayout::Blank);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "no_images");
    let result = read_presentation(&path);
    cleanup(&path);
    assert!(result.is_ok());
}

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

#[test]
fn fix_column_left_edge_round_trips() {
    let offset = 300 * 9_525u32; // 300 px - well beyond threshold
    let slide = SlideContent::new("two-col").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200 + offset, 2_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
        shape_at("R2", 5_500_000, 2_000_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "fix_col_left");

    let fixes = fixes_for(&path);
    assert!(!fixes.is_empty(), "expected fixable violations before fix");

    apply_fixes(&path, &fixes).expect("apply_fixes failed");

    let v = violations_for(&path, "COLUMN_LEFT_EDGE");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "expected no COLUMN_LEFT_EDGE violations after fix: {v:#?}"
    );
}

#[test]
fn fix_grid_row_top_round_trips() {
    // 2×2 grid right of midpoint so two-column detector falls through to grid.
    let (cw, ch) = (2_000_000u32, 2_000_000u32);
    let col0_x: u32 = 3_700_000;
    let col1_x: u32 = col0_x + cw + 600_000;
    let row0_y: u32 = 800_000;
    let row1_y: u32 = 3_200_000;
    let skew: u32 = 100_000; // ~10.5 px - beyond 2 px threshold

    let slide = SlideContent::new("grid").with_shapes(vec![
        shape_at("A1", col0_x, row0_y, cw, ch),
        shape_at("A2", col1_x, row0_y + skew, cw, ch), // shifted down
        shape_at("B1", col0_x, row1_y, cw, ch),
        shape_at("B2", col1_x, row1_y, cw, ch),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "fix_grid_row");

    let fixes = fixes_for(&path);
    assert!(!fixes.is_empty(), "expected fixable violations before fix");

    apply_fixes(&path, &fixes).expect("apply_fixes failed");

    let v = violations_for(&path, "GRID_ROW_TOP");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "expected no GRID_ROW_TOP violations after fix: {v:#?}"
    );
}

#[test]
fn fix_column_right_left_edge_round_trips() {
    let slide = SlideContent::new("two-col").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200, 2_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
        shape_at(
            "R2",
            5_500_000 + 300 * 9_525u32,
            2_000_000,
            1_500_000,
            600_000,
        ),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "fix_col_right");

    let fixes = fixes_for(&path);
    assert!(!fixes.is_empty(), "expected fixable violations before fix");

    apply_fixes(&path, &fixes).expect("apply_fixes failed");

    let v = violations_for(&path, "COLUMN_RIGHT_LEFT_EDGE");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "expected no COLUMN_RIGHT_LEFT_EDGE violations after fix: {v:#?}"
    );
}

#[test]
fn fix_column_top_edge_round_trips() {
    let slide = SlideContent::new("two-col").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200, 2_000_000, 1_500_000, 600_000),
        // Right column starts 300px lower.
        shape_at(
            "R1",
            5_500_000,
            1_000_000 + 300 * 9_525u32,
            1_500_000,
            600_000,
        ),
        shape_at("R2", 5_500_000, 2_500_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "fix_col_top");

    let fixes = fixes_for(&path);
    assert!(!fixes.is_empty(), "expected fixable violations before fix");

    apply_fixes(&path, &fixes).expect("apply_fixes failed");

    let v = violations_for(&path, "COLUMN_TOP_EDGE");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "expected no COLUMN_TOP_EDGE violations after fix: {v:#?}"
    );
}

#[test]
fn fix_grid_col_left_round_trips() {
    // 2×2 grid right of midpoint.
    let (cw, ch) = (2_000_000u32, 2_000_000u32);
    let col0_x: u32 = 3_700_000;
    let col1_x: u32 = col0_x + cw + 600_000;
    let row0_y: u32 = 800_000;
    let row1_y: u32 = 3_200_000;
    let skew: u32 = 100_000;

    let slide = SlideContent::new("grid").with_shapes(vec![
        shape_at("A1", col0_x, row0_y, cw, ch),
        shape_at("A2", col1_x, row0_y, cw, ch),
        shape_at("B1", col0_x + skew, row1_y, cw, ch), // col 0 bottom shifted right
        shape_at("B2", col1_x, row1_y, cw, ch),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "fix_grid_col");

    let fixes = fixes_for(&path);
    assert!(!fixes.is_empty(), "expected fixable violations before fix");

    apply_fixes(&path, &fixes).expect("apply_fixes failed");

    let v = violations_for(&path, "GRID_COL_LEFT");
    cleanup(&path);
    assert!(
        v.is_empty(),
        "expected no GRID_COL_LEFT violations after fix: {v:#?}"
    );
}

#[test]
fn no_violations_on_perfectly_aligned_deck() {
    let make_slide = |title: &str| {
        SlideContent::new(title).with_shapes(vec![
            shape_at("Left", 457_200, 1_600_000, 3_500_000, 4_000_000),
            shape_at("Right", 4_800_000, 1_600_000, 3_500_000, 4_000_000),
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

#[test]
fn element_overflow_detected() {
    // A shape whose right edge runs past the 9_144_000 EMU slide width.
    let slide = SlideContent::new("overflow").with_shapes(vec![shape_at(
        "OffEdge", 8_000_000, 1_000_000, 2_000_000, 500_000,
    )]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "overflow");

    let v = violations_for(&path, "ELEMENT_OVERFLOW");
    cleanup(&path);
    assert!(
        !v.is_empty(),
        "expected ELEMENT_OVERFLOW for a shape past the slide edge"
    );
}

#[test]
fn text_element_overlap_detected() {
    let slide = SlideContent::new("overlap").with_shapes(vec![
        shape_at("A", 1_000_000, 3_000_000, 2_000_000, 2_000_000),
        shape_at("B", 2_000_000, 4_000_000, 2_000_000, 2_000_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "overlap");

    let v = violations_for(&path, "TEXT_ELEMENT_OVERLAP");
    cleanup(&path);
    assert!(
        !v.is_empty(),
        "expected TEXT_ELEMENT_OVERLAP for two intersecting text shapes"
    );
}

#[test]
fn double_space_detected() {
    let slide = SlideContent::new("text").with_shapes(vec![shape_at(
        "Two  spaces in this box",
        1_000_000,
        3_000_000,
        4_000_000,
        1_000_000,
    )]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "double_space");

    let v = violations_for(&path, "DOUBLE_SPACE");
    cleanup(&path);
    assert!(
        !v.is_empty(),
        "expected DOUBLE_SPACE for shape text with a double space"
    );
}

#[test]
fn ignore_slide_level_suppresses_rule_for_whole_slide() {
    // ppt-rs never creates notes slides, so this exercises the "create from scratch" path.
    let slide = SlideContent::new("test").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("L2", 457_200, 2_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
        shape_at("R2", 5_500_000, 2_000_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "ignore_slide_level");

    append_notes_directive(&path, 0, None, "COLUMN_LEFT_EDGE")
        .expect("append_notes_directive failed");

    let exclusions = slide_exclusions(&path).expect("slide_exclusions failed");
    cleanup(&path);

    let ex = exclusions.get(&0).expect("no exclusion recorded for slide 0");
    assert!(
        ex.suppresses_rule_for_slide("COLUMN_LEFT_EDGE"),
        "expected COLUMN_LEFT_EDGE to be suppressed for slide 0"
    );
}

#[test]
fn ignore_element_level_suppresses_rule_for_that_element_only() {
    let slide = SlideContent::new("test").with_shapes(vec![
        shape_at("L1", 457_200, 1_000_000, 1_500_000, 600_000),
        shape_at("R1", 5_500_000, 1_000_000, 1_500_000, 600_000),
    ]);
    let bytes = create_pptx_with_content("Fixture", vec![slide]).unwrap();
    let path = write_tmp(&bytes, "ignore_element_level");

    append_notes_directive(&path, 0, Some(42), "ELEMENT_OVERFLOW")
        .expect("append_notes_directive failed");

    let exclusions = slide_exclusions(&path).expect("slide_exclusions failed");
    cleanup(&path);

    let ex = exclusions.get(&0).expect("no exclusion recorded for slide 0");
    assert!(
        ex.suppresses_rule_for_element(42, "ELEMENT_OVERFLOW"),
        "expected ELEMENT_OVERFLOW to be suppressed for element 42"
    );
    assert!(
        !ex.suppresses_rule_for_element(99, "ELEMENT_OVERFLOW"),
        "suppression should not apply to a different element id"
    );
}
