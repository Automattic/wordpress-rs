//! Resolving the addresses a web page links to, the way a browser does.
//!
//! A page's `href` and `src` attributes are often relative, such as
//! `/wp/xmlrpc.php?rsd`, `//cdn.example.net/a.png` or `a.png`. A browser
//! resolves them against the page's address, or against the page's
//! `<base href>` when it has one. [`HtmlLinks`] does the same, so code that
//! reads links out of a page never sends a request to a relative address.

use scraper::{ElementRef, Html, Selector};
use url::Url;

/// A parsed web page, with the address its relative links resolve against.
pub(crate) struct HtmlLinks {
    document: Html,
    base_url: Url,
}

impl HtmlLinks {
    /// Parses `html`, downloaded from `page_url`.
    ///
    /// `page_url` should be the address the page was finally served from,
    /// after redirects, when the caller knows it.
    pub(crate) fn parse(html: &str, page_url: &Url) -> Self {
        let document = Html::parse_document(html);
        let base_url = document_base_url(&document, page_url);
        Self { document, base_url }
    }

    /// The elements matching `selector` that a browser acts on, in document
    /// order. SVG and MathML elements are left out, and so is anything inside
    /// an HTML `<template>`. HTML elements inside an SVG `<foreignObject>` are
    /// kept, as a browser keeps them.
    pub(crate) fn active_elements<'a>(
        &'a self,
        selector: &'a Selector,
    ) -> impl Iterator<Item = ElementRef<'a>> + 'a {
        self.document
            .select(selector)
            .filter(|element| is_active_html_element(*element))
    }

    /// Resolves `address`, taken from one of this page's attributes. See
    /// [`resolve_web_address`].
    pub(crate) fn resolve(&self, address: &str) -> Option<Url> {
        resolve_web_address(address, &self.base_url)
    }
}

/// Resolves `address` against `base_url`, the way a browser resolves a link.
///
/// Returns `None` when `address` is empty or only whitespace, doesn't parse,
/// or resolves to anything but an `http` or `https` URL, such as a `data:` or
/// `javascript:` URL. Surrounding ASCII whitespace is ignored, as browsers do.
/// Other whitespace, such as a no-break space, is part of the address.
///
/// Unlike a browser, an empty address doesn't resolve to `base_url` itself:
/// no caller wants a link to the page it's reading.
pub(crate) fn resolve_web_address(address: &str, base_url: &Url) -> Option<Url> {
    let address = address.trim_ascii();
    if address.is_empty() {
        return None;
    }
    base_url
        .join(address)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
}

/// The URL relative addresses in `document` resolve against: the first
/// `<base href>`, resolved against `page_url`, or `page_url` itself.
///
/// As in the HTML standard, a base that doesn't parse, or is a `data:` or
/// `javascript:` URL, is ignored, and so is one a browser doesn't act on (see
/// [`is_active_html_element`]).
fn document_base_url(document: &Html, page_url: &Url) -> Url {
    let selector = Selector::parse("base[href]").expect("the base selector is valid");
    document
        .select(&selector)
        .find(|base| is_active_html_element(*base))
        .and_then(|base| base.value().attr("href"))
        .and_then(|href| page_url.join(href).ok())
        .filter(|base| !matches!(base.scheme(), "data" | "javascript"))
        .unwrap_or_else(|| page_url.clone())
}

/// Whether a browser acts on `element`: it's an HTML element, not an SVG or
/// MathML one, and it isn't inside an HTML `<template>`, whose contents are
/// inert. An SVG element named `template` has no such effect.
fn is_active_html_element(element: ElementRef<'_>) -> bool {
    let in_template = element.ancestors().any(|node| {
        node.value()
            .as_element()
            .is_some_and(|ancestor| is_html(ancestor) && ancestor.name() == "template")
    });
    is_html(element.value()) && !in_template
}

