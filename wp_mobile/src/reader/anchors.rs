use std::collections::HashSet;

use scraper::{ElementRef, Node};

/// A place in a Reader post that a link can go to, like a heading or a footnote.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Record)]
pub struct ReaderPostAnchor {
    /// The anchor's name. A link to it ends in `#` and the name.
    pub id: String,
    pub kind: ReaderPostAnchorKind,
    /// The text there: a heading's, or a footnote's without its link back. `None` for other kinds
    /// of anchor.
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum ReaderPostAnchorKind {
    /// A heading, from `<h1>` (level 1) to `<h6>`.
    Heading { level: u8 },
    /// A footnote.
    Footnote,
    /// Anywhere else a link in the post goes to, like the place a footnote links back to.
    Other,
}

/// The post's anchors, in order: headings with an `id`, footnotes, and anywhere else a link in the
/// post goes to.
pub(super) fn anchors(root: ElementRef) -> Vec<ReaderPostAnchor> {
    let linked: HashSet<String> = root
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter_map(|element| in_page_target(element.value().attr("href")?))
        .collect();

    let mut anchors: Vec<ReaderPostAnchor> = Vec::new();
    for element in root.descendants().filter_map(ElementRef::wrap) {
        let value = element.value();
        // `<a name>` is how anchors were made before `id`.
        let Some(id) = value
            .id()
            .or_else(|| value.attr("name").filter(|_| value.name() == "a"))
        else {
            continue;
        };
        if anchors.iter().any(|anchor| anchor.id == id) {
            continue;
        }

        let heading_level = value
            .name()
            .strip_prefix('h')
            .and_then(|level| level.parse::<u8>().ok())
            .filter(|level| (1..=6).contains(level));
        let anchor = if let Some(level) = heading_level {
            ReaderPostAnchor {
                id: id.to_owned(),
                kind: ReaderPostAnchorKind::Heading { level },
                text: text_without_in_page_links(element),
            }
        } else if is_footnote(element) {
            ReaderPostAnchor {
                id: id.to_owned(),
                kind: ReaderPostAnchorKind::Footnote,
                text: text_without_in_page_links(element),
            }
        } else if linked.contains(id) {
            ReaderPostAnchor {
                id: id.to_owned(),
                kind: ReaderPostAnchorKind::Other,
                text: None,
            }
        } else {
            continue;
        };
        anchors.push(anchor);
    }
    anchors
}

/// The anchor a link goes to, if it's in the same post: `#name`.
fn in_page_target(href: &str) -> Option<String> {
    let name = href.strip_prefix('#').filter(|name| !name.is_empty())?;
    Some(percent_decode(name))
}

/// A list item in the footnotes block, or in the `footnotes` list Markdown and footnote plugins
/// make.
fn is_footnote(element: ElementRef) -> bool {
    element.value().name() == "li"
        && element
            .ancestors()
            .filter_map(ElementRef::wrap)
            .any(|ancestor| {
                ancestor
                    .value()
                    .classes()
                    .any(|class| class == "wp-block-footnotes" || class == "footnotes")
            })
}

/// An element's text, without the text of links to elsewhere in the post, like a footnote's `↩︎`.
fn text_without_in_page_links(element: ElementRef) -> Option<String> {
    let text: String = element
        .descendants()
        .filter_map(|node| match node.value() {
            Node::Text(text) => Some((node, text)),
            _ => None,
        })
        .filter(|(node, _)| {
            !node
                .ancestors()
                .take_while(|ancestor| ancestor.id() != element.id())
                .filter_map(ElementRef::wrap)
                .any(|ancestor| {
                    ancestor.value().name() == "a"
                        && ancestor
                            .value()
                            .attr("href")
                            .and_then(in_page_target)
                            .is_some()
                })
        })
        .map(|(_, text)| &**text)
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// Decodes `%XX` escapes, which some footnote plugins put in their anchors' names.
pub(super) fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escape = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|hex| u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok());
        match escape {
            Some(byte) => {
                decoded.push(byte);
                index += 3;
            }
            None => {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::ReaderPostContent;
    use rstest::rstest;

    fn anchor(id: &str, kind: ReaderPostAnchorKind, text: Option<&str>) -> ReaderPostAnchor {
        ReaderPostAnchor {
            id: id.to_owned(),
            kind,
            text: text.map(str::to_owned),
        }
    }

    #[test]
    fn finds_footnotes_and_headings() {
        // The footnotes block's markup.
        let content = ReaderPostContent::new(
            r##"<h2 id="intro">An <em>introduction</em></h2>
            <p>A claim.<sup data-fn="f1" class="fn"><a href="#f1" id="f1-link">1</a></sup></p>
            <h3>No id, so no anchor</h3>
            <ol class="wp-block-footnotes"><li id="f1">The <a href="https://example.com/">source</a>. <a href="#f1-link" aria-label="Jump to footnote reference 1">↩︎</a></li></ol>"##,
            None,
        );
        assert_eq!(
            content.anchors(),
            [
                anchor(
                    "intro",
                    ReaderPostAnchorKind::Heading { level: 2 },
                    Some("An introduction")
                ),
                anchor("f1-link", ReaderPostAnchorKind::Other, None),
                anchor("f1", ReaderPostAnchorKind::Footnote, Some("The source.")),
            ]
        );
    }

    #[test]
    fn finds_markdown_footnotes() {
        let content = ReaderPostContent::new(
            r##"<p>Text<sup id="fnref:1"><a href="#fn:1">1</a></sup></p>
            <div class="footnotes"><ol><li id="fn:1"><p>A note. <a href="#fnref:1">&#8617;</a></p></li></ol></div>"##,
            None,
        );
        assert_eq!(
            content.anchors(),
            [
                anchor("fnref:1", ReaderPostAnchorKind::Other, None),
                anchor("fn:1", ReaderPostAnchorKind::Footnote, Some("A note.")),
            ]
        );
    }

    #[rstest]
    #[case::linked_element(r##"<a href="#map">Map</a><div id="map"></div>"##, &["map"])]
    #[case::named_link(r##"<a href="#top">Top</a><a name="top"></a>"##, &["top"])]
    // The cleanup removes empty paragraphs, and empty elements between blocks, unless they're anchors.
    #[case::named_link_in_a_paragraph(
        r##"<p><a name="top"></a></p><p>Hi <a href="#top">Top</a></p>"##,
        &["top"]
    )]
    #[case::named_link_between_paragraphs(
        r##"<p>Hi</p><a name="top"></a><p><a href="#top">Top</a></p>"##,
        &["top"]
    )]
    #[case::percent_encoded(r##"<a href="#caf%C3%A9">Café</a><div id="café"></div>"##, &["café"])]
    #[case::not_linked(r#"<div id="wrapper"></div><figure id="attachment_1"></figure>"#, &[])]
    #[case::link_to_another_page(r##"<a href="https://example.com/#map">Map</a><div id="map"></div>"##, &[])]
    fn finds_what_links_go_to(#[case] html: &str, #[case] ids: &[&str]) {
        let found: Vec<String> = ReaderPostContent::new(html, None)
            .anchors()
            .into_iter()
            .map(|anchor| anchor.id)
            .collect();
        assert_eq!(found, ids);
    }
}
