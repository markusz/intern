use std::collections::HashMap;
use std::io::{Read, Write};

use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::reader::Reader as XmlReader;
use quick_xml::writer::Writer as XmlWriter;
use zip::ZipArchive;
use zip::write::FileOptions;

use crate::error::Error;
use crate::rules::Fix;

/// Apply a set of fixes to a PPTX file in-place, backing up the original to `<path>.bak`.
pub fn apply_fixes(path: &str, fixes: &[Fix]) -> Result<(), Error> {
    if fixes.is_empty() {
        return Ok(());
    }

    let bak = format!("{path}.bak");
    std::fs::copy(path, &bak).map_err(|e| Error::Write {
        path: path.to_owned(),
        message: format!("cannot create backup: {e}"),
    })?;

    let result = rewrite_zip(path, fixes);
    if result.is_err() {
        // Restore from backup on failure.
        let _ = std::fs::copy(&bak, path);
    }
    result
}

fn rewrite_zip(path: &str, fixes: &[Fix]) -> Result<(), Error> {
    let mut by_slide: HashMap<usize, Vec<&Fix>> = HashMap::new();
    for fix in fixes {
        by_slide.entry(fix.slide_idx()).or_default().push(fix);
    }

    let data = std::fs::read(path).map_err(|e| Error::Open {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let mut archive = ZipArchive::new(std::io::Cursor::new(&data)).map_err(|e| Error::Open {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    {
        let mut zip_out = zip::ZipWriter::new(std::io::Cursor::new(&mut out));

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| Error::Write {
                path: path.to_owned(),
                message: e.to_string(),
            })?;

            let name = file.name().to_owned();
            let options = FileOptions::default().compression_method(file.compression());

            if let Some(slide_idx) = parse_slide_idx(&name)
                && let Some(slide_fixes) = by_slide.get(&slide_idx)
            {
                let mut xml = String::new();
                file.read_to_string(&mut xml).map_err(|e| Error::Write {
                    path: path.to_owned(),
                    message: e.to_string(),
                })?;
                let patched = patch_slide_xml(&xml, slide_fixes);
                zip_out
                    .start_file(&name, options)
                    .map_err(|e| Error::Write {
                        path: path.to_owned(),
                        message: e.to_string(),
                    })?;
                zip_out
                    .write_all(patched.as_bytes())
                    .map_err(|e| Error::Write {
                        path: path.to_owned(),
                        message: e.to_string(),
                    })?;
                continue;
            }

            let mut bytes: Vec<u8> = Vec::new();
            file.read_to_end(&mut bytes).map_err(|e| Error::Write {
                path: path.to_owned(),
                message: e.to_string(),
            })?;
            zip_out
                .start_file(&name, options)
                .map_err(|e| Error::Write {
                    path: path.to_owned(),
                    message: e.to_string(),
                })?;
            zip_out.write_all(&bytes).map_err(|e| Error::Write {
                path: path.to_owned(),
                message: e.to_string(),
            })?;
        }

        zip_out.finish().map_err(|e| Error::Write {
            path: path.to_owned(),
            message: e.to_string(),
        })?;
    }

    std::fs::write(path, out).map_err(|e| Error::Write {
        path: path.to_owned(),
        message: e.to_string(),
    })
}

/// Converts "ppt/slides/slideN.xml" to the 0-based slide index N-1.
fn parse_slide_idx(path: &str) -> Option<usize> {
    let rest = path.strip_prefix("ppt/slides/slide")?;
    let n: usize = rest.strip_suffix(".xml")?.parse().ok()?;
    Some(n - 1)
}

/// Streams the slide XML through quick-xml, patching the attributes of targeted elements.
///
/// PPTX shape structure (in document order within `<p:sp>`):
///   `<p:nvSpPr><p:cNvPr name="..."/>` — comes first, used for name lookup
///   `<p:spPr><a:xfrm><a:off x y/><a:ext cx cy/>` — position/size
///   `<p:txBody>...<a:rPr sz/>` — font runs
///
/// Because `cNvPr` always precedes `<a:off>` and `<a:rPr>`, the state machine
/// sets `current_fix` before it needs to patch anything.
fn patch_slide_xml(xml: &str, fixes: &[&Fix]) -> String {
    let fix_map: HashMap<String, &Fix> = fixes
        .iter()
        .map(|f| (f.element_name().to_owned(), *f))
        .collect();

    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut writer = XmlWriter::new(Vec::new());
    let mut current_fix: Option<&Fix> = None;
    let mut in_text_run = false;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => {
                match local_name_str(&e).as_str() {
                    "sp" => {
                        // Entering a new shape — clear fix until we see cNvPr.
                        current_fix = None;
                        writer.write_event(Event::Start(e)).ok();
                    }
                    "cNvPr" => {
                        if let Some(name) = attr_value(&e, b"name") {
                            current_fix = fix_map.get(&name).copied();
                        }
                        writer.write_event(Event::Start(e)).ok();
                    }
                    "t" => {
                        in_text_run = matches!(current_fix, Some(Fix::NormalizeWhitespace { .. }));
                        writer.write_event(Event::Start(e)).ok();
                    }
                    "rPr" | "defRPr" => {
                        if let Some(fix) = current_fix {
                            writer.write_event(Event::Start(patch_font(e, fix))).ok();
                        } else {
                            writer.write_event(Event::Start(e)).ok();
                        }
                    }
                    _ => {
                        writer.write_event(Event::Start(e)).ok();
                    }
                }
            }

            Ok(Event::Empty(e)) => match (local_name_str(&e).as_str(), current_fix) {
                ("cNvPr", _) => {
                    if let Some(name) = attr_value(&e, b"name") {
                        current_fix = fix_map.get(&name).copied();
                    }
                    writer.write_event(Event::Empty(e)).ok();
                }
                ("off", Some(fix)) => {
                    writer.write_event(Event::Empty(patch_off(e, fix))).ok();
                }
                ("ext", Some(fix)) => {
                    writer.write_event(Event::Empty(patch_ext(e, fix))).ok();
                }
                ("rPr" | "defRPr", Some(fix)) => {
                    writer.write_event(Event::Empty(patch_font(e, fix))).ok();
                }
                _ => {
                    writer.write_event(Event::Empty(e)).ok();
                }
            },

            Ok(Event::End(e)) => {
                match local_name_bytes(e.name().as_ref()) {
                    b"sp" => current_fix = None,
                    b"t" => in_text_run = false,
                    _ => {}
                }
                writer.write_event(Event::End(e)).ok();
            }

            Ok(Event::Text(e)) if in_text_run => {
                if let Ok(raw) = e.unescape() {
                    let normalized = normalize_ws(&raw);
                    writer
                        .write_event(Event::Text(BytesText::new(&normalized)))
                        .ok();
                } else {
                    writer.write_event(Event::Text(e)).ok();
                }
            }

            Ok(e) => {
                writer.write_event(e).ok();
            }

            Err(_) => break,
        }
    }

    String::from_utf8(writer.into_inner()).unwrap_or_else(|_| xml.to_owned())
}

