use std::collections::HashMap;

use ppt_rs::opc::Package;
use ppt_rs::oxml::slide::TextRun;
use ppt_rs::oxml::{SlideParser, XmlParser};

use crate::error::Error;
use crate::model::{ElementKind, Rect, SlideData, SlideElement};

pub fn read_presentation(path: &str) -> Result<Vec<SlideData>, Error> {
    let pkg = Package::open(path).map_err(|e| Error::Open {
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
                .ok_or_else(|| Error::SlideMissing(slide_path.clone()))?;
            parse_slide(idx, &xml)
        })
        .collect()
}

fn parse_slide(index: usize, xml: &str) -> Result<SlideData, Error> {
    let parsed = SlideParser::parse(xml).map_err(|e| Error::ParseSlide(e.to_string()))?;
    let families = parse_font_families(xml);
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
            let runs: Vec<_> = s.paragraphs.iter().flat_map(|p| p.runs.iter()).collect();
            let font_size = runs.iter().find_map(|r| r.font_size);
            let font_family = families.get(&s.name).cloned();
            let text_color = dominant_color(&runs);
            let paragraphs = s
                .paragraphs
                .iter()
                .map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>())
                .filter(|t| !t.trim().is_empty())
                .collect();
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
                font_family,
                text_color,
                paragraphs,
            }
        })
        .collect();

    elements.extend(parse_images(xml));

    Ok(SlideData { index, elements })
}

fn dominant_color(runs: &[&TextRun]) -> Option<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for run in runs {
        if let Some(ref c) = run.color {
            *counts.entry(c.clone()).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c)
}

// Returns a map of shape name → dominant font family, skipping theme font references
// like "+mj-lt" (major Latin) and "+mn-lt" (minor Latin).
fn parse_font_families(xml: &str) -> HashMap<String, String> {
    let Ok(root) = XmlParser::parse_str(xml) else {
        return HashMap::new();
    };
    let Some(sp_tree) = root.find_descendant("spTree") else {
        return HashMap::new();
    };
    sp_tree
        .find_all_descendants("sp")
        .into_iter()
        .filter_map(|sp| {
            let name = sp.find_descendant("cNvPr")?.attr("name")?.to_string();
            let typeface = sp
                .find_all_descendants("latin")
                .into_iter()
                .filter_map(|e| e.attr("typeface").map(str::to_string))
                .find(|t| !t.starts_with('+'))?;
            Some((name, typeface))
        })
        .collect()
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
                font_family: None,
                text_color: None,
                paragraphs: vec![],
            })
        })
        .collect()
}

fn slide_order(pkg: &Package) -> Vec<String> {
    let Some(rels_xml) = pkg.get_part_string("ppt/_rels/presentation.xml.rels") else {
        return fallback_order(pkg);
    };
    let pres_xml = pkg.get_part_string("ppt/presentation.xml");
    let order = resolve_slide_order(pres_xml.as_deref(), &rels_xml);
    if order.is_empty() {
        fallback_order(pkg)
    } else {
        order
    }
}

// Slide *display* order is defined by `<p:sldIdLst>` in presentation.xml, where each
// `<p:sldId>` carries a relationship id. The .rels part maps that id to the slide part
// path. Relationship-id order is creation order, not display order, so a reordered deck
// would otherwise report the wrong slide numbers — only fall back to rId order when the
// sldIdLst is unavailable.
fn resolve_slide_order(presentation_xml: Option<&str>, rels_xml: &str) -> Vec<String> {
    let targets = slide_rel_targets(rels_xml);
    if targets.is_empty() {
        return vec![];
    }

    if let Some(pres) = presentation_xml {
        let ordered: Vec<String> = sldid_order(pres)
            .iter()
            .filter_map(|rid| targets.get(rid).cloned())
            .collect();
        if !ordered.is_empty() {
            return ordered;
        }
    }

    let mut by_id: Vec<(u32, String)> = targets
        .iter()
        .filter_map(|(rid, path)| Some((rel_id_num(rid)?, path.clone())))
        .collect();
    by_id.sort_by_key(|(n, _)| *n);
    by_id.into_iter().map(|(_, p)| p).collect()
}

