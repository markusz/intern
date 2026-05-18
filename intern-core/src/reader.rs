use std::collections::HashMap;

// ppt-rs `Package` handles OPC/zip unpacking. Its `SlideParser` is excluded:
// it uses a flat `find_all("sp")` that misses shapes inside `<p:grpSp>` groups
// and applies no group-transform math, so grouped shapes get wrong coordinates.
use ppt_rs::opc::Package;

use crate::error::Error;
use crate::model::{
    DEFAULT_SLIDE_HEIGHT_EMU, DEFAULT_SLIDE_WIDTH_EMU, ElementKind, Paragraph, ParagraphKind,
    Presentation, Rect, SlideData, SlideElement,
};
use crate::xml;

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
            let slide_xml = pkg
                .get_part_string(slide_path)
                .ok_or_else(|| Error::SlideMissing(slide_path.clone()))?;
            parse_slide(idx, &slide_xml)
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
    let root = xml::Element::parse(presentation_xml).ok()?;
    let sz = root.find_descendant("sldSz")?;
    let cx: i64 = sz.attr("cx")?.parse().ok()?;
    let cy: i64 = sz.attr("cy")?.parse().ok()?;
    (cx > 0 && cy > 0).then_some((cx, cy))
}

fn parse_slide(index: usize, slide_xml: &str) -> Result<SlideData, Error> {
    let root = xml::Element::parse(slide_xml).map_err(|e| Error::ParseSlide(e.to_string()))?;
    let Some(sp_tree) = root.find_descendant("spTree") else {
        return Ok(SlideData {
            index,
            elements: vec![],
            units: vec![],
        });
    };
    Ok(SlideData {
        index,
        elements: walk_shapes(sp_tree, &[]),
        units: walk_top_level_units(sp_tree),
    })
}

fn parse_sp(sp: &xml::Element) -> Option<SlideElement> {
    let cnv_pr = sp.find_descendant("cNvPr")?;
    let id: u32 = cnv_pr.attr("id").and_then(|s| s.parse().ok()).or_else(|| {
        let name = cnv_pr.attr("name").unwrap_or("?");
        eprintln!("intern: shape '{name}' has no cNvPr id - skipped");
        None
    })?;
    let name = cnv_pr.attr("name").unwrap_or("").to_string();
    let rect = parse_rect(sp)?;
    if rect.w <= 0 || rect.h <= 0 {
        return None;
    }
    let kind = classify_sp(sp);
    let (paragraphs, font_size, font_family, text_color) = parse_text_body(sp, &kind);
    Some(SlideElement {
        id,
        name,
        kind,
        rect,
        font_size,
        font_family,
        text_color,
        paragraphs,
    })
}

fn classify_sp(sp: &xml::Element) -> ElementKind {
    if let Some(ph) = sp.find_descendant("ph") {
        let ph_type = ph.attr("type").unwrap_or("");
        if ph_type == "title" || ph_type == "ctrTitle" {
            return ElementKind::Title;
        }
        if ph_type.is_empty() || ph_type == "body" || ph_type == "subTitle" || ph_type == "obj" {
            return ElementKind::Body;
        }
    }
    // Name-based fallback matching the ppt-rs generator convention.
    if let Some(name) = sp.find_descendant("cNvPr").and_then(|e| e.attr("name")) {
        let lower = name.to_lowercase();
        if lower == "title" || lower.contains("title") {
            return ElementKind::Title;
        }
        if lower == "content" || lower.contains("content") {
            return ElementKind::Body;
        }
    }
    if sp
        .find_descendant("cNvSpPr")
        .and_then(|e| e.attr("txBox"))
        .is_some_and(|v| v == "1")
    {
        return ElementKind::TextBox;
    }
    ElementKind::Autoshape
}

fn parse_rect(node: &xml::Element) -> Option<Rect> {
    let xfrm = node.find_descendant("xfrm")?;
    let off = xfrm.find("off")?;
    let ext = xfrm.find("ext")?;
    let x: i64 = off.attr("x")?.parse().ok()?;
    let y: i64 = off.attr("y")?.parse().ok()?;
    let w: i64 = ext.attr("cx")?.parse().ok()?;
    let h: i64 = ext.attr("cy")?.parse().ok()?;
    Some(Rect { x, y, w, h })
}

