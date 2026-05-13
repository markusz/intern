use crate::model::{ElementKind, SLIDE_HEIGHT_EMU, SLIDE_WIDTH_EMU, SlideData};

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
    if right_min_left < left_max_right - SLIDE_WIDTH_EMU / 20 {
        return None;
    }

    Some(SlideLayout::TwoColumn { left, right })
}

fn try_grid(slide: &SlideData, indices: &[usize]) -> Option<SlideLayout> {
    let row_tol = SLIDE_HEIGHT_EMU / 15;
    let col_tol = SLIDE_WIDTH_EMU / 15;

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

    // Require at least 2/3 of expected cells to be filled
    if indices.len() < rows.len() * cols.len() * 2 / 3 {
        return None;
    }

    Some(SlideLayout::Grid { rows, cols })
}