fn is_html(element: &scraper::node::Element) -> bool {
    &*element.name.ns == "http://www.w3.org/1999/xhtml"
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn url(url: &str) -> Url {
        Url::parse(url).expect("test URLs are valid")
    }

    fn resolve_in(html: &str, page_url: &str, address: &str) -> Option<String> {
        HtmlLinks::parse(html, &url(page_url))
            .resolve(address)
            .map(String::from)
    }

    #[rstest]
    #[case::root_relative("https://example.com/blog/", "/a.png", "https://example.com/a.png")]
    #[case::path_relative("https://example.com/blog/", "a.png", "https://example.com/blog/a.png")]
    #[case::path_relative_without_trailing_slash(
        "https://example.com/blog",
        "a.png",
        "https://example.com/a.png"
    )]
    #[case::parent(
        "https://example.com/blog/post/",
        "../a.png",
        "https://example.com/blog/a.png"
    )]
    #[case::scheme_relative(
        "https://example.com/",
        "//cdn.example.net/a.png",
        "https://cdn.example.net/a.png"
    )]
    #[case::scheme_relative_on_http(
        "http://example.com/",
        "//cdn.example.net/a.png",
        "http://cdn.example.net/a.png"
    )]
    #[case::absolute(
        "https://example.com/",
        "https://cdn.example.net/a.png",
        "https://cdn.example.net/a.png"
    )]
    #[case::surrounding_whitespace(
        "https://example.com/",
        "  /a.png\n",
        "https://example.com/a.png"
    )]
    fn resolves_addresses_like_a_browser(
        #[case] page_url: &str,
        #[case] address: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(resolve_in("", page_url, address).as_deref(), Some(expected));
    }

    #[rstest]
    #[case::leading_no_break_space("\u{00A0}/a.png", "https://example.com/blog/%C2%A0/a.png")]
    #[case::trailing_ideographic_space("/a.png\u{3000}", "https://example.com/a.png%E3%80%80")]
    #[case::tab_and_newline_inside("/a\t.p\nng", "https://example.com/a.png")]
    fn keeps_whitespace_a_browser_keeps(#[case] address: &str, #[case] expected: &str) {
        // Browsers strip only ASCII whitespace from the ends, and tabs and
        // newlines from inside, so other whitespace is part of the address.
        assert_eq!(
            resolve_in("", "https://example.com/blog/", address).as_deref(),
            Some(expected)
        );
    }

    #[rstest]
    #[case::empty("")]
    #[case::blank("   ")]
    #[case::blank_with_form_feed(" \u{000C}\t\r\n")]
    #[case::unparseable("https://[invalid/a.png")]
    #[case::data("data:image/png;base64,iVBORw0KGgo=")]
    #[case::javascript("javascript:alert(1)")]
    #[case::mailto("mailto:someone@example.com")]
    fn rejects_addresses_that_arent_on_the_web(#[case] address: &str) {
        assert_eq!(resolve_in("", "https://example.com/", address), None);
    }

    #[rstest]
    #[case::absolute(
        r#"<base href="https://static.example.net/assets/">"#,
        "https://static.example.net/assets/a.png"
    )]
    #[case::relative(r#"<base href="/assets/">"#, "https://example.com/assets/a.png")]
    #[case::first_of_several(
        r#"<base href="/first/"><base href="/second/">"#,
        "https://example.com/first/a.png"
    )]
    #[case::data(r#"<base href="data:text/plain,x">"#, "https://example.com/blog/a.png")]
    #[case::javascript(
        r#"<base href="javascript:void(0)">"#,
        "https://example.com/blog/a.png"
    )]
    #[case::data_in_capitals(
        r#"<base href="DATA:text/plain,x">"#,
        "https://example.com/blog/a.png"
    )]
    #[case::in_template(
        r#"<template><base href="/assets/"></template>"#,
        "https://example.com/blog/a.png"
    )]
    #[case::in_svg(
        r#"<svg><base href="/assets/"></base></svg>"#,
        "https://example.com/blog/a.png"
    )]
    #[case::in_foreign_object(
        r#"<svg><foreignObject><base href="/assets/"></foreignObject></svg>"#,
        "https://example.com/assets/a.png"
    )]
    fn resolves_against_the_base_element_a_browser_uses(
        #[case] head: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(
            resolve_in(head, "https://example.com/blog/", "a.png").as_deref(),
            Some(expected)
        );
    }

    #[test]
    fn lists_only_elements_a_browser_acts_on() {
        let html = r#"<link rel="a" href="/a">
            <template><link rel="b" href="/b"></template>
            <svg><link rel="c" href="/c"></link></svg>
            <link rel="d" href="/d">
            <svg><foreignObject><link rel="e" href="/e"></foreignObject></svg>
            <svg><template><foreignObject><link rel="f" href="/f"></foreignObject></template></svg>
            <math><link rel="g" href="/g"></link></math>"#;
        let links = HtmlLinks::parse(html, &url("https://example.com/"));
        let selector = Selector::parse("link").unwrap();
        let rels: Vec<_> = links
            .active_elements(&selector)
            .filter_map(|element| element.value().attr("rel"))
            .collect();
        // `e` and `f` are HTML elements in an SVG `<foreignObject>`. An SVG
        // `template` isn't inert, so `f` counts too.
        assert_eq!(rels, ["a", "d", "e", "f"]);
    }
}