fn parse_text_body(
    sp: &xml::Element,
    element_kind: &ElementKind,
) -> (Vec<Paragraph>, Option<u32>, Option<String>, Option<String>) {
    let Some(tx_body) = sp.find("txBody") else {
        return (vec![], None, None, None);
    };
    let mut paragraphs = Vec::new();
    let mut font_size: Option<u32> = None;
    let mut font_family: Option<String> = None;
    let mut color_counts: HashMap<String, usize> = HashMap::new();

    // Body placeholders inherit bullet formatting from the slide layout; all other
    // element kinds (TextBox, Autoshape) do not have bullets unless explicitly set.
    let inherited_kind = match element_kind {
        ElementKind::Body => ParagraphKind::Bullet,
        _ => ParagraphKind::Plain,
    };

    for para in tx_body.find_all("p") {
        let text = collect_para_text(para, &mut font_size, &mut font_family, &mut color_counts);
        if !text.trim().is_empty() {
            let kind = detect_bullet_kind(para, inherited_kind.clone());
            paragraphs.push(Paragraph { text, kind });
        }
    }

    let text_color = color_counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(c, _)| c);
    (paragraphs, font_size, font_family, text_color)
}

fn detect_bullet_kind(para: &xml::Element, default: ParagraphKind) -> ParagraphKind {
    let Some(ppr) = para.find("pPr") else {
        return default;
    };
    if ppr.find("buChar").is_some() || ppr.find("buAutoNum").is_some() {
        return ParagraphKind::Bullet;
    }
    if ppr.find("buNone").is_some() {
        return ParagraphKind::Plain;
    }
    default
}

fn collect_para_text(
    para: &xml::Element,
    font_size: &mut Option<u32>,
    font_family: &mut Option<String>,
    color_counts: &mut HashMap<String, usize>,
) -> String {
    let mut text = String::new();
    for run in para.find_all("r") {
        if let Some(t) = run.find("t") {
            text.push_str(&t.text);
        }
        let Some(rpr) = run.find("rPr") else {
            continue;
        };
        if font_size.is_none() {
            *font_size = rpr.attr("sz").and_then(|v| v.parse().ok());
        }
        if font_family.is_none() {
            *font_family = rpr
                .find("latin")
                .and_then(|l| l.attr("typeface"))
                .filter(|t| !t.starts_with('+'))
                .map(str::to_string);
        }
        if let Some(color) = rpr.find_descendant("srgbClr").and_then(|c| c.attr("val")) {
            *color_counts.entry(color.to_string()).or_default() += 1;
        }
    }
    text
}

fn parse_pic(pic: &xml::Element) -> Option<SlideElement> {
    let cnv_pr = pic.find_descendant("cNvPr");
    let id: u32 = cnv_pr
        .and_then(|e| e.attr("id"))
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            let name = cnv_pr.and_then(|e| e.attr("name")).unwrap_or("?");
            eprintln!("intern: picture '{name}' has no cNvPr id - skipped");
            None
        })?;
    let name = cnv_pr
        .and_then(|e| e.attr("name"))
        .unwrap_or("Picture")
        .to_string();
    let rect = parse_rect(pic)?;
    if rect.w <= 0 || rect.h <= 0 {
        return None;
    }
    Some(SlideElement {
        id,
        name,
        kind: ElementKind::Image,
        rect,
        font_size: None,
        font_family: None,
        text_color: None,
        paragraphs: vec![],
    })
}

#[derive(Clone, Copy)]
struct GroupTransform {
    off_x: i64,
    off_y: i64,
    ext_cx: i64,
    ext_cy: i64,
    ch_off_x: i64,
    ch_off_y: i64,
    ch_ext_cx: i64,
    ch_ext_cy: i64,
}

impl GroupTransform {
    fn apply(self, rect: Rect) -> Rect {
        let sx = self.ext_cx as f64 / self.ch_ext_cx as f64;
        let sy = self.ext_cy as f64 / self.ch_ext_cy as f64;
        Rect {
            x: (self.off_x as f64 + (rect.x - self.ch_off_x) as f64 * sx).round() as i64,
            y: (self.off_y as f64 + (rect.y - self.ch_off_y) as f64 * sy).round() as i64,
            w: (rect.w as f64 * sx).round() as i64,
            h: (rect.h as f64 * sy).round() as i64,
        }
    }
}

fn walk_shapes(node: &xml::Element, transforms: &[GroupTransform]) -> Vec<SlideElement> {
    let mut elements = Vec::new();
    for child in &node.children {
        match child.tag.as_str() {
            "sp" => {
                if let Some(mut el) = parse_sp(child) {
                    el.rect = apply_group_transforms(el.rect, transforms);
                    elements.push(el);
                }
            }
            "pic" => {
                if let Some(mut el) = parse_pic(child) {
                    el.rect = apply_group_transforms(el.rect, transforms);
                    elements.push(el);
                }
            }
            "grpSp" => elements.extend(enter_group(child, transforms)),
            _ => {}
        }
    }
    elements
}

