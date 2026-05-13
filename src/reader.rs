use ppt_rs::opc::Package;
use ppt_rs::oxml::{SlideParser, XmlParser};

use crate::error::PptlintError;
use crate::model::{ElementKind, Rect, SlideData, SlideElement};

pub fn read_presentation(path: &str) -> Result<Vec<SlideData>, PptlintError> {
    let pkg = Package::open(path).map_err(|e| PptlintError::Open {
        path: path.to_string(),
        message: e.to_string(),
    })?;
    let paths = slide_order(&pkg);
    paths
        .iter()
        .enumerate()
        .map(|(idx, slide_path)| {
            let xml = pkg
                .get_part_string(slide_path)
                .ok_or_else(|| PptlintError::SlideMissing(slide_path.clone()))?;
            parse_slide(idx, &xml)
        })
        .collect()
}

fn parse_slide(index: usize, xml: &str) -> Result<SlideData, PptlintError> {
    let parsed = SlideParser::parse(xml).map_err(|e| PptlintError::ParseSlide(e.to_string()))?;
    let mut elements: Vec<SlideElement> = parsed
        .shapes
        .into_iter()
        .filter(|s| s.width > 0 && s.height > 0)
        .map(|s| {
            let kind = if s.is_title {
                ElementKind::Title
            } else if s.is_body {
                ElementKind::Body
            } else {
                ElementKind::TextBox
            };
            let font_size = s
                .paragraphs
                .iter()
                .flat_map(|p| p.runs.iter())
                .find_map(|r| r.font_size);
            SlideElement {
                name: s.name,
                kind,
                rect: Rect {
                    x: s.x,
                    y: s.y,
                    w: s.width,
                    h: s.height,
                },
                font_size,
            }
        })
        .collect();

    elements.extend(parse_images(xml));

    Ok(SlideData { index, elements })
}

fn parse_images(xml: &str) -> Vec<SlideElement> {
    let Ok(root) = XmlParser::parse_str(xml) else {
        return vec![];
    };
    let Some(sp_tree) = root.find_descendant("spTree") else {
        return vec![];
    };

    sp_tree
        .find_all_descendants("pic")
        .into_iter()
        .filter_map(|pic| {
            let name = pic
                .find_descendant("cNvPr")
                .and_then(|e| e.attr("name"))
                .unwrap_or("Picture")
                .to_string();
            let xfrm = pic.find_descendant("xfrm")?;
            let off = xfrm.find("off")?;
            let ext = xfrm.find("ext")?;
            let x: i64 = off.attr("x")?.parse().ok()?;
            let y: i64 = off.attr("y")?.parse().ok()?;
            let w: i64 = ext.attr("cx")?.parse().ok()?;
            let h: i64 = ext.attr("cy")?.parse().ok()?;
            if w <= 0 || h <= 0 {
                return None;
            }
            Some(SlideElement {
                name,
                kind: ElementKind::Image,
                rect: Rect { x, y, w, h },
                font_size: None,
            })
        })
        .collect()
}

fn slide_order(pkg: &Package) -> Vec<String> {
    let Some(rels_xml) = pkg.get_part_string("ppt/_rels/presentation.xml.rels") else {
        return fallback_order(pkg);
    };
    let Ok(root) = XmlParser::parse_str(&rels_xml) else {
        return fallback_order(pkg);
    };

    let mut rels: Vec<(u32, String)> = root
        .find_all("Relationship")
        .into_iter()
        .filter(|r| {
            r.attr("Type")
                .map(|t| t.contains("/slide") && !t.contains("Layout") && !t.contains("Master"))
                .unwrap_or(false)
        })
        .filter_map(|r| {
            let id: u32 = r.attr("Id")?.trim_start_matches("rId").parse().ok()?;
            let target = r.attr("Target")?;
            let path = if let Some(stripped) = target.strip_prefix('/') {
                stripped.to_string()
            } else {
                format!("ppt/{target}")
            };
            Some((id, path))
        })
        .collect();

    rels.sort_by_key(|(n, _)| *n);
    rels.into_iter().map(|(_, p)| p).collect()
}

fn fallback_order(pkg: &Package) -> Vec<String> {
    let mut paths: Vec<String> = pkg
        .part_paths()
        .into_iter()
        .filter(|p| {
            p.starts_with("ppt/slides/slide") && p.ends_with(".xml") && !p.contains("_rels")
        })
        .map(str::to_string)
        .collect();
    paths.sort();
    paths
}
