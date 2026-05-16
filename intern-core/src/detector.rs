use crate::model::{ElementKind, SLIDE_HEIGHT_EMU, SLIDE_WIDTH_EMU, SlideData};

// Two-column detection: columns may overlap by at most 5% of slide width before we reject the layout.
const COLUMN_OVERLAP_TOLERANCE_EMU: i64 = SLIDE_WIDTH_EMU / 20;
// Grid clustering: shapes within this distance (y-axis) are grouped into the same row.
const GRID_ROW_TOLERANCE_EMU: i64 = SLIDE_HEIGHT_EMU / 15;
// Grid clustering: shapes within this distance (x-axis) are grouped into the same column.
const GRID_COL_TOLERANCE_EMU: i64 = SLIDE_WIDTH_EMU / 15;
// Grid density: require at least 2/3 of expected cells to be filled before treating as a grid.
const GRID_MIN_FILL_NUMER: usize = 2;
const GRID_MIN_FILL_DENOM: usize = 3;

pub enum SlideLayout {
    TwoColumn {
        left: Vec<usize>,
        right: Vec<usize>,
    },
    Grid {
        rows: Vec<Vec<usize>>,
        cols: Vec<Vec<usize>>,
    },
    Other,
}

pub fn detect(slide: &SlideData) -> SlideLayout {
    let content: Vec<usize> = slide
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e.kind, ElementKind::Title | ElementKind::Body))
        .map(|(i, _)| i)
        .collect();

    if content.len() < 2 {
        return SlideLayout::Other;
    }

    if let Some(layout) = try_two_column(slide, &content) {
        return layout;
    }

    if content.len() >= 3
        && let Some(layout) = try_grid(slide, &content)
    {
        return layout;
    }

    SlideLayout::Other
}

fn try_two_column(slide: &SlideData, indices: &[usize]) -> Option<SlideLayout> {
    let mid = SLIDE_WIDTH_EMU / 2;
    let left: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&i| slide.elements[i].rect.cx() < mid)
        .collect();
    let right: Vec<usize> = indices
        .iter()
        .copied()
        .filter(|&i| slide.elements[i].rect.cx() >= mid)
        .collect();

    if left.is_empty() || right.is_empty() {
        return None;
    }

    // Reject if columns heavily overlap (more than 5% of slide width)
    let left_max_right = left.iter().map(|&i| slide.elements[i].rect.right()).max()?;
    let right_min_left = right.iter().map(|&i| slide.elements[i].rect.x).min()?;
    if right_min_left < left_max_right - COLUMN_OVERLAP_TOLERANCE_EMU {
        return None;
    }

    Some(SlideLayout::TwoColumn { left, right })
}

