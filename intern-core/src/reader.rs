use std::collections::{HashMap, HashSet};

use ppt_rs::opc::Package;
use ppt_rs::oxml::slide::TextRun;
use ppt_rs::oxml::{SlideParser, XmlParser};

use crate::error::Error;
use crate::model::{
    DEFAULT_SLIDE_HEIGHT_EMU, DEFAULT_SLIDE_WIDTH_EMU, ElementKind, Presentation, Rect, SlideData,
    SlideElement,
};

pub fn read_presentation(path: &str) -> Result<Presentation, Error> {
    let pkg = Package::open(path).map_err(|e| Error::Open {
        path: path.to_string(),
        message: e.to_string(),
    })?;
    let paths = slide_order(&pkg);
    let slides = paths
        .iter()
        .enumerate()
        .map(|(idx, slide_path)| {
            let xml = pkg
                .get_part_string(slide_path)
                .ok_or_else(|| Error::SlideMissing(slide_path.clone()))?;
            parse_slide(idx, &xml)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (slide_width, slide_height) = slide_size(&pkg);
    Ok(Presentation {
        slides,
        slide_width,
        slide_height,
    })
}

// Reads the deck's slide dimensions from `<p:sldSz>` in presentation.xml, falling
// back to the standard 16:9 widescreen size when the element is absent or invalid.
fn slide_size(pkg: &Package) -> (i64, i64) {
    pkg.get_part_string("ppt/presentation.xml")
        .as_deref()
        .and_then(parse_slide_size)
        .unwrap_or((DEFAULT_SLIDE_WIDTH_EMU, DEFAULT_SLIDE_HEIGHT_EMU))
}

fn parse_slide_size(presentation_xml: &str) -> Option<(i64, i64)> {
    let root = XmlParser::parse_str(presentation_xml).ok()?;
    let sz = root.find_descendant("sldSz")?;
    let cx: i64 = sz.attr("cx")?.parse().ok()?;
    let cy: i64 = sz.attr("cy")?.parse().ok()?;
    (cx > 0 && cy > 0).then_some((cx, cy))
}

fn parse_slide(index: usize, xml: &str) -> Result<SlideData, Error> {
    let parsed = SlideParser::parse(xml).map_err(|e| Error::ParseSlide(e.to_string()))?;
    let families = parse_font_families(xml);
    let textboxes = textbox_names(xml);
    let mut elements: Vec<SlideElement> = parsed
        .shapes
        .into_iter()
        .filter(|s| s.width > 0 && s.height > 0)
        .map(|s| {
            let kind = if s.is_title {
                ElementKind::Title
            } else if s.is_body {
                ElementKind::Body
            } else if textboxes.contains(&s.name) {
                ElementKind::TextBox
            } else {
                ElementKind::Autoshape
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

// Returns the names of every `<p:sp>` that is a genuine text box, i.e. carries
// `txBox="1"` on its `<p:cNvSpPr>`. Shapes absent from this set are autoshapes.
fn textbox_names(xml: &str) -> HashSet<String> {
    let Ok(root) = XmlParser::parse_str(xml) else {
        return HashSet::new();
    };
    let Some(sp_tree) = root.find_descendant("spTree") else {
        return HashSet::new();
    };
    sp_tree
        .find_all_descendants("sp")
        .into_iter()
        .filter_map(|sp| {
            let name = sp.find_descendant("cNvPr")?.attr("name")?.to_string();
            let is_textbox = sp
                .find_descendant("cNvSpPr")
                .and_then(|e| e.attr("txBox"))
                .is_some_and(|v| v == "1");
            is_textbox.then_some(name)
        })
        .collect()
}

// Parses top-level `<p:pic>` images. Pictures nested inside a `<p:grpSp>` are
// skipped: their `<a:off>` is in the group's child coordinate space and `intern`
// does not yet apply group transforms, so their slide position is unknown.
fn parse_images(xml: &str) -> Vec<SlideElement> {
    let Ok(root) = XmlParser::parse_str(xml) else {
        return vec![];
    };
    let Some(sp_tree) = root.find_descendant("spTree") else {
        return vec![];
    };

    sp_tree
        .find_all("pic")
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
// would otherwise report the wrong slide numbers - only fall back to rId order when the
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

/// An `intern: disable` directive parsed from a slide's speaker notes.
#[derive(Debug, PartialEq, Eq)]
pub enum SlideExclusion {
    /// `intern: disable` - the whole slide is excluded from every rule.
    All,
    /// `intern: disable RULE_A,RULE_B` - excluded only from the named rules.
    Rules(Vec<String>),
}

/// Reads each slide's `intern: disable` speaker-note directive, keyed by 0-based
/// slide index. Slides absent from the map carry no directive.
pub fn slide_exclusions(path: &str) -> Result<HashMap<usize, SlideExclusion>, Error> {
    let pkg = Package::open(path).map_err(|e| Error::Open {
        path: path.to_string(),
        message: e.to_string(),
    })?;
    let exclusions = slide_order(&pkg)
        .iter()
        .enumerate()
        .filter_map(|(idx, slide_path)| {
            parse_exclusion(&slide_notes(&pkg, slide_path)).map(|ex| (idx, ex))
        })
        .collect();
    Ok(exclusions)
}

// Reads the speaker-note text for a slide, or an empty string if it has none.
fn slide_notes(pkg: &Package, slide_path: &str) -> String {
    let Some(rels_path) = rels_path_for(slide_path) else {
        return String::new();
    };
    let Some(rels_xml) = pkg.get_part_string(&rels_path) else {
        return String::new();
    };
    let Some(notes_path) = notes_target(&rels_xml) else {
        return String::new();
    };
    match pkg.get_part_string(&notes_path) {
        Some(notes_xml) => extract_notes_text(&notes_xml),
        None => String::new(),
    }
}

// Maps "ppt/slides/slideN.xml" to its relationship part
// "ppt/slides/_rels/slideN.xml.rels".
fn rels_path_for(slide_path: &str) -> Option<String> {
    let (dir, file) = slide_path.rsplit_once('/')?;
    Some(format!("{dir}/_rels/{file}.rels"))
}

// Finds the notesSlide relationship target in a slide's .rels XML, resolved to a
// package-absolute part path.
fn notes_target(rels_xml: &str) -> Option<String> {
    let root = XmlParser::parse_str(rels_xml).ok()?;
    let target = root
        .find_all("Relationship")
        .into_iter()
        .filter(|r| {
            r.attr("Type")
                .map(|t| t.contains("notesSlide"))
                .unwrap_or(false)
        })
        .find_map(|r| r.attr("Target"))?;
    Some(resolve_part_path("ppt/slides", target))
}

// Resolves a relationship target (which may contain `../`) against the part's
// directory into a package-absolute path.
fn resolve_part_path(base_dir: &str, target: &str) -> String {
    if let Some(stripped) = target.strip_prefix('/') {
        return stripped.to_string();
    }
    let mut parts: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

// Concatenates the text of every paragraph in a notesSlide part, one line each.
fn extract_notes_text(notes_xml: &str) -> String {
    let Ok(root) = XmlParser::parse_str(notes_xml) else {
        return String::new();
    };
    root.find_all_descendants("p")
        .into_iter()
        .map(|p| {
            p.find_all_descendants("t")
                .into_iter()
                .map(|t| t.text.as_str())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// Parses every `intern: disable` line in a slide's notes into one directive. A bare
// line disables the whole slide; lines with rule ids accumulate into a rule list.
fn parse_exclusion(notes: &str) -> Option<SlideExclusion> {
    let mut rules: Vec<String> = Vec::new();
    let mut seen = false;
    for line in notes.lines() {
        let Some(rest) = disable_directive_rest(line) else {
            continue;
        };
        seen = true;
        if rest.is_empty() {
            return Some(SlideExclusion::All);
        }
        for id in rest.split(',') {
            let id = id.trim();
            if !id.is_empty() {
                rules.push(id.to_ascii_uppercase());
            }
        }
    }
    seen.then_some(SlideExclusion::Rules(rules))
}

// If `line` is an `intern: disable` directive (case-insensitive, optional space
// after the colon), returns the trimmed text after the keyword - empty for a bare
// whole-slide directive.
fn disable_directive_rest(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let prefix = ["intern: disable", "intern:disable"]
        .into_iter()
        .find(|p| lower.starts_with(p))?;
    let rest = &trimmed[prefix.len()..];
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else {
        None
    }
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
    fn parse_images_skips_pictures_inside_groups() {
        // A picture nested in a group carries child-space coordinates; until group
        // transforms are applied it must not be read as a slide-level image.
        let xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:pic>
                        <p:nvPicPr><p:cNvPr name="Nested"/></p:nvPicPr>
                        <p:spPr><a:xfrm><a:off x="1" y="2"/><a:ext cx="3" cy="4"/></a:xfrm></p:spPr>
                    </p:pic>
                </p:grpSp>
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

    #[test]
    fn textbox_names_distinguishes_text_boxes_from_autoshapes() {
        let xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:cNvPr name="Real Text Box"/><p:cNvSpPr txBox="1"/></p:nvSpPr>
                </p:sp>
                <p:sp>
                    <p:nvSpPr><p:cNvPr name="Rectangle 3"/><p:cNvSpPr/></p:nvSpPr>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let names = textbox_names(xml);
        assert!(names.contains("Real Text Box"));
        assert!(!names.contains("Rectangle 3"));
    }

    #[test]
    fn parse_slide_size_reads_sldsz_dimensions() {
        let xml = r#"<p:presentation xmlns:p="p">
            <p:sldSz cx="12192000" cy="6858000" type="screen16x9"/>
        </p:presentation>"#;
        assert_eq!(parse_slide_size(xml), Some((12_192_000, 6_858_000)));
    }

    #[test]
    fn parse_slide_size_none_when_missing_or_invalid() {
        assert_eq!(parse_slide_size(r#"<p:presentation xmlns:p="p"/>"#), None);
        let zero = r#"<p:presentation xmlns:p="p"><p:sldSz cx="0" cy="0"/></p:presentation>"#;
        assert_eq!(parse_slide_size(zero), None);
    }

    #[test]
    fn rels_path_for_maps_slide_to_its_rels() {
        assert_eq!(
            rels_path_for("ppt/slides/slide3.xml").as_deref(),
            Some("ppt/slides/_rels/slide3.xml.rels"),
        );
    }

    #[test]
    fn resolve_part_path_collapses_parent_segments() {
        assert_eq!(
            resolve_part_path("ppt/slides", "../notesSlides/notesSlide3.xml"),
            "ppt/notesSlides/notesSlide3.xml",
        );
    }

    #[test]
    fn resolve_part_path_strips_leading_slash() {
        assert_eq!(
            resolve_part_path("ppt/slides", "/ppt/notesSlides/n1.xml"),
            "ppt/notesSlides/n1.xml",
        );
    }

    #[test]
    fn notes_target_finds_the_notes_relationship() {
        let xml = r#"<Relationships xmlns="x">
            <Relationship Id="rId1" Type="http://x/slideLayout" Target="../slideLayouts/sl1.xml"/>
            <Relationship Id="rId2" Type="http://x/notesSlide" Target="../notesSlides/notesSlide1.xml"/>
        </Relationships>"#;
        assert_eq!(
            notes_target(xml).as_deref(),
            Some("ppt/notesSlides/notesSlide1.xml"),
        );
    }

    #[test]
    fn extract_notes_text_joins_paragraphs() {
        let xml = r#"<p:notes xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree><p:sp><p:txBody>
                <a:p><a:r><a:t>first line</a:t></a:r></a:p>
                <a:p><a:r><a:t>intern: disable</a:t></a:r></a:p>
            </p:txBody></p:sp></p:spTree></p:cSld>
        </p:notes>"#;
        assert_eq!(extract_notes_text(xml), "first line\nintern: disable");
    }

    #[test]
    fn parse_exclusion_bare_marker_disables_whole_slide() {
        assert_eq!(
            parse_exclusion("speaker notes\n  intern: disable  \nmore"),
            Some(SlideExclusion::All),
        );
        assert_eq!(
            parse_exclusion("Intern: Disable"),
            Some(SlideExclusion::All)
        );
    }

    #[test]
    fn parse_exclusion_collects_named_rules() {
        assert_eq!(
            parse_exclusion("intern: disable TITLE_Y, grid_row_top"),
            Some(SlideExclusion::Rules(vec![
                "TITLE_Y".to_string(),
                "GRID_ROW_TOP".to_string(),
            ])),
        );
    }

    #[test]
    fn parse_exclusion_bare_line_wins_over_named() {
        // A whole-slide directive anywhere in the notes overrides rule lists.
        assert_eq!(
            parse_exclusion("intern: disable TITLE_Y\nintern: disable"),
            Some(SlideExclusion::All),
        );
    }

    #[test]
    fn parse_exclusion_none_without_a_directive() {
        assert_eq!(parse_exclusion(""), None);
        assert_eq!(parse_exclusion("just ordinary speaker notes"), None);
        // "disable" must be its own keyword, not a prefix of another word.
        assert_eq!(parse_exclusion("intern: disabled forever"), None);
    }
}