fn local_name_str(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn local_name_bytes(name: &[u8]) -> &[u8] {
    name.iter()
        .rposition(|&b| b == b':')
        .map_or(name, |i| &name[i + 1..])
}

fn attr_value(e: &BytesStart<'_>, local: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == local)
        .and_then(|a| String::from_utf8(a.value.as_ref().to_vec()).ok())
}

/// Rebuild a `BytesStart` with selected attributes replaced.
/// The element name (including namespace prefix) is preserved verbatim.
fn replace_attrs(e: BytesStart<'_>, replacements: &HashMap<&[u8], Vec<u8>>) -> BytesStart<'static> {
    let name_qname = e.name();
    let name_bytes = name_qname.as_ref();
    let name_str = std::str::from_utf8(name_bytes).unwrap_or("");
    let name_len = name_str.len();
    let mut buf = name_str.to_owned();

    for attr in e.attributes().flatten() {
        let local = attr.key.local_name();
        let val_bytes = replacements
            .get(local.as_ref())
            .map(|v| v.as_slice())
            .unwrap_or_else(|| attr.value.as_ref());

        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
        let val = std::str::from_utf8(val_bytes).unwrap_or("");

        buf.push(' ');
        buf.push_str(key);
        buf.push_str("=\"");
        buf.push_str(val);
        buf.push('"');
    }

    BytesStart::from_content(buf, name_len)
}

fn normalize_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for ch in s.trim().chars() {
        if ch == ' ' {
            if !prev_space {
                out.push(ch);
            }
            prev_space = true;
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

fn patch_off(e: BytesStart<'_>, fix: &Fix) -> BytesStart<'static> {
    let mut r: HashMap<&[u8], Vec<u8>> = HashMap::new();
    match fix {
        Fix::SetX { x, .. } => {
            r.insert(b"x", x.to_string().into_bytes());
        }
        Fix::SetY { y, .. } => {
            r.insert(b"y", y.to_string().into_bytes());
        }
        Fix::SetXW { x, .. } => {
            if let Some(x) = x {
                r.insert(b"x", x.to_string().into_bytes());
            }
        }
        Fix::SetFontSize { .. } | Fix::NormalizeWhitespace { .. } => {}
    }
    replace_attrs(e, &r)
}

fn patch_ext(e: BytesStart<'_>, fix: &Fix) -> BytesStart<'static> {
    let mut r: HashMap<&[u8], Vec<u8>> = HashMap::new();
    if let Fix::SetXW { w: Some(w), .. } = fix {
        r.insert(b"cx", w.to_string().into_bytes());
    }
    replace_attrs(e, &r)
}

fn patch_font(e: BytesStart<'_>, fix: &Fix) -> BytesStart<'static> {
    let mut r: HashMap<&[u8], Vec<u8>> = HashMap::new();
    if let Fix::SetFontSize { size, .. } = fix {
        r.insert(b"sz", size.to_string().into_bytes());
    }
    replace_attrs(e, &r)
}

#[cfg(test)]
mod tests {
    use super::{normalize_ws, parse_slide_idx, patch_slide_xml};
    use crate::rules::Fix;

    #[test]
    fn parse_slide_idx_first_slide() {
        assert_eq!(parse_slide_idx("ppt/slides/slide1.xml"), Some(0));
    }

    #[test]
    fn parse_slide_idx_tenth_slide() {
        assert_eq!(parse_slide_idx("ppt/slides/slide10.xml"), Some(9));
    }

    #[test]
    fn parse_slide_idx_ignores_non_slide_paths() {
        assert_eq!(parse_slide_idx("ppt/slides/_rels/slide1.xml.rels"), None);
        assert_eq!(parse_slide_idx("ppt/slideLayouts/slideLayout1.xml"), None);
        assert_eq!(parse_slide_idx("[Content_Types].xml"), None);
    }

    #[test]
    fn parse_slide_idx_rejects_non_numeric_suffix() {
        assert_eq!(parse_slide_idx("ppt/slides/slideABC.xml"), None);
    }

    #[test]
    fn normalize_ws_collapses_spaces() {
        assert_eq!(normalize_ws("hello  world"), "hello world");
        assert_eq!(normalize_ws("a   b   c"), "a b c");
    }

    #[test]
    fn normalize_ws_trims_edges() {
        assert_eq!(normalize_ws("  hello  "), "hello");
        assert_eq!(normalize_ws(" leading"), "leading");
        assert_eq!(normalize_ws("trailing "), "trailing");
    }

    #[test]
    fn normalize_ws_leaves_clean_text_unchanged() {
        assert_eq!(normalize_ws("clean text"), "clean text");
    }

    #[test]
    fn patch_slide_xml_normalizes_double_space() {
        let xml = r#"<root xmlns:p="p" xmlns:a="a">
  <p:sp>
    <p:nvSpPr><p:cNvPr name="Body"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>hello  world</a:t></a:r></a:p></p:txBody>
  </p:sp>
</root>"#;
        let fix = Fix::NormalizeWhitespace {
            slide_idx: 0,
            element_name: "Body".into(),
        };
        let result = patch_slide_xml(xml, &[&fix]);
        assert!(result.contains("hello world"), "got: {result}");
        assert!(!result.contains("hello  world"), "got: {result}");
    }

    #[test]
    fn patch_slide_xml_trims_trailing_space() {
        let xml = r#"<root xmlns:p="p" xmlns:a="a">
  <p:sp>
    <p:nvSpPr><p:cNvPr name="Body"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>trailing space </a:t></a:r></a:p></p:txBody>
  </p:sp>
</root>"#;
        let fix = Fix::NormalizeWhitespace {
            slide_idx: 0,
            element_name: "Body".into(),
        };
        let result = patch_slide_xml(xml, &[&fix]);
        assert!(result.contains(">trailing space<"), "got: {result}");
    }

    #[test]
    fn patch_slide_xml_only_patches_named_shape() {
        let xml = r#"<root xmlns:p="p" xmlns:a="a">
  <p:sp>
    <p:nvSpPr><p:cNvPr name="Other"/></p:nvSpPr>
    <p:txBody><a:p><a:r><a:t>hello  world</a:t></a:r></a:p></p:txBody>
  </p:sp>
</root>"#;
        let fix = Fix::NormalizeWhitespace {
            slide_idx: 0,
            element_name: "Body".into(),
        };
        let result = patch_slide_xml(xml, &[&fix]);
        // Different shape — text should be untouched.
        assert!(result.contains("hello  world"), "got: {result}");
    }
}