fn try_grid(slide: &SlideData, indices: &[usize]) -> Option<SlideLayout> {
    let row_tol = GRID_ROW_TOLERANCE_EMU;
    let col_tol = GRID_COL_TOLERANCE_EMU;

    let mut by_y = indices.to_vec();
    by_y.sort_by_key(|&i| slide.elements[i].rect.y);

    let mut rows: Vec<Vec<usize>> = Vec::new();
    for idx in by_y {
        let y = slide.elements[idx].rect.y;
        if let Some(row) = rows
            .iter_mut()
            // SAFETY: every row starts as vec![idx] and is never emptied, so row[0] always exists.
            .find(|row| (slide.elements[row[0]].rect.y - y).abs() <= row_tol)
        {
            row.push(idx);
        } else {
            rows.push(vec![idx]);
        }
    }

    if rows.len() < 2 {
        return None;
    }

    let mut by_x = indices.to_vec();
    by_x.sort_by_key(|&i| slide.elements[i].rect.x);

    let mut cols: Vec<Vec<usize>> = Vec::new();
    for idx in by_x {
        let x = slide.elements[idx].rect.x;
        if let Some(col) = cols
            .iter_mut()
            // SAFETY: every col starts as vec![idx] and is never emptied, so col[0] always exists.
            .find(|col| (slide.elements[col[0]].rect.x - x).abs() <= col_tol)
        {
            col.push(idx);
        } else {
            cols.push(vec![idx]);
        }
    }

    if cols.len() < 2 {
        return None;
    }

    let expected_cells = rows.len() * cols.len();
    let filled_cells = indices.len();
    let too_sparse = filled_cells * GRID_MIN_FILL_DENOM < expected_cells * GRID_MIN_FILL_NUMER;
    if too_sparse {
        return None;
    }

    Some(SlideLayout::Grid { rows, cols })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, SlideElement};

    fn el(kind: ElementKind, x: i64, y: i64, w: i64, h: i64) -> SlideElement {
        SlideElement {
            name: "e".into(),
            kind,
            rect: Rect { x, y, w, h },
            font_size: None,
            font_family: None,
            text_color: None,
            paragraphs: vec![],
        }
    }

    fn slide(elements: Vec<SlideElement>) -> SlideData {
        SlideData { index: 0, elements }
    }

    #[test]
    fn fewer_than_two_content_elements_is_other() {
        let s = slide(vec![el(
            ElementKind::Image,
            100_000,
            100_000,
            500_000,
            500_000,
        )]);
        assert!(matches!(detect(&s), SlideLayout::Other));
    }

    #[test]
    fn title_and_body_do_not_count_as_content() {
        let s = slide(vec![
            el(ElementKind::Title, 0, 0, 500_000, 500_000),
            el(ElementKind::Body, 0, 1_000_000, 500_000, 500_000),
        ]);
        assert!(matches!(detect(&s), SlideLayout::Other));
    }

    #[test]
    fn two_column_detected_when_split_across_midpoint() {
        let s = slide(vec![
            el(ElementKind::TextBox, 400_000, 1_000_000, 1_000_000, 500_000),
            el(
                ElementKind::TextBox,
                5_000_000,
                1_000_000,
                1_000_000,
                500_000,
            ),
        ]);
        match detect(&s) {
            SlideLayout::TwoColumn { left, right } => {
                assert_eq!(left, vec![0]);
                assert_eq!(right, vec![1]);
            }
            _ => panic!("expected TwoColumn"),
        }
    }

    #[test]
    fn heavily_overlapping_columns_are_not_two_column() {
        // The left element is wide enough to reach deep into the right half.
        let s = slide(vec![
            el(ElementKind::TextBox, 400_000, 1_000_000, 4_500_000, 500_000),
            el(
                ElementKind::TextBox,
                4_400_000,
                1_000_000,
                1_000_000,
                500_000,
            ),
        ]);
        assert!(matches!(detect(&s), SlideLayout::Other));
    }

    #[test]
    fn grid_detected_for_two_by_two() {
        // All four elements sit right of the midpoint so the two-column
        // detector finds an empty left column and falls through to grid.
        let s = slide(vec![
            el(ElementKind::Image, 5_000_000, 1_000_000, 1_000_000, 800_000),
            el(ElementKind::Image, 7_000_000, 1_000_000, 1_000_000, 800_000),
            el(ElementKind::Image, 5_000_000, 3_000_000, 1_000_000, 800_000),
            el(ElementKind::Image, 7_000_000, 3_000_000, 1_000_000, 800_000),
        ]);
        match detect(&s) {
            SlideLayout::Grid { rows, cols } => {
                assert_eq!(rows.len(), 2);
                assert_eq!(cols.len(), 2);
            }
            _ => panic!("expected Grid"),
        }
    }

    #[test]
    fn sparse_grid_is_other() {
        // 3 elements spanning 2 rows × 3 columns - only half the cells filled.
        let s = slide(vec![
            el(ElementKind::Image, 5_000_000, 1_000_000, 1_000_000, 800_000),
            el(ElementKind::Image, 8_000_000, 1_000_000, 1_000_000, 800_000),
            el(ElementKind::Image, 6_500_000, 3_000_000, 1_000_000, 800_000),
        ]);
        assert!(matches!(detect(&s), SlideLayout::Other));
    }
}
