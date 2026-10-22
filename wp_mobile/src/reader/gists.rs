//! GitHub gists, which the page loads from GitHub.
//!
//! A gist comes in a post one of two ways. Jetpack renders it on the site, as GitHub's embed HTML,
//! and puts GitHub's stylesheet for it in the page's `<head>`, which the Reader API doesn't send.
//! Without it, the code loses its line numbers, indentation and colours. A gist from a URL on its
//! own line is broken as well: WordPress embeds it before `wpautop` runs, which then adds line
//! breaks and paragraphs to its HTML, and lets out the warning about hidden Unicode characters that
//! GitHub keeps in a `<template>`. Or the post has GitHub's embed code, a `<script>` from
//! `https://gist.github.com/<owner>/<id>.js`, which would go with the post's other scripts.
//!
//! So the cleanup replaces each gist with a code block: its code, when the post has it, and a link
//! to the gist. The code block has the address of the gist's data, which `reader.js` loads once the
//! page has shown: the gist's HTML, and GitHub's stylesheet for it. When the stylesheet has loaded,
//! the gist takes the code block's place. Offline, or when GitHub is slow or doesn't answer, the
//! code block stays, and the rest of the post never waits for it.
//!
//! GitHub's embed script would be simpler, but it writes the gist with `document.write`, which only
//! works from a script the page waits for. When GitHub stalls, the whole post stays blank. GitHub
//! only serves the data to other sites as JSONP, a script that calls the function its `callback`
//! names, so the page's policy allows scripts from `gist.github.com` when the post has a gist.
//!
//! An offline mode that packages everything the page needs could fetch the data ahead of time, and
//! show the gist itself offline too.
//!
//! TODO: Try gists in the apps' web views. So far they've been checked against GitHub in headless
//! Chrome, and in macOS WebKit with a stand-in for GitHub (`ReaderScriptTests`).

use std::sync::LazyLock;

use html5ever::{Attribute, LocalName, QualName, ns};
use scraper::{
    ElementRef, Html, Node, Selector,
    node::{Element, Text},
};
use url::Url;

use super::clean::set_attribute;

/// The class of the code block the cleanup makes of a gist.
const CODE_BLOCK_CLASS: &str = "reader-gist";

/// The code block's attribute with the address of the gist's data, for `reader.js`.
const DATA_ATTRIBUTE: &str = "data-reader-gist";

static GISTS: LazyLock<Selector> = LazyLock::new(|| selector("div.gist"));
static SCRIPTS: LazyLock<Selector> = LazyLock::new(|| selector("script[src]"));
static FILES: LazyLock<Selector> = LazyLock::new(|| selector(".gist-file"));
static LINES: LazyLock<Selector> = LazyLock::new(|| selector("td.blob-code"));
static FILE_LINKS: LazyLock<Selector> = LazyLock::new(|| selector(".gist-meta a[href]"));
static CODE_BLOCKS: LazyLock<Selector> =
    LazyLock::new(|| selector(&format!("figure.{CODE_BLOCK_CLASS}[{DATA_ATTRIBUTE}]")));
static WITH_DATA: LazyLock<Selector> = LazyLock::new(|| selector(&format!("[{DATA_ATTRIBUTE}]")));

fn selector(selector: &str) -> Selector {
    Selector::parse(selector).expect("the gist selectors are valid")
}

/// Replaces each gist, as GitHub's embed HTML or its embed script, with a code block: a `<pre>` for
/// each of its files the post has the code for, and links to the gist.
///
/// `reader.js` loads a script from the code block's attribute, and a post could write one itself,
/// so the attribute only stays on a code block, with the address of a gist's data.
pub(super) fn rebuild_gists(document: &mut Html) {
    let root = document.root_element();
    let gists: Vec<_> = root
        .select(&GISTS)
        .filter_map(|gist| Some((gist.id(), Gist::from_html(gist)?)))
        .chain(root.select(&SCRIPTS).filter_map(|script| {
            Some((script.id(), Gist::from_script(script.value().attr("src")?)?))
        }))
        .collect();

    for (id, gist) in gists {
        let mut node = document
            .tree
            .get_mut(id)
            .expect("the ID came from this tree");
        let figure = node
            .insert_before(Node::Element(element(
                "figure",
                &[
                    ("class", CODE_BLOCK_CLASS),
                    (DATA_ATTRIBUTE, &gist.address.data()),
                ],
            )))
            .id();
        node.detach();

        let mut figure = document
            .tree
            .get_mut(figure)
            .expect("the figure was just added");
        for file in &gist.files {
            figure
                .append(Node::Element(element("pre", &[])))
                .append(Node::Element(element("code", &[])))
                .append(text(&file.code));
        }
        let mut caption = figure.append(Node::Element(element("figcaption", &[])));
        if gist.files.is_empty() {
            let page = gist.address.page();
            caption
                .append(Node::Element(element("a", &[("href", &page)])))
                .append(text(&page));
        }
        for (index, file) in gist.files.iter().enumerate() {
            if index > 0 {
                caption.append(text(", "));
            }
            caption
                .append(Node::Element(element("a", &[("href", &file.url)])))
                .append(text(&file.name));
        }
    }

    set_gist_data(document, |element, data| {
        let is_code_block =
            element.name() == "figure" && element.classes().any(|class| class == CODE_BLOCK_CLASS);
        is_code_block.then(|| gist_data(data)).flatten()
    });
}