fn rel_id_num(rid: &str) -> Option<u32> {
    rid.trim_start_matches("rId").parse().ok()
}

// Maps each slide relationship id (e.g. "rId2") to its resolved part path.
fn slide_rel_targets(rels_xml: &str) -> HashMap<String, String> {
    let Ok(root) = XmlParser::parse_str(rels_xml) else {
        return HashMap::new();
    };
    root.find_all("Relationship")
        .into_iter()
        .filter(|r| {
            r.attr("Type")
                .map(|t| t.contains("/slide") && !t.contains("Layout") && !t.contains("Master"))
                .unwrap_or(false)
        })
        .filter_map(|r| {
            let id = r.attr("Id")?.to_string();
            let target = r.attr("Target")?;
            let path = if let Some(stripped) = target.strip_prefix('/') {
                stripped.to_string()
            } else {
                format!("ppt/{target}")
            };
            Some((id, path))
        })
        .collect()
}

// Reads the ordered relationship ids from `<p:sldIdLst>`.
fn sldid_order(presentation_xml: &str) -> Vec<String> {
    let Ok(root) = XmlParser::parse_str(presentation_xml) else {
        return vec![];
    };
    let Some(lst) = root.find_descendant("sldIdLst") else {
        return vec![];
    };
    lst.find_all("sldId")
        .into_iter()
        .filter_map(|s| s.attr("r:id").map(str::to_string))
        .collect()
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
    // Numeric, not lexical: lexical order places slide10.xml before slide2.xml.
    paths.sort_by_key(|p| slide_path_num(p).unwrap_or(u32::MAX));
    paths
}