fn enter_group(grp: &xml::Element, parent_transforms: &[GroupTransform]) -> Vec<SlideElement> {
    let Some(transform) = parse_group_transform(grp) else {
        return vec![];
    };
    let mut transforms = parent_transforms.to_vec();
    transforms.push(transform);
    walk_shapes(grp, &transforms)
}

fn parse_group_transform(grp: &xml::Element) -> Option<GroupTransform> {
    let xfrm = grp.find("grpSpPr")?.find("xfrm")?;
    let off_x: i64 = xfrm.find("off")?.attr("x")?.parse().ok()?;
    let off_y: i64 = xfrm.find("off")?.attr("y")?.parse().ok()?;
    let ext_cx: i64 = xfrm.find("ext")?.attr("cx")?.parse().ok()?;
    let ext_cy: i64 = xfrm.find("ext")?.attr("cy")?.parse().ok()?;
    let ch_off_x: i64 = xfrm.find("chOff")?.attr("x")?.parse().ok()?;
    let ch_off_y: i64 = xfrm.find("chOff")?.attr("y")?.parse().ok()?;
    let ch_ext_cx: i64 = xfrm.find("chExt")?.attr("cx")?.parse().ok()?;
    let ch_ext_cy: i64 = xfrm.find("chExt")?.attr("cy")?.parse().ok()?;
    // Division by zero guard: a degenerate group with no child extent is skipped.
    if ch_ext_cx == 0 || ch_ext_cy == 0 {
        return None;
    }
    Some(GroupTransform {
        off_x,
        off_y,
        ext_cx,
        ext_cy,
        ch_off_x,
        ch_off_y,
        ch_ext_cx,
        ch_ext_cy,
    })
}

// Transforms a rect from child-space to slide-space by applying each group
// transform. Transforms are ordered outermost-first; applying in reverse gives
// innermost-first traversal (child -> slide direction).
fn apply_group_transforms(rect: Rect, transforms: &[GroupTransform]) -> Rect {
    transforms.iter().rev().fold(rect, |r, t| t.apply(r))
}

/// Top-level positioning units for a slide: non-grouped shapes plus one Group
/// element per top-level `<p:grpSp>` (holding the group's bounding rect).
/// Only walks direct children of `node`; does not recurse into groups.
fn walk_top_level_units(node: &xml::Element) -> Vec<SlideElement> {
    let mut units = Vec::new();
    for child in &node.children {
        match child.tag.as_str() {
            "sp" => {
                if let Some(el) = parse_sp(child) {
                    units.push(el);
                }
            }
            "pic" => {
                if let Some(el) = parse_pic(child) {
                    units.push(el);
                }
            }
            "grpSp" => {
                if let Some(el) = parse_group_element(child) {
                    units.push(el);
                }
            }
            _ => {}
        }
    }
    units
}