/// The addresses of the gists' data, which `reader.js` loads, for the page's Content Security
/// Policy.
pub(super) fn gist_data_urls(root: ElementRef) -> Vec<String> {
    root.select(&CODE_BLOCKS)
        .filter_map(|block| gist_data(block.value().attr(DATA_ATTRIBUTE)?))
        .collect()
}

/// Removes the addresses of the gists' data, for reading offline: the code blocks stay as they are.
pub(super) fn remove_gist_data(document: &mut Html) {
    set_gist_data(document, |_, _| None);
}

/// Sets each element's address of a gist's data to what `data` says, or removes it.
fn set_gist_data(document: &mut Html, data: impl Fn(&Element, &str) -> Option<String>) {
    let changes: Vec<_> = document
        .root_element()
        .select(&WITH_DATA)
        .map(|element| {
            let value = element.value().attr(DATA_ATTRIBUTE).unwrap_or_default();
            (element.id(), data(element.value(), value))
        })
        .collect();
    for (id, data) in changes {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            match data {
                Some(data) => set_attribute(element, DATA_ATTRIBUTE, &data),
                None => element
                    .attrs
                    .retain(|(name, _)| &*name.local != DATA_ATTRIBUTE),
            }
        }
    }
}

/// `url`, if it's the address of a gist's data: `https://gist.github.com/<owner>/<id>.json`, with a
/// `file` to show only that one of its files.
pub(super) fn gist_data(url: &str) -> Option<String> {
    let url = Url::parse(url).ok()?;
    GistAddress::from_url(&url, "json").map(|gist| gist.data())
}

/// Which gist, and which of its files to show, if not all of them.
#[derive(Debug, Clone, PartialEq)]
struct GistAddress {
    /// Who owns it. Old gists' URLs can leave it out.
    owner: Option<String>,
    id: String,
    file: Option<String>,
}

impl GistAddress {
    /// The gist at `url`, if it's the gist's `.<extension>`, like its embed script or its data, with
    /// nothing else in it.
    fn from_url(url: &Url, extension: &str) -> Option<Self> {
        let is_plain = url.scheme() == "https"
            && url.host_str() == Some("gist.github.com")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none();
        if !is_plain {
            return None;
        }
        let mut segments: Vec<&str> = url.path_segments()?.collect();
        let id = segments.pop()?.strip_suffix(extension)?.strip_suffix('.')?;
        segments.push(id);
        let (owner, id) = gist_path(&segments)?;
        let file = match url.query() {
            None => None,
            Some(_) => match url.query_pairs().collect::<Vec<_>>().as_slice() {
                [(name, file)] if name == "file" && !file.is_empty() => Some(file.to_string()),
                _ => return None,
            },
        };
        Some(Self {
            owner: owner.map(str::to_owned),
            id: id.to_owned(),
            file,
        })
    }

    fn path(&self) -> String {
        match &self.owner {
            Some(owner) => format!("{owner}/{}", self.id),
            None => self.id.clone(),
        }
    }

    /// The address of the gist's data: its HTML, and GitHub's stylesheet for it.
    fn data(&self) -> String {
        let mut url = Url::parse(&format!("https://gist.github.com/{}.json", self.path()))
            .expect("owners and IDs are safe in a URL");
        if let Some(file) = &self.file {
            url.query_pairs_mut().append_pair("file", file);
        }
        url.into()
    }

    /// The gist's page on GitHub.
    fn page(&self) -> String {
        format!("https://gist.github.com/{}", self.path())
    }
}

/// A gist in the post.
#[derive(Debug)]
struct Gist {
    address: GistAddress,
    /// Its files, when the post has GitHub's HTML for them, rather than only its script.
    files: Vec<GistFile>,
}