// Extracts N from "ppt/slides/slideN.xml".
fn slide_path_num(path: &str) -> Option<u32> {
    path.strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sldid_order_reads_relationship_ids_in_document_order() {
        let xml = r#"<p:presentation xmlns:p="p" xmlns:r="r">
            <p:sldIdLst>
                <p:sldId id="256" r:id="rId3"/>
                <p:sldId id="257" r:id="rId2"/>
                <p:sldId id="258" r:id="rId5"/>
            </p:sldIdLst>
        </p:presentation>"#;
        assert_eq!(sldid_order(xml), vec!["rId3", "rId2", "rId5"]);
    }

    #[test]
    fn sldid_order_empty_when_no_list() {
        assert!(sldid_order(r#"<p:presentation xmlns:p="p"></p:presentation>"#).is_empty());
        assert!(sldid_order("not xml").is_empty());
    }

    #[test]
    fn slide_rel_targets_maps_ids_and_resolves_paths() {
        let xml = r#"<Relationships xmlns="x">
            <Relationship Id="rId1" Type="http://x/slideLayout" Target="slideLayouts/sl1.xml"/>
            <Relationship Id="rId2" Type="http://x/slide" Target="slides/slide1.xml"/>
            <Relationship Id="rId3" Type="http://x/slide" Target="/ppt/slides/slide2.xml"/>
        </Relationships>"#;
        let m = slide_rel_targets(xml);
        assert_eq!(m.len(), 2); // slideLayout relationship excluded
        assert_eq!(m.get("rId2").unwrap(), "ppt/slides/slide1.xml");
        assert_eq!(m.get("rId3").unwrap(), "ppt/slides/slide2.xml");
    }

    #[test]
    fn resolve_slide_order_follows_sldidlst_not_relationship_id_order() {
        // Relationship ids are NOT in display order: the slide shown first is rId3.
        let rels = r#"<Relationships xmlns="x">
            <Relationship Id="rId2" Type="t/slide" Target="slides/slide1.xml"/>
            <Relationship Id="rId3" Type="t/slide" Target="slides/slide2.xml"/>
        </Relationships>"#;
        let pres = r#"<p:presentation xmlns:p="p" xmlns:r="r">
            <p:sldIdLst>
                <p:sldId r:id="rId3"/>
                <p:sldId r:id="rId2"/>
            </p:sldIdLst>
        </p:presentation>"#;
        assert_eq!(
            resolve_slide_order(Some(pres), rels),
            vec!["ppt/slides/slide2.xml", "ppt/slides/slide1.xml"],
        );
    }

    #[test]
    fn resolve_slide_order_falls_back_to_numeric_relationship_id_order() {
        // No presentation.xml: order by ascending rId number, not lexically.
        let rels = r#"<Relationships xmlns="x">
            <Relationship Id="rId2" Type="t/slide" Target="slides/slide2.xml"/>
            <Relationship Id="rId10" Type="t/slide" Target="slides/slide10.xml"/>
            <Relationship Id="rId1" Type="t/slide" Target="slides/slide1.xml"/>
        </Relationships>"#;
        assert_eq!(
            resolve_slide_order(None, rels),
            vec![
                "ppt/slides/slide1.xml",
                "ppt/slides/slide2.xml",
                "ppt/slides/slide10.xml",
            ],
        );
    }

    #[test]
    fn resolve_slide_order_empty_without_slide_relationships() {
        let rels = r#"<Relationships xmlns="x">
            <Relationship Id="rId1" Type="t/slideMaster" Target="slideMasters/sm1.xml"/>
        </Relationships>"#;
        assert!(resolve_slide_order(None, rels).is_empty());
    }

    #[test]
    fn slide_path_num_orders_numerically() {
        let mut paths = vec![
            "ppt/slides/slide10.xml".to_string(),
            "ppt/slides/slide2.xml".to_string(),
            "ppt/slides/slide1.xml".to_string(),
        ];
        paths.sort_by_key(|p| slide_path_num(p).unwrap_or(u32::MAX));
        assert_eq!(
            paths,
            vec![
                "ppt/slides/slide1.xml",
                "ppt/slides/slide2.xml",
                "ppt/slides/slide10.xml",
            ],
        );
    }

    #[test]
    fn parse_images_extracts_picture_geometry() {
        let xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:pic>
                    <p:nvPicPr><p:cNvPr id="4" name="Logo"/></p:nvPicPr>
                    <p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></a:xfrm></p:spPr>
                </p:pic>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let imgs = parse_images(xml);
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].name, "Logo");
        assert_eq!(imgs[0].kind, ElementKind::Image);
        assert_eq!(imgs[0].rect.x, 100);
        assert_eq!(imgs[0].rect.y, 200);
        assert_eq!(imgs[0].rect.w, 300);
        assert_eq!(imgs[0].rect.h, 400);
    }

    #[test]
    fn parse_images_skips_zero_sized_pictures() {
        let xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:pic>
                    <p:nvPicPr><p:cNvPr name="Empty"/></p:nvPicPr>
                    <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm></p:spPr>
                </p:pic>
            </p:spTree></p:cSld>
        </p:sld>"#;
        assert!(parse_images(xml).is_empty());
    }

    #[test]
    fn parse_font_families_skips_theme_references() {
        // The first run uses a theme reference ("+mn-lt"); only the explicit
        // typeface should be reported.
        let xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:cNvPr name="Body 1"/></p:nvSpPr>
                    <p:txBody><a:p>
                        <a:r><a:rPr><a:latin typeface="+mn-lt"/></a:rPr></a:r>
                        <a:r><a:rPr><a:latin typeface="Calibri"/></a:rPr></a:r>
                    </a:p></p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let fams = parse_font_families(xml);
        assert_eq!(fams.get("Body 1").map(String::as_str), Some("Calibri"));
    }
}
