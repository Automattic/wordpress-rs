//! WordPress shortcodes the Reader API sends without running, like `[gallery ids="1,2"]`.
//!
//! They come from Jetpack and Atomic sites. The Reader shows those posts as the site rendered
//! them, and the site leaves some shortcodes, like `[gallery]` and `[embed]`, for WordPress.com to
//! run. The Reader API doesn't run them. Plugins' shortcodes, like `[vc_row]`, can't run on
//! WordPress.com at all.

use std::{collections::HashSet, ops::Range, sync::LazyLock};

use scraper::{
    Html, Node, Selector,
    node::{Element, Text},
};
use url::Url;

use super::clean::set_attribute;

/// Hides the shortcodes in a post's text, keeping what they enclose:
/// `[su_dropcap size="5"]I[/su_dropcap]n` becomes `In`. An `[embed]` of a URL becomes a link
/// to it. A paragraph that held only shortcodes goes too.
///
/// Brackets are common in prose, like `[sic]`, so only these count as shortcodes:
///
/// - A lowercase name with attributes, like `[gallery ids="1,2"]`, `[audio https://…]` or
///   `[soundcloud … /]`.
/// - A lowercase name that's closed later in the post, like `[time]…[/time]`, and the `[/time]`
///   that closes it.
///
/// Code is left as it is: a post about shortcodes shows them in `<code>` or `<pre>`.
pub(super) fn hide_shortcodes(document: &mut Html) {
    let texts: Vec<_> = document
        .tree
        .root()
        .descendants()
        .filter_map(|node| {
            let text = node.value().as_text().filter(|text| text.contains('['))?;
            let in_code = node.ancestors().any(|ancestor| {
                ancestor.value().as_element().is_some_and(|element| {
                    matches!(element.name(), "pre" | "code" | "kbd" | "samp" | "textarea")
                })
            });
            (!in_code).then(|| (node.id(), text.to_string()))
        })
        .collect();

    let mut opened = HashSet::new();
    let mut closed = HashSet::new();
    for (_, text) in &texts {
        for token in tokens(text) {
            if token.closing {
                closed.insert(token.name);
            } else {
                opened.insert(token.name);
            }
        }
    }
    let is_shortcode = |token: &Token| {
        if token.closing {
            opened.contains(token.name)
        } else {
            token.has_attributes() || closed.contains(token.name)
        }
    };

    let mut parents = Vec::new();
    for (id, text) in &texts {
        let pieces = pieces(text, is_shortcode);
        if let [Piece::Text(only)] = pieces.as_slice()
            && only == text
        {
            continue;
        }

        let mut node = document
            .tree
            .get_mut(*id)
            .expect("the ID came from this tree");
        for piece in pieces {
            match piece {
                Piece::Text(text) if text.is_empty() => {}
                Piece::Text(text) => {
                    node.insert_before(Node::Text(Text { text: text.into() }));
                }
                Piece::Link(url) => {
                    node.insert_before(link(&url))
                        .append(Node::Text(Text { text: url.into() }));
                }
            }
        }
        if let Some(parent) = node.parent() {
            parents.push(parent.id());
        }
        node.detach();
    }

    for id in parents {
        let is_blank_paragraph = document.tree.get(id).is_some_and(|parent| {
            let is_paragraph = parent
                .value()
                .as_element()
                .is_some_and(|element| element.name() == "p");
            is_paragraph
                && parent.children().all(|child| {
                    child
                        .value()
                        .as_text()
                        .is_some_and(|text| text.trim().is_empty())
                })
        });
        if is_blank_paragraph {
            document
                .tree
                .get_mut(id)
                .expect("the ID came from this tree")
                .detach();
        }
    }
}

/// A `[name …]` or a `[/name]` in some text.
struct Token<'a> {
    range: Range<usize>,
    name: &'a str,
    closing: bool,
    /// What's between the name and the `]`.
    attributes: &'a str,
}

impl Token<'_> {
    /// Whether it has attributes, which bracketed prose like `[sic]` doesn't.
    fn has_attributes(&self) -> bool {
        let attributes = self.attributes.trim();
        let has_named_attribute = attributes
            .as_bytes()
            .windows(2)
            .any(|pair| (pair[0].is_ascii_alphanumeric() || pair[0] == b'_') && pair[1] == b'=');
        has_named_attribute
            || attributes.starts_with("http://")
            || attributes.starts_with("https://")
            || attributes.ends_with('/')
    }
}

/// The shortcode-shaped tokens in some text. WordPress's shortcode names are case-sensitive, and
/// lowercase in practice, so `[Bracketed text]` isn't one.
fn tokens(text: &str) -> impl Iterator<Item = Token<'_>> {
    let mut from = 0;
    std::iter::from_fn(move || {
        while let Some(offset) = text[from..].find('[') {
            let start = from + offset;
            from = start + 1;
            if let Some(token) = token_at(text, start) {
                from = token.range.end;
                return Some(token);
            }
        }
        None
    })
}

fn token_at(text: &str, start: usize) -> Option<Token<'_>> {
    let inner = &text[start + 1..];
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(inner) => (true, inner),
        None => (false, inner),
    };
    let name_length = inner
        .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'))
        .unwrap_or(inner.len());
    let name = &inner[..name_length];
    let end = inner[name_length..].find(']')?;
    let attributes = &inner[name_length..name_length + end];

    let is_name = name.starts_with(|c: char| c.is_ascii_lowercase());
    // The name ends at a space, a `/` or the `]`, and nothing else opens before the `]`.
    let ends_name = attributes.is_empty()
        || attributes.starts_with(char::is_whitespace)
        || attributes.starts_with('/');
    let is_token = is_name
        && ends_name
        && !attributes.contains('[')
        && !(closing && !attributes.trim().is_empty());
    is_token.then(|| Token {
        range: start..start + 1 + usize::from(closing) + name_length + end + 1,
        name,
        closing,
        attributes,
    })
}