/// Parses a `<p:grpSp>` into a single Group element whose rect is the group's
/// bounding box in slide-space. Returns `None` for degenerate groups (zero extent).
fn parse_group_element(grp: &xml::Element) -> Option<SlideElement> {
    let cnv_pr = grp.find("nvGrpSpPr")?.find("cNvPr")?;
    let id: u32 = cnv_pr.attr("id")?.parse().ok()?;
    let name = cnv_pr.attr("name").unwrap_or("").to_string();
    let xfrm = grp.find("grpSpPr")?.find("xfrm")?;
    let x: i64 = xfrm.find("off")?.attr("x")?.parse().ok()?;
    let y: i64 = xfrm.find("off")?.attr("y")?.parse().ok()?;
    let w: i64 = xfrm.find("ext")?.attr("cx")?.parse().ok()?;
    let h: i64 = xfrm.find("ext")?.attr("cy")?.parse().ok()?;
    if w == 0 || h == 0 {
        return None;
    }
    Some(SlideElement {
        id,
        name,
        kind: ElementKind::Group,
        rect: Rect { x, y, w, h },
        font_size: None,
        font_family: None,
        text_color: None,
        paragraphs: vec![],
    })
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
pub(crate) fn resolve_slide_order(presentation_xml: Option<&str>, rels_xml: &str) -> Vec<String> {
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
pub(crate) fn slide_rel_targets(rels_xml: &str) -> HashMap<String, String> {
    let Ok(root) = xml::Element::parse(rels_xml) else {
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
pub(crate) fn sldid_order(presentation_xml: &str) -> Vec<String> {
    let Ok(root) = xml::Element::parse(presentation_xml) else {
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

/// All `intern: disable` directives parsed from one slide's speaker notes.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct SlideExclusion {
    /// `None` = no whole-slide directive.
    /// `Some([])` = bare `intern: disable` (all rules).
    /// `Some(ids)` = `intern: disable RULE_A RULE_B` (named rules only).
    pub slide: Option<Vec<String>>,
    /// Per-element directives from `intern: disable(id) [RULES]`.
    pub elements: Vec<ElementExclusion>,
}

/// A per-element `intern: disable(id) [RULES]` directive.
#[derive(Debug, PartialEq, Eq)]
pub struct ElementExclusion {
    pub element_id: u32,
    /// Empty = all rules suppressed for this element.
    pub rules: Vec<String>,
}

impl SlideExclusion {
    /// True when a bare `intern: disable` suppresses the whole slide.
    pub fn suppresses_slide(&self) -> bool {
        self.slide.as_deref() == Some(&[])
    }

    /// True when `rule_id` is suppressed for the whole slide.
    pub fn suppresses_rule_for_slide(&self, rule_id: &str) -> bool {
        match &self.slide {
            None => false,
            Some(ids) if ids.is_empty() => true,
            Some(ids) => ids.iter().any(|id| id == rule_id),
        }
    }

    /// True when `rule_id` is suppressed for a specific element on this slide.
    pub fn suppresses_rule_for_element(&self, element_id: u32, rule_id: &str) -> bool {
        self.elements.iter().any(|ex| {
            ex.element_id == element_id
                && (ex.rules.is_empty() || ex.rules.iter().any(|id| id == rule_id))
        })
    }
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
pub(crate) fn rels_path_for(slide_path: &str) -> Option<String> {
    let (dir, file) = slide_path.rsplit_once('/')?;
    Some(format!("{dir}/_rels/{file}.rels"))
}

// Finds the notesSlide relationship target in a slide's .rels XML, resolved to a
// package-absolute part path.
pub(crate) fn notes_target(rels_xml: &str) -> Option<String> {
    let root = xml::Element::parse(rels_xml).ok()?;
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
pub(crate) fn resolve_part_path(base_dir: &str, target: &str) -> String {
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
    let Ok(root) = xml::Element::parse(notes_xml) else {
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

// Parses every `intern: disable` line in a slide's notes into one directive.
// A bare line disables the whole slide; a line with rule ids accumulates them;
// a line with `(id)` syntax records a per-element suppression.
fn parse_exclusion(notes: &str) -> Option<SlideExclusion> {
    let mut slide_all = false;
    let mut slide_rules: Vec<String> = Vec::new();
    let mut elements: Vec<ElementExclusion> = Vec::new();

    for line in notes.lines() {
        let Some(rest) = disable_directive_rest(line) else {
            continue;
        };
        if rest.is_empty() {
            slide_all = true;
            continue;
        }
        if let Some(elem) = parse_element_exclusion(rest) {
            elements.push(elem);
            continue;
        }
        if !slide_all {
            for id in rest.split([',', ' ']) {
                let id = id.trim().to_ascii_uppercase();
                if !id.is_empty() {
                    slide_rules.push(id);
                }
            }
        }
    }

    let slide = if slide_all {
        Some(vec![])
    } else if !slide_rules.is_empty() {
        Some(slide_rules)
    } else {
        None
    };

    if slide.is_some() || !elements.is_empty() {
        Some(SlideExclusion { slide, elements })
    } else {
        None
    }
}

// Parses `(id) [RULE_A RULE_B]` from the rest of a disable directive line.
fn parse_element_exclusion(rest: &str) -> Option<ElementExclusion> {
    let inner = rest.strip_prefix('(')?;
    let (id_str, rule_rest) = inner.split_once(')')?;
    let element_id = id_str.trim().parse::<u32>().ok()?;
    let rules: Vec<String> = rule_rest
        .split([',', ' '])
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    Some(ElementExclusion { element_id, rules })
}

// If `line` is an `intern: disable` directive (case-insensitive, optional space
// after the colon), returns the text after the keyword:
//   - empty string for a bare whole-slide directive
//   - leading whitespace trimmed for `intern: disable RULE_A`
//   - `(id)...` untouched for element-level `intern: disable(42) RULE_A`
fn disable_directive_rest(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let prefix = ["intern: disable", "intern:disable"]
        .into_iter()
        .find(|p| lower.starts_with(p))?;
    let rest = &trimmed[prefix.len()..];
    if rest.is_empty() || rest.starts_with(char::is_whitespace) {
        Some(rest.trim())
    } else if rest.starts_with('(') {
        Some(rest)
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
    fn parse_pic_extracts_picture_geometry() {
        let pic_xml = r#"<p:pic xmlns:p="p" xmlns:a="a">
            <p:nvPicPr><p:cNvPr id="4" name="Logo"/></p:nvPicPr>
            <p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="400"/></a:xfrm></p:spPr>
        </p:pic>"#;
        let pic = xml::Element::parse(pic_xml).unwrap();
        let el = parse_pic(&pic).unwrap();
        assert_eq!(el.name, "Logo");
        assert_eq!(el.kind, ElementKind::Image);
        assert_eq!(el.rect.x, 100);
        assert_eq!(el.rect.y, 200);
        assert_eq!(el.rect.w, 300);
        assert_eq!(el.rect.h, 400);
    }

    #[test]
    fn parse_pic_skips_zero_sized_pictures() {
        let pic_xml = r#"<p:pic xmlns:p="p" xmlns:a="a">
            <p:nvPicPr><p:cNvPr id="1" name="Empty"/></p:nvPicPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm></p:spPr>
        </p:pic>"#;
        let pic = xml::Element::parse(pic_xml).unwrap();
        assert!(parse_pic(&pic).is_none());
    }

    #[test]
    fn walk_shapes_applies_group_translate_to_child_sp() {
        // Group at (10,10), identity scale; child sp at (5,5) -> slide (15,15).
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="10" y="10"/>
                        <a:ext cx="100" cy="100"/>
                        <a:chOff x="0" y="0"/>
                        <a:chExt cx="100" cy="100"/>
                    </a:xfrm></p:grpSpPr>
                    <p:sp>
                        <p:nvSpPr><p:cNvPr id="2" name="Shape1"/><p:cNvSpPr txBox="1"/></p:nvSpPr>
                        <p:spPr><a:xfrm><a:off x="5" y="5"/><a:ext cx="20" cy="20"/></a:xfrm></p:spPr>
                        <p:txBody><a:p><a:r><a:t>hi</a:t></a:r></a:p></p:txBody>
                    </p:sp>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements.len(), 1);
        assert_eq!(slide.elements[0].rect.x, 15);
        assert_eq!(slide.elements[0].rect.y, 15);
    }

    #[test]
    fn walk_shapes_applies_group_transform_to_nested_pic() {
        // Group at (100,200), identity scale; pic at (50,50) -> slide (150,250).
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="100" y="200"/>
                        <a:ext cx="300" cy="400"/>
                        <a:chOff x="0" y="0"/>
                        <a:chExt cx="300" cy="400"/>
                    </a:xfrm></p:grpSpPr>
                    <p:pic>
                        <p:nvPicPr><p:cNvPr id="3" name="Logo"/></p:nvPicPr>
                        <p:spPr><a:xfrm><a:off x="50" y="50"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
                    </p:pic>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements.len(), 1);
        assert_eq!(slide.elements[0].kind, ElementKind::Image);
        assert_eq!(slide.elements[0].rect.x, 150);
        assert_eq!(slide.elements[0].rect.y, 250);
    }

    #[test]
    fn walk_shapes_composes_nested_group_transforms() {
        // Group A at (100,0), identity scale; Group B inside at (50,50), identity
        // scale; shape inside B at (10,10) -> A-child (60,60) -> slide (160,60).
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="100" y="0"/>
                        <a:ext cx="200" cy="200"/>
                        <a:chOff x="0" y="0"/>
                        <a:chExt cx="200" cy="200"/>
                    </a:xfrm></p:grpSpPr>
                    <p:grpSp>
                        <p:grpSpPr><a:xfrm>
                            <a:off x="50" y="50"/>
                            <a:ext cx="100" cy="100"/>
                            <a:chOff x="0" y="0"/>
                            <a:chExt cx="100" cy="100"/>
                        </a:xfrm></p:grpSpPr>
                        <p:sp>
                            <p:nvSpPr><p:cNvPr id="4" name="Inner"/><p:cNvSpPr txBox="1"/></p:nvSpPr>
                            <p:spPr><a:xfrm><a:off x="10" y="10"/><a:ext cx="20" cy="20"/></a:xfrm></p:spPr>
                            <p:txBody><a:p><a:r><a:t>nested</a:t></a:r></a:p></p:txBody>
                        </p:sp>
                    </p:grpSp>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements.len(), 1);
        assert_eq!(slide.elements[0].rect.x, 160);
        assert_eq!(slide.elements[0].rect.y, 60);
    }

    #[test]
    fn walk_shapes_skips_degenerate_group_with_zero_chext() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="0" y="0"/>
                        <a:ext cx="100" cy="100"/>
                        <a:chOff x="0" y="0"/>
                        <a:chExt cx="0" cy="0"/>
                    </a:xfrm></p:grpSpPr>
                    <p:pic>
                        <p:nvPicPr><p:cNvPr id="5" name="Image"/></p:nvPicPr>
                        <p:spPr><a:xfrm><a:off x="1" y="1"/><a:ext cx="10" cy="10"/></a:xfrm></p:spPr>
                    </p:pic>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert!(slide.elements.is_empty());
    }

    #[test]
    fn walk_top_level_units_returns_non_grouped_shapes_and_group_bbox() {
        // spTree has one plain sp and one grpSp. units should contain both,
        // but the grpSp should appear as a Group element with the group bbox,
        // not as its children.
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr>
                        <p:cNvPr id="1" name="Box1"/>
                        <p:cNvSpPr txBox="1"/>
                        <p:nvPr/>
                    </p:nvSpPr>
                    <p:spPr><a:xfrm><a:off x="100" y="200"/><a:ext cx="300" cy="100"/></a:xfrm></p:spPr>
                </p:sp>
                <p:grpSp>
                    <p:nvGrpSpPr>
                        <p:cNvPr id="10" name="Group 1"/>
                        <p:cNvGrpSpPr/>
                        <p:nvPr/>
                    </p:nvGrpSpPr>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="500" y="600"/>
                        <a:ext cx="800" cy="400"/>
                        <a:chOff x="500" y="600"/>
                        <a:chExt cx="800" cy="400"/>
                    </a:xfrm></p:grpSpPr>
                    <p:sp>
                        <p:nvSpPr>
                            <p:cNvPr id="11" name="InnerShape"/>
                            <p:cNvSpPr txBox="1"/>
                            <p:nvPr/>
                        </p:nvSpPr>
                        <p:spPr><a:xfrm><a:off x="510" y="610"/><a:ext cx="200" cy="100"/></a:xfrm></p:spPr>
                    </p:sp>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        // elements: sp + inner shape (group children are flattened)
        assert_eq!(slide.elements.len(), 2);
        // units: the plain sp + one Group element (not the inner shape)
        assert_eq!(slide.units.len(), 2);
        let group = slide.units.iter().find(|e| e.kind == ElementKind::Group);
        let group = group.expect("expected a Group unit");
        assert_eq!(group.id, 10);
        assert_eq!(group.rect.x, 500);
        assert_eq!(group.rect.y, 600);
        assert_eq!(group.rect.w, 800);
        assert_eq!(group.rect.h, 400);
        // The plain sp should appear in units with its own coords.
        let plain = slide
            .units
            .iter()
            .find(|e| e.kind != ElementKind::Group)
            .expect("expected a non-Group unit");
        assert_eq!(plain.rect.x, 100);
    }

    #[test]
    fn walk_top_level_units_skips_degenerate_group() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:grpSp>
                    <p:nvGrpSpPr>
                        <p:cNvPr id="20" name="Degen"/>
                        <p:cNvGrpSpPr/>
                        <p:nvPr/>
                    </p:nvGrpSpPr>
                    <p:grpSpPr><a:xfrm>
                        <a:off x="0" y="0"/>
                        <a:ext cx="0" cy="0"/>
                        <a:chOff x="0" y="0"/>
                        <a:chExt cx="100" cy="100"/>
                    </a:xfrm></p:grpSpPr>
                </p:grpSp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert!(slide.units.is_empty(), "degenerate group should be skipped");
    }

    #[test]
    fn classify_sp_title_by_ph_type() {
        let sp_xml = r#"<p:sp xmlns:p="p">
            <p:nvSpPr>
                <p:cNvPr id="6" name="Other"/>
                <p:nvPr><p:ph type="title"/></p:nvPr>
            </p:nvSpPr>
            <p:spPr><a:xfrm xmlns:a="a"><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></a:xfrm></p:spPr>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        assert_eq!(classify_sp(&sp), ElementKind::Title);
    }

    #[test]
    fn classify_sp_title_by_name_fallback() {
        let sp_xml = r#"<p:sp xmlns:p="p">
            <p:nvSpPr><p:cNvPr id="7" name="Title"/><p:cNvSpPr txBox="1"/></p:nvSpPr>
            <p:spPr><a:xfrm xmlns:a="a"><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></a:xfrm></p:spPr>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        assert_eq!(classify_sp(&sp), ElementKind::Title);
    }

    #[test]
    fn classify_sp_textbox_when_txbox_and_no_name_match() {
        let sp_xml = r#"<p:sp xmlns:p="p">
            <p:nvSpPr><p:cNvPr id="8" name="Text Box 5"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm xmlns:a="a"><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></a:xfrm></p:spPr>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        assert_eq!(classify_sp(&sp), ElementKind::TextBox);
    }

    #[test]
    fn classify_sp_autoshape_without_txbox() {
        let sp_xml = r#"<p:sp xmlns:p="p">
            <p:nvSpPr><p:cNvPr id="9" name="Rectangle 3"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
            <p:spPr><a:xfrm xmlns:a="a"><a:off x="0" y="0"/><a:ext cx="1" cy="1"/></a:xfrm></p:spPr>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        assert_eq!(classify_sp(&sp), ElementKind::Autoshape);
    }

    #[test]
    fn parse_sp_extracts_text_and_font_size() {
        let sp_xml = r#"<p:sp xmlns:p="p" xmlns:a="a">
            <p:nvSpPr><p:cNvPr id="10" name="Body 1"/><p:cNvSpPr txBox="1"/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
            <p:txBody>
                <a:p><a:r><a:rPr sz="1800"/><a:t>Hello</a:t></a:r></a:p>
                <a:p><a:r><a:t>World</a:t></a:r></a:p>
            </p:txBody>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        let el = parse_sp(&sp).unwrap();
        let texts: Vec<&str> = el.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["Hello", "World"]);
        assert_eq!(el.font_size, Some(1800));
    }

    #[test]
    fn body_placeholder_paragraphs_default_to_bullet_kind() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr>
                        <p:cNvPr id="11" name="Content"/>
                        <p:nvPr><p:ph type="body"/></p:nvPr>
                    </p:nvSpPr>
                    <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
                    <p:txBody><a:p><a:r><a:t>Point one</a:t></a:r></a:p></p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements[0].paragraphs[0].kind, ParagraphKind::Bullet);
    }

    #[test]
    fn autoshape_paragraphs_default_to_plain_kind() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:cNvPr id="12" name="Rect 1"/><p:cNvSpPr/></p:nvSpPr>
                    <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
                    <p:txBody><a:p><a:r><a:t>Label text</a:t></a:r></a:p></p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements[0].paragraphs[0].kind, ParagraphKind::Plain);
    }

    #[test]
    fn autoshape_paragraph_with_buchar_is_bullet_kind() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr><p:cNvPr id="13" name="Rect 1"/><p:cNvSpPr/></p:nvSpPr>
                    <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
                    <p:txBody>
                        <a:p>
                            <a:pPr><a:buChar char="•"/></a:pPr>
                            <a:r><a:t>Bullet item</a:t></a:r>
                        </a:p>
                    </p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements[0].paragraphs[0].kind, ParagraphKind::Bullet);
    }

    #[test]
    fn body_paragraph_with_bunone_is_plain_kind() {
        let slide_xml = r#"<p:sld xmlns:p="p" xmlns:a="a">
            <p:cSld><p:spTree>
                <p:sp>
                    <p:nvSpPr>
                        <p:cNvPr id="14" name="Content"/>
                        <p:nvPr><p:ph type="body"/></p:nvPr>
                    </p:nvSpPr>
                    <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
                    <p:txBody>
                        <a:p>
                            <a:pPr><a:buNone/></a:pPr>
                            <a:r><a:t>Not a bullet</a:t></a:r>
                        </a:p>
                    </p:txBody>
                </p:sp>
            </p:spTree></p:cSld>
        </p:sld>"#;
        let slide = parse_slide(0, slide_xml).unwrap();
        assert_eq!(slide.elements[0].paragraphs[0].kind, ParagraphKind::Plain);
    }

    #[test]
    fn parse_sp_skips_font_family_theme_references() {
        let sp_xml = r#"<p:sp xmlns:p="p" xmlns:a="a">
            <p:nvSpPr><p:cNvPr id="15" name="Body 1"/></p:nvSpPr>
            <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="100" cy="100"/></a:xfrm></p:spPr>
            <p:txBody>
                <a:p>
                    <a:r><a:rPr><a:latin typeface="+mn-lt"/></a:rPr><a:t>a</a:t></a:r>
                    <a:r><a:rPr><a:latin typeface="Calibri"/></a:rPr><a:t>b</a:t></a:r>
                </a:p>
            </p:txBody>
        </p:sp>"#;
        let sp = xml::Element::parse(sp_xml).unwrap();
        let el = parse_sp(&sp).unwrap();
        assert_eq!(el.font_family.as_deref(), Some("Calibri"));
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

    fn all_slide() -> SlideExclusion {
        SlideExclusion {
            slide: Some(vec![]),
            elements: vec![],
        }
    }

    fn slide_rules(ids: &[&str]) -> SlideExclusion {
        SlideExclusion {
            slide: Some(ids.iter().map(|s| s.to_string()).collect()),
            elements: vec![],
        }
    }

    #[test]
    fn parse_exclusion_bare_marker_disables_whole_slide() {
        assert_eq!(
            parse_exclusion("speaker notes\n  intern: disable  \nmore"),
            Some(all_slide()),
        );
        assert_eq!(parse_exclusion("Intern: Disable"), Some(all_slide()));
    }

    #[test]
    fn parse_exclusion_collects_named_rules() {
        assert_eq!(
            parse_exclusion("intern: disable TITLE_Y, grid_row_top"),
            Some(slide_rules(&["TITLE_Y", "GRID_ROW_TOP"])),
        );
    }

    #[test]
    fn parse_exclusion_named_rules_space_separated() {
        assert_eq!(
            parse_exclusion("intern: disable TITLE_Y GRID_ROW_TOP"),
            Some(slide_rules(&["TITLE_Y", "GRID_ROW_TOP"])),
        );
    }

    #[test]
    fn parse_exclusion_bare_line_wins_over_named() {
        assert_eq!(
            parse_exclusion("intern: disable TITLE_Y\nintern: disable"),
            Some(all_slide()),
        );
    }

    #[test]
    fn parse_exclusion_none_without_a_directive() {
        assert_eq!(parse_exclusion(""), None);
        assert_eq!(parse_exclusion("just ordinary speaker notes"), None);
        assert_eq!(parse_exclusion("intern: disabled forever"), None);
    }

    #[test]
    fn parse_exclusion_element_all_rules() {
        assert_eq!(
            parse_exclusion("intern: disable(42)"),
            Some(SlideExclusion {
                slide: None,
                elements: vec![ElementExclusion {
                    element_id: 42,
                    rules: vec![]
                }],
            }),
        );
    }

    #[test]
    fn parse_exclusion_element_named_rules() {
        assert_eq!(
            parse_exclusion("intern: disable(7) EMPTY_TEXTBOX TITLE_Y"),
            Some(SlideExclusion {
                slide: None,
                elements: vec![ElementExclusion {
                    element_id: 7,
                    rules: vec!["EMPTY_TEXTBOX".to_string(), "TITLE_Y".to_string()],
                }],
            }),
        );
    }

    #[test]
    fn parse_exclusion_element_and_slide_together() {
        let ex =
            parse_exclusion("intern: disable TITLE_Y\nintern: disable(5) EMPTY_TEXTBOX").unwrap();
        assert_eq!(ex.slide, Some(vec!["TITLE_Y".to_string()]));
        assert_eq!(ex.elements.len(), 1);
        assert_eq!(ex.elements[0].element_id, 5);
    }

    #[test]
    fn slide_exclusion_helpers_work() {
        let ex = SlideExclusion {
            slide: Some(vec!["TITLE_Y".to_string()]),
            elements: vec![ElementExclusion {
                element_id: 3,
                rules: vec![],
            }],
        };
        assert!(!ex.suppresses_slide());
        assert!(ex.suppresses_rule_for_slide("TITLE_Y"));
        assert!(!ex.suppresses_rule_for_slide("EMPTY_TEXTBOX"));
        assert!(ex.suppresses_rule_for_element(3, "EMPTY_TEXTBOX"));
        assert!(!ex.suppresses_rule_for_element(99, "EMPTY_TEXTBOX"));

        let all = all_slide();
        assert!(all.suppresses_slide());
        assert!(all.suppresses_rule_for_slide("ANYTHING"));
    }
}
