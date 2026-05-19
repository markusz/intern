use std::collections::VecDeque;
use std::str;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Maximum XML element nesting depth accepted by `Element::parse`. Real PPTX
/// parts nest only tens of levels deep; the cap rejects pathologically deep
/// input so that neither tree traversal nor the recursive `Drop` of the parsed
/// tree can overflow the stack.
const MAX_DEPTH: usize = 1000;

/// Lightweight XML parse error.
#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<quick_xml::Error> for ParseError {
    fn from(e: quick_xml::Error) -> Self {
        Self(e.to_string())
    }
}

/// A lightweight XML element: local tag name (no namespace prefix), attributes
/// (full qualified name preserved so callers can match `r:id`), direct text
/// content, and child elements.
#[derive(Debug, Clone, Default)]
pub struct Element {
    pub tag: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    /// Parses an XML string into a DOM tree. Namespace prefixes are stripped
    /// from element tag names but preserved for attribute names (e.g. `r:id`).
    pub fn parse(xml: &str) -> Result<Self, ParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut stack: Vec<Element> = vec![Element::default()];
        let mut buf = Vec::new();

        loop {
            match reader
                .read_event_into(&mut buf)
                .map_err(|e| ParseError(e.to_string()))?
            {
                Event::Start(ref e) => {
                    if stack.len() > MAX_DEPTH {
                        return Err(ParseError("XML nesting exceeds the maximum depth".into()));
                    }
                    let tag = strip_prefix(e.local_name().as_ref());
                    let attrs = parse_attrs(e)?;
                    stack.push(Element {
                        tag,
                        attrs,
                        ..Element::default()
                    });
                }
                Event::Empty(ref e) => {
                    let tag = strip_prefix(e.local_name().as_ref());
                    let attrs = parse_attrs(e)?;
                    let leaf = Element {
                        tag,
                        attrs,
                        ..Element::default()
                    };
                    stack
                        .last_mut()
                        .ok_or_else(|| ParseError("empty parse stack".into()))?
                        .children
                        .push(leaf);
                }
                Event::Text(ref e) => {
                    if let Ok(text) = e.unescape()
                        && !text.is_empty()
                    {
                        stack
                            .last_mut()
                            .ok_or_else(|| ParseError("empty parse stack".into()))?
                            .text
                            .push_str(&text);
                    }
                }
                Event::End(_) if stack.len() > 1 => {
                    let finished = stack
                        .pop()
                        .ok_or_else(|| ParseError("empty parse stack".into()))?;
                    stack
                        .last_mut()
                        .ok_or_else(|| ParseError("empty parse stack".into()))?
                        .children
                        .push(finished);
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        let mut root = stack
            .pop()
            .ok_or_else(|| ParseError("empty parse stack".into()))?;
        root.children
            .pop()
            .ok_or_else(|| ParseError("empty document".into()))
    }

    /// Returns the value of the named attribute, or `None` if absent.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Returns the first direct child with the given local tag name.
    pub fn find(&self, tag: &str) -> Option<&Self> {
        self.children.iter().find(|c| c.tag == tag)
    }

    /// Returns all direct children with the given local tag name.
    pub fn find_all(&self, tag: &str) -> Vec<&Self> {
        self.children.iter().filter(|c| c.tag == tag).collect()
    }

    /// Returns the first descendant (BFS) with the given local tag name.
    pub fn find_descendant(&self, tag: &str) -> Option<&Self> {
        let mut queue = VecDeque::new();
        for child in &self.children {
            queue.push_back(child);
        }
        while let Some(node) = queue.pop_front() {
            if node.tag == tag {
                return Some(node);
            }
            for child in &node.children {
                queue.push_back(child);
            }
        }
        None
    }

    /// Returns all descendants with the given local tag name, depth-first
    /// pre-order. Iterative, so deeply nested input cannot overflow the stack.
    pub fn find_all_descendants(&self, tag: &str) -> Vec<&Self> {
        let mut out = Vec::new();
        let mut stack: Vec<&Element> = self.children.iter().rev().collect();
        while let Some(node) = stack.pop() {
            if node.tag == tag {
                out.push(node);
            }
            stack.extend(node.children.iter().rev());
        }
        out
    }
}

fn strip_prefix(bytes: &[u8]) -> String {
    let s = str::from_utf8(bytes).unwrap_or("");
    s.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(s)
        .to_string()
}

fn parse_attrs(e: &BytesStart<'_>) -> Result<Vec<(String, String)>, ParseError> {
    let mut attrs = Vec::new();
    for result in e.attributes() {
        let attr = result.map_err(|e| ParseError(e.to_string()))?;
        let name = str::from_utf8(attr.key.as_ref()).unwrap_or("").to_string();
        let value = attr
            .unescape_value()
            .map_err(|e| ParseError(e.to_string()))?
            .into_owned();
        attrs.push((name, value));
    }
    Ok(attrs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(xml: &str) -> Element {
        Element::parse(xml).expect("parse failed")
    }

    #[test]
    fn parses_root_tag_stripping_namespace() {
        let el = parse(r#"<p:sld xmlns:p="p"><p:cSld/></p:sld>"#);
        assert_eq!(el.tag, "sld");
        assert_eq!(el.children[0].tag, "cSld");
    }

    #[test]
    fn attr_returns_plain_attribute() {
        let el = parse(r#"<a:ext cx="100" cy="200"/>"#);
        assert_eq!(el.attr("cx"), Some("100"));
        assert_eq!(el.attr("cy"), Some("200"));
        assert_eq!(el.attr("missing"), None);
    }

    #[test]
    fn attr_preserves_namespace_prefix_in_attribute_names() {
        // r:id must be findable by its qualified name.
        let el = parse(r#"<p:sldId id="256" r:id="rId3"/>"#);
        assert_eq!(el.attr("r:id"), Some("rId3"));
        assert_eq!(el.attr("id"), Some("256"));
    }

    #[test]
    fn find_returns_first_direct_child() {
        let el = parse(r#"<root><a/><b/><a/></root>"#);
        assert_eq!(el.find("a").map(|e| e.tag.as_str()), Some("a"));
        assert!(el.find("c").is_none());
    }

    #[test]
    fn find_all_returns_matching_direct_children() {
        let el = parse(r#"<root><a/><b/><a/></root>"#);
        assert_eq!(el.find_all("a").len(), 2);
        assert_eq!(el.find_all("b").len(), 1);
        assert_eq!(el.find_all("c").len(), 0);
    }

    #[test]
    fn find_does_not_descend() {
        // "b" is a grandchild, not a direct child.
        let el = parse(r#"<root><a><b/></a></root>"#);
        assert!(el.find("b").is_none());
    }

    #[test]
    fn find_descendant_locates_nested_element() {
        let el = parse(r#"<root><a><b><c/></b></a></root>"#);
        assert_eq!(el.find_descendant("c").map(|e| e.tag.as_str()), Some("c"));
    }

    #[test]
    fn find_all_descendants_collects_all_matching() {
        let el = parse(r#"<root><a/><x><a/></x></root>"#);
        assert_eq!(el.find_all_descendants("a").len(), 2);
    }

    #[test]
    fn parse_rejects_pathologically_deep_xml() {
        // A tree this deep would overflow the stack when traversed or dropped;
        // parsing must reject it rather than build it.
        let depth = 5_000;
        let xml = format!("{}<t/>{}", "<n>".repeat(depth), "</n>".repeat(depth));
        assert!(Element::parse(&xml).is_err());
    }

    #[test]
    fn text_content_is_collected_on_leaf() {
        let el = parse(r#"<a:t>Hello World</a:t>"#);
        assert_eq!(el.text, "Hello World");
    }

    #[test]
    fn empty_element_gives_empty_text() {
        let el = parse(r#"<a:rPr sz="1800"/>"#);
        assert!(el.text.is_empty());
    }

    #[test]
    fn parse_error_on_empty_document() {
        assert!(Element::parse("").is_err());
        assert!(Element::parse("not xml").is_err());
    }
}
