use crate::model::{ElementKind, Rect, SlideData};

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

pub fn detect(slide: &SlideData, slide_width: i64, slide_height: i64) -> SlideLayout {
    let content: Vec<usize> = slide
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| !matches!(e.kind, ElementKind::Title | ElementKind::Body))
        .map(|(i, _)| i)
        .collect();
    let content = drop_contained(slide, content);

    if content.len() < 2 {
        return SlideLayout::Other;
    }

    if let Some(layout) = try_two_column(slide, &content, slide_width) {
        return layout;
    }

    if content.len() >= 3
        && let Some(layout) = try_grid(slide, &content, slide_width, slide_height)
    {
        return layout;
    }

    SlideLayout::Other
}

// Drops elements fully contained within a larger content element. A box nested
// inside another (e.g. a label inside a visualization frame) is a child of that
// container, not an independently aligned element - only the outer box should be
// considered when checking column / grid alignment.
fn drop_contained(slide: &SlideData, content: Vec<usize>) -> Vec<usize> {
    content
        .iter()
        .copied()
        .filter(|&i| {
            let inner = &slide.elements[i].rect;
            !content
                .iter()
                .any(|&j| j != i && contains(&slide.elements[j].rect, inner))
        })
        .collect()
}

// True when `outer` fully encloses `inner` and is strictly larger - the area check
// keeps two identical rects from each "containing" the other.
fn contains(outer: &Rect, inner: &Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.right() >= inner.right()
        && outer.bottom() >= inner.bottom()
        && outer.w * outer.h > inner.w * inner.h
}

fn try_two_column(slide: &SlideData, indices: &[usize], slide_width: i64) -> Option<SlideLayout> {
    let mid = slide_width / 2;
    // Columns may overlap by at most 5% of slide width before we reject the layout.
    let column_overlap_tolerance = slide_width / 20;
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
    if right_min_left < left_max_right - column_overlap_tolerance {
        return None;
    }

    Some(SlideLayout::TwoColumn { left, right })
}

fn try_grid(
    slide: &SlideData,
    indices: &[usize],
    slide_width: i64,
    slide_height: i64,
) -> Option<SlideLayout> {
    // Shapes within this distance are clustered into the same grid row / column.
    let row_tol = slide_height / 15;
    let col_tol = slide_width / 15;

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
        assert!(matches!(
            detect(&s, 9_144_000, 6_858_000),
            SlideLayout::Other
        ));
    }

    #[test]
    fn title_and_body_do_not_count_as_content() {
        let s = slide(vec![
            el(ElementKind::Title, 0, 0, 500_000, 500_000),
            el(ElementKind::Body, 0, 1_000_000, 500_000, 500_000),
        ]);
        assert!(matches!(
            detect(&s, 9_144_000, 6_858_000),
            SlideLayout::Other
        ));
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
        match detect(&s, 9_144_000, 6_858_000) {
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
        assert!(matches!(
            detect(&s, 9_144_000, 6_858_000),
            SlideLayout::Other
        ));
    }

    #[test]
    fn contained_element_is_excluded_from_layout() {
        // A small box nested inside the left column's outer box must not count as
        // its own column element.
        let s = slide(vec![
            el(
                ElementKind::TextBox,
                400_000,
                1_000_000,
                3_000_000,
                3_000_000,
            ),
            el(
                ElementKind::TextBox,
                5_000_000,
                1_000_000,
                3_000_000,
                3_000_000,
            ),
            el(ElementKind::TextBox, 1_000_000, 1_500_000, 200_000, 200_000),
        ]);
        match detect(&s, 9_144_000, 6_858_000) {
            SlideLayout::TwoColumn { left, right } => {
                assert_eq!(left, vec![0], "nested element should be dropped");
                assert_eq!(right, vec![1]);
            }
            _ => panic!("expected TwoColumn"),
        }
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
        match detect(&s, 9_144_000, 6_858_000) {
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
        assert!(matches!(
            detect(&s, 9_144_000, 6_858_000),
            SlideLayout::Other
        ));
    }
}