#[derive(Debug)]
struct GistFile {
    name: String,
    /// The gist's page, at the file.
    url: String,
    code: String,
}

impl Gist {
    /// The gist in GitHub's embed HTML, `gist`, if each of its files has its code, and a link to
    /// it in the gist's footer.
    ///
    /// One with a single file shows only that one, as the shortcode's `file` does, so the post
    /// shows what it did when it was written, even if the gist gets more files.
    fn from_html(gist: ElementRef) -> Option<Self> {
        let mut path: Option<(Option<String>, String)> = None;
        let mut files = Vec::new();
        for file in gist.select(&FILES) {
            // A row for each line, which holds the line's text and nothing else.
            let lines: Vec<String> = file
                .select(&LINES)
                .map(|cell| {
                    cell.text()
                        .collect::<String>()
                        .trim_end_matches(['\n', '\r'])
                        .to_owned()
                })
                .collect();
            if lines.is_empty() {
                return None;
            }
            let link = file.select(&FILE_LINKS).find_map(file_link)?;
            let file_path = (link.owner, link.id);
            if path.get_or_insert_with(|| file_path.clone()) != &file_path {
                return None;
            }
            files.push(GistFile {
                name: link.name,
                url: link.url,
                code: lines.join("\n"),
            });
        }
        let (owner, id) = path?;
        let file = match files.as_slice() {
            [file] => Some(file.name.clone()),
            _ => None,
        };
        Some(Self {
            address: GistAddress { owner, id, file },
            files,
        })
    }

    /// The gist a post's `<script>` embeds, if it's a gist's embed script. Its URL can leave out
    /// the scheme, or use `http:`.
    fn from_script(src: &str) -> Option<Self> {
        let src = src.trim();
        let src = match src.strip_prefix("//") {
            Some(rest) => format!("https://{rest}"),
            None => src.to_owned(),
        };
        let mut url = Url::parse(&src).ok()?;
        if url.scheme() == "http" {
            url.set_scheme("https").ok()?;
        }
        Some(Self {
            address: GistAddress::from_url(&url, "js")?,
            files: Vec::new(),
        })
    }
}

/// What a link in a gist's footer says about the file it goes to.
struct FileLink {
    owner: Option<String>,
    id: String,
    url: String,
    name: String,
}