enum Piece {
    Text(String),
    Link(String),
}

/// Some text, split around its shortcodes, without them.
fn pieces(text: &str, is_shortcode: impl Fn(&Token) -> bool) -> Vec<Piece> {
    let tokens: Vec<_> = tokens(text).filter(|token| is_shortcode(token)).collect();
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut from = 0;
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        current.push_str(&text[from..token.range.start]);
        from = token.range.end;
        index += 1;

        // `[embed]https://…[/embed]`
        if token.name == "embed"
            && !token.closing
            && let Some(close) = tokens.get(index)
            && close.name == "embed"
            && close.closing
            && let Some(url) = web_url(&text[token.range.end..close.range.start])
        {
            pieces.push(Piece::Text(std::mem::take(&mut current)));
            pieces.push(Piece::Link(url));
            from = close.range.end;
            index += 1;
        }
    }
    current.push_str(&text[from..]);
    pieces.push(Piece::Text(current));
    pieces
}

fn web_url(text: &str) -> Option<String> {
    let text = text.trim();
    let url = Url::parse(text).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| text.to_owned())
}

static LINK: LazyLock<Selector> = LazyLock::new(|| Selector::parse("a").expect("valid selector"));

/// A new `<a href="…">`. `scraper` doesn't offer a way to make an element, so this copies one it
/// parsed.
fn link(url: &str) -> Node {
    let fragment = Html::parse_fragment("<a></a>");
    let mut element: Element = fragment
        .select(&LINK)
        .next()
        .expect("the fragment has a link")
        .value()
        .clone();
    set_attribute(&mut element, "href", url);
    Node::Element(element)
}

#[cfg(test)]
mod tests {
    use crate::reader::ReaderPostContent;
    use rstest::rstest;

    #[rstest]
    #[case::attributes(
        r#"<p>Before</p><p>[gallery type="rectangular" ids="1,2,3"]</p><p>After</p>"#,
        "<p>Before</p><p>After</p>"
    )]
    #[case::curly_quotes("<p>[gdrive url=”https://example.com/a.pdf”] Text</p>", "<p> Text</p>")]
    #[case::self_closing(
        r#"<p>Hi [soundcloud url="https://api.soundcloud.com/users/1" /]</p>"#,
        "<p>Hi </p>"
    )]
    #[case::unquoted_attribute("<p>[stuck-order id=72429180] Text</p>", "<p> Text</p>")]
    #[case::positional_url(
        "<p>Listen [audio https://example.com/a.mp3] here</p>",
        "<p>Listen  here</p>"
    )]
    #[case::enclosing(
        r#"<p>[su_dropcap size="5"]I[/su_dropcap]n recent years</p>"#,
        "<p>In recent years</p>"
    )]
    #[case::closed_without_attributes(
        "<p>Date: [time]Jul 9 2024 19:00 UTC[/time]</p>",
        "<p>Date: Jul 9 2024 19:00 UTC</p>"
    )]
    #[case::across_elements(
        r#"[caption id="attachment_1" width="300"]<img src="https://example.com/a.jpg"> A photo[/caption]"#,
        r#"<img src="https://example.com/a.jpg"> A photo"#
    )]
    #[case::nested(
        "<p>[vc_row][vc_column width=\"1/2\"]Text[/vc_column][/vc_row]</p>",
        "<p>Text</p>"
    )]
    #[case::embed(
        "<p>More info. [embed]https://www.youtube.com/watch?v=a[/embed] Next</p>",
        r#"<p>More info. <a href="https://www.youtube.com/watch?v=a">https://www.youtube.com/watch?v=a</a> Next</p>"#
    )]
    #[case::embed_of_something_else("<p>[embed]not a URL[/embed]</p>", "<p>not a URL</p>")]
    fn hides_shortcodes(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(ReaderPostContent::new(html, None).html(), expected);
    }

    #[rstest]
    #[case::sic("<p>A good Idea [sic]. Yes.</p>")]
    #[case::words("<p>The liberal media [emphasis mine]:</p>")]
    #[case::capitalized("<p>[Facebook group] and [REDACTED] and [Tweet x=1]</p>")]
    #[case::citation("<p>A claim.[1]</p>")]
    #[case::spaced_equals("<p>Let [x = 5].</p>")]
    #[case::unclosed(
        "<p>[audio <a href=\"https://example.com/a.mp3\">https://example.com/a.mp3</a>]</p>"
    )]
    #[case::code(r#"<p>Use <code>[gallery ids="1,2"]</code></p><pre>[time]x[/time]</pre>"#)]
    #[case::closing_without_opening("<p>Text [/time]</p>")]
    fn leaves_other_brackets_alone(#[case] html: &str) {
        assert_eq!(ReaderPostContent::new(html, None).html(), html);
    }

    #[test]
    fn leaves_shortcodes_out_of_the_text() {
        let content =
            ReaderPostContent::new("<p>[vc_column_text]Some text[/vc_column_text]</p>", None);
        assert_eq!(content.plain_text(), "Some text");
        assert_eq!(content.simplified_html(), "<p>Some text</p>");
    }

    #[test]
    fn checks_excerpts_with_shortcodes_left_out_the_way_wordpress_does() {
        // WordPress leaves a shortcode out of an excerpt along with what it encloses.
        let content = ReaderPostContent::new(
            r#"<p>[su_dropcap size="5"]I[/su_dropcap]n recent years, science has come to understand a lot more.</p>"#,
            None,
        );
        assert!(!content.is_manual_excerpt("n recent years, science has come to understand a lot"));
    }
}
