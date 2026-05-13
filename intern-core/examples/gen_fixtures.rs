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
        .unwrap()
        .join("fixtures");
    fs::create_dir_all(&dir).expect("create fixtures/");
    let path = dir.join(filename);
    fs::write(&path, &bytes).expect("write fixture");
    println!("  {}", path.display());
}

fn main() {
    println!("Generating fixtures...");
    clean_deck();
    two_column_misaligned();
    grid_uneven_h_spacing();
    grid_row_misaligned();
    println!("Done.");
}

/// 3-slide deck with no violations — happy-path baseline.
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

/// Two-column layout where left-column shapes have inconsistent X edges.
/// Fires: COLUMN_LEFT_EDGE
fn two_column_misaligned() {
    let col_w: u32 = 3_200_000;
    let col_h: u32 = 900_000;
    let left_x = MARGIN;
    let right_x = SLIDE_W / 2 + MARGIN;
    let gap: u32 = 1_050_000;

    let shapes: Vec<Shape> = vec![
        // Left column — intentionally misaligned X on some rows
        rect("L1", left_x, MARGIN + gap * 0, col_w, col_h),
        rect("L2", left_x + 95_250, MARGIN + gap * 1, col_w, col_h), // ~10px right
        rect("L3", left_x, MARGIN + gap * 2, col_w, col_h),
        rect("L4", left_x + 285_750, MARGIN + gap * 3, col_w, col_h), // ~30px right
        rect("L5", left_x, MARGIN + gap * 4, col_w, col_h),
        // Right column — perfectly aligned
        rect("R1", right_x, MARGIN + gap * 0, col_w, col_h),
        rect("R2", right_x, MARGIN + gap * 1, col_w, col_h),
        rect("R3", right_x, MARGIN + gap * 2, col_w, col_h),
        rect("R4", right_x, MARGIN + gap * 3, col_w, col_h),
        rect("R5", right_x, MARGIN + gap * 4, col_w, col_h),
    ];

    save(
        "two_column.pptx",
        "Two Column",
        vec![SlideContent::new("Two Column").with_shapes(shapes)],
    );
}

/// 2×2 grid entirely right of the slide midpoint so the two-column detector
/// sees an empty left column and falls through to grid detection.
/// Fires: GRID_H_SPACING (uneven horizontal gap between rows)
fn grid_uneven_h_spacing() {
    let cell_w: u32 = 2_000_000;
    let cell_h: u32 = 2_000_000;

    // Both columns have cx > slide midpoint (4_572_000) → two-column left=[∅] → fails.
    let col0_x: u32 = 3_700_000; // cx = 4_700_000 ✓
    let row0_y: u32 = 800_000;
    let row1_y: u32 = 3_200_000;

    // Row 0 gap: 500k (~52px). Row 1 gap: 800k (~84px).
    // Global median = 800k; row 0 deviates by 300k (~31px) → GRID_H_SPACING fires.
    let col1_x_row0: u32 = col0_x + cell_w + 500_000;
    let col1_x_row1: u32 = col0_x + cell_w + 800_000;

    let shapes = vec![
        rect("A1", col0_x, row0_y, cell_w, cell_h),
        rect("A2", col1_x_row0, row0_y, cell_w, cell_h),
        rect("B1", col0_x, row1_y, cell_w, cell_h),
        rect("B2", col1_x_row1, row1_y, cell_w, cell_h),
    ];

    save(
        "grid_h_spacing.pptx",
        "Grid H Spacing",
        vec![SlideContent::new("Grid").with_shapes(shapes)],
    );
}

/// Same right-of-midpoint 2×2 grid, but one element's top edge is shifted
/// relative to its row peers.
/// Fires: GRID_ROW_TOP
fn grid_row_misaligned() {
    let cell_w: u32 = 2_000_000;
    let cell_h: u32 = 2_000_000;

    let col0_x: u32 = 3_700_000;
    let col1_x: u32 = col0_x + cell_w + 600_000;
    let row0_y: u32 = 800_000;
    let row1_y: u32 = 3_200_000;

    // A2 is shifted down by ~10.5px relative to A1 → GRID_ROW_TOP fires.
    let a2_y = row0_y + 100_000;

    let shapes = vec![
        rect("A1", col0_x, row0_y, cell_w, cell_h),
        rect("A2", col1_x, a2_y, cell_w, cell_h),
        rect("B1", col0_x, row1_y, cell_w, cell_h),
        rect("B2", col1_x, row1_y, cell_w, cell_h),
    ];

    save(
        "grid_row_top.pptx",
        "Grid Row Top",
        vec![SlideContent::new("Grid Row").with_shapes(shapes)],
    );
}