/// The file a link in a gist's footer goes to, if it's the gist's page at the file, like
/// `https://gist.github.com/octocat/6cad326836d38bd3a7ae#file-hello_world-rb`, with the file's name
/// as its text. The footer's other links are "view raw" and GitHub's home page.
fn file_link(link: ElementRef) -> Option<FileLink> {
    let href = link.value().attr("href")?;
    let url = Url::parse(href).ok()?;
    let is_gist_page = matches!(url.scheme(), "http" | "https")
        && url.host_str() == Some("gist.github.com")
        && url
            .fragment()
            .is_some_and(|fragment| fragment.starts_with("file-"));
    if !is_gist_page {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.collect();
    let (owner, id) = gist_path(&segments)?;
    let name = link.text().collect::<String>().trim().to_owned();
    (!name.is_empty()).then(|| FileLink {
        owner: owner.map(str::to_owned),
        id: id.to_owned(),
        url: href.to_owned(),
        name,
    })
}

/// The owner and ID in a gist's path, `<owner>/<id>` or `<id>`, if they're the characters GitHub
/// allows: an owner is a username, and an ID is hexadecimal.
fn gist_path<'a>(segments: &[&'a str]) -> Option<(Option<&'a str>, &'a str)> {
    let (owner, id) = match segments {
        [id] => (None, *id),
        [owner, id] => (Some(*owner), *id),
        _ => return None,
    };
    let is_owner = |owner: &str| {
        (1..=39).contains(&owner.len())
            && owner
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    let is_id = (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit());
    (is_id && owner.is_none_or(is_owner)).then_some((owner, id))
}

fn element(name: &str, attributes: &[(&str, &str)]) -> Element {
    let attributes = attributes
        .iter()
        .map(|(name, value)| Attribute {
            name: QualName::new(None, ns!(), LocalName::from(*name)),
            value: (*value).into(),
        })
        .collect();
    Element::new(
        QualName::new(None, ns!(html), LocalName::from(name)),
        attributes,
    )
}

fn text(text: &str) -> Node {
    Node::Text(Text { text: text.into() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::safety_tests::safety_problems;
    use crate::reader::{
        ReaderBrand, ReaderPageOptions, ReaderPostContent, ReaderTheme, reader_default_app_colors,
    };
    use rstest::rstest;

    /// GitHub's embed HTML for a gist with one file, cut down to what the cleanup reads, plus the
    /// footer's three links.
    const GIST: &str = r#"<div class="gist" id="gist14873282"><div class="gist-file"><div class="gist-data"><table class="highlight"><tbody><tr><td class="blob-num" data-line-number="1"></td><td class="blob-code blob-code-inner"><span class="pl-k">def</span> <span class="pl-en">hi</span></td></tr><tr><td class="blob-num" data-line-number="2"></td><td class="blob-code blob-code-inner">
</td></tr><tr><td class="blob-num" data-line-number="3"></td><td class="blob-code blob-code-inner">   <span class="pl-en">puts</span> <span class="pl-s">"Hi"</span></td></tr></tbody></table></div><div class="gist-meta"><a href="https://gist.github.com/octocat/6cad326836d38bd3a7ae/raw/de5b9b59/hello world.rb">view raw</a>
<a href="https://gist.github.com/octocat/6cad326836d38bd3a7ae#file-hello-world-rb">
  hello world.rb
</a>
hosted with ❤ by <a href="https://github.com/">GitHub</a></div></div></div>"#;

    const DATA: &str =
        "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json?file=hello+world.rb";

    fn options(offline: bool) -> ReaderPageOptions {
        ReaderPageOptions {
            theme: ReaderTheme::Sepia,
            app_colors: reader_default_app_colors(ReaderBrand::Wordpress),
            language: None,
            offline,
            font: None,
            text_scale: 1.0,
            server_css: None,
            css: None,
        }
    }

    #[test]
    fn replaces_a_gist_with_its_code() {
        let content = ReaderPostContent::new(GIST, None);
        assert_eq!(
            content.html(),
            format!(
                r#"<figure class="reader-gist" data-reader-gist="{DATA}"><pre><code>def hi

   puts "Hi"</code></pre><figcaption><a href="https://gist.github.com/octocat/6cad326836d38bd3a7ae#file-hello-world-rb">hello world.rb</a></figcaption></figure>"#
            )
        );
        // `reader.js` loads the gist, not the page.
        assert_eq!(content.scripts(), Vec::<String>::new());
        assert_eq!(safety_problems(&content), Vec::<String>::new());
    }

    #[test]
    fn lets_reader_js_load_the_gist_on_the_page() {
        let content = ReaderPostContent::new(GIST, None);
        let page = content.page_html(options(false));
        assert!(page.contains(r#"<figure class="reader-gist" data-reader-gist=""#));
        assert!(!page.contains("<script src"), "{page}");
        assert!(
            page.contains("https://gist.github.com; object-src"),
            "{page}"
        );
    }

    #[test]
    fn shows_only_the_code_offline() {
        let content = ReaderPostContent::new(GIST, None);
        assert!(!content.offline_html().contains("data-reader-gist"));
        let page = content.page_html(options(true));
        assert!(page.contains(r#"<figure class="reader-gist"><pre><code>def hi"#));
        assert!(!page.contains("https://gist.github.com;"), "{page}");
    }

    #[test]
    fn keeps_a_gist_s_embed_script_where_it_is() {
        let content = ReaderPostContent::new(
            r#"<p>Before</p><script src="//gist.github.com/octocat/6cad326836d38bd3a7ae.js"></script><p>After</p>"#,
            None,
        );
        assert_eq!(
            content.html(),
            r#"<p>Before</p><figure class="reader-gist" data-reader-gist="https://gist.github.com/octocat/6cad326836d38bd3a7ae.json"><figcaption><a href="https://gist.github.com/octocat/6cad326836d38bd3a7ae">https://gist.github.com/octocat/6cad326836d38bd3a7ae</a></figcaption></figure><p>After</p>"#
        );
        assert_eq!(safety_problems(&content), Vec::<String>::new());
    }

    #[test]
    fn shows_every_file_of_a_gist_with_several() {
        let second = GIST
            .replace("hello world.rb", "second.rb")
            .replace("hello-world-rb", "second-rb")
            .replace("def</span>", "class</span>");
        let files = |html: &str| {
            let start = html.find(r#"<div class="gist-file">"#).expect("a file");
            html[start..html.len() - "</div>".len()].to_owned()
        };
        let html = format!(
            r#"<div class="gist">{}{}</div>"#,
            files(GIST),
            files(&second)
        );
        let content = ReaderPostContent::new(&html, None);
        let html = content.html();
        // Without a `file`, the data has all of them.
        assert!(
            html.contains(
                r#"data-reader-gist="https://gist.github.com/octocat/6cad326836d38bd3a7ae.json""#
            ),
            "{html}"
        );
        assert_eq!(html.matches("<pre>").count(), 2, "{html}");
        assert!(html.contains("<code>class hi"), "{html}");
        assert!(
            html.contains(r#">hello world.rb</a>, <a href="https://gist.github.com/octocat/6cad326836d38bd3a7ae#file-second-rb">second.rb</a></figcaption>"#),
            "{html}"
        );
    }

    #[rstest]
    #[case::no_gist_markup(r#"<div class="gist"><p>A gist of the plot.</p></div>"#)]
    #[case::no_link_to_the_file(
        r#"<div class="gist"><div class="gist-file"><table><tr><td class="blob-code">x</td></tr></table></div></div>"#
    )]
    #[case::a_link_to_another_site(
        r#"<div class="gist"><div class="gist-file"><table><tr><td class="blob-code">x</td></tr></table><div class="gist-meta"><a href="https://example.com/octocat/6cad#file-x">x</a></div></div></div>"#
    )]
    #[case::another_script(r#"<script src="https://gist.github.com.example.com/a/1.js"></script>"#)]
    fn leaves_what_isnt_a_gist(#[case] html: &str) {
        let content = ReaderPostContent::new(html, None);
        assert!(
            !content.html().contains("reader-gist"),
            "{}",
            content.html()
        );
        assert!(
            !content
                .page_html(options(false))
                .contains("https://gist.github.com;"),
            "{html}"
        );
    }

    #[rstest]
    #[case::another_host(
        r#"<figure class="reader-gist" data-reader-gist="https://example.com/a.json"></figure>"#
    )]
    #[case::not_a_code_block(
        r#"<div data-reader-gist="https://gist.github.com/octocat/6cad326836d38bd3a7ae.json"></div>"#
    )]
    fn keeps_the_gist_s_data_only_on_a_code_block(#[case] html: &str) {
        let content = ReaderPostContent::new(html, None);
        assert!(
            !content.html().contains("data-reader-gist"),
            "{}",
            content.html()
        );
        assert!(
            !content
                .page_html(options(false))
                .contains("https://gist.github.com;")
        );
        assert_eq!(safety_problems(&content), Vec::<String>::new());
    }

    #[test]
    fn cleaning_the_result_again_changes_nothing() {
        let html = ReaderPostContent::new(GIST, None).html();
        assert_eq!(ReaderPostContent::new(&html, None).html(), html);
    }

    #[rstest]
    #[case::a_gist(
        "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json",
        "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json"
    )]
    #[case::one_file(DATA, DATA)]
    #[case::a_file_with_a_space_encoded_another_way(
        "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json?file=hello%20world.rb",
        DATA
    )]
    #[case::an_old_gist_without_an_owner(
        "https://gist.github.com/1234567.json",
        "https://gist.github.com/1234567.json"
    )]
    fn reads_the_address_of_a_gist_s_data(#[case] url: &str, #[case] data: &str) {
        assert_eq!(gist_data(url).as_deref(), Some(data));
    }

    #[rstest]
    #[case::http("http://gist.github.com/octocat/6cad.json")]
    #[case::another_host("https://gist.github.com.example.com/octocat/6cad.json")]
    #[case::a_port("https://gist.github.com:8443/octocat/6cad.json")]
    #[case::a_user("https://user@gist.github.com/octocat/6cad.json")]
    #[case::a_fragment("https://gist.github.com/octocat/6cad.json#x")]
    #[case::the_embed_script("https://gist.github.com/octocat/6cad.js")]
    #[case::the_gist_s_page("https://gist.github.com/octocat/6cad")]
    #[case::no_dot("https://gist.github.com/octocat/6cadjson")]
    #[case::a_raw_file("https://gist.github.com/octocat/6cad/raw/1/a.json")]
    #[case::an_id_that_isnt_hexadecimal("https://gist.github.com/octocat/6cad-x.json")]
    #[case::an_owner_with_a_dot("https://gist.github.com/octo.cat/6cad.json")]
    #[case::a_callback("https://gist.github.com/octocat/6cad.json?callback=x")]
    #[case::a_second_file("https://gist.github.com/octocat/6cad.json?file=a&file=b")]
    #[case::an_empty_file("https://gist.github.com/octocat/6cad.json?file=")]
    #[case::not_a_url("gist.github.com/octocat/6cad.json")]
    fn rejects_anything_else(#[case] url: &str) {
        assert_eq!(gist_data(url), None);
    }
}
