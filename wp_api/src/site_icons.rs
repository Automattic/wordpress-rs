//! Finding the icons a web page declares, such as a site's favicon.
//!
//! A page can declare several icons, each meant for a different place: a small
//! `<link rel="icon">` for a browser tab, a larger `apple-touch-icon` for a
//! home screen, and so on. WordPress declares four for a site with a site icon:
//!
//! ```html
//! <link rel="icon" href="…/cropped-icon-32x32.png" sizes="32x32" />
//! <link rel="icon" href="…/cropped-icon-192x192.png" sizes="192x192" />
//! <link rel="apple-touch-icon" href="…/cropped-icon-180x180.png" />
//! <meta name="msapplication-TileImage" content="…/cropped-icon-270x270.png" />
//! ```
//!
//! [`site_icons`] lists every icon a page declares. [`preferred_site_icon`]
//! picks the one to show at a given size, and falls back to `/favicon.ico` when
//! the page declares none that an app can show.

use crate::parsed_url::ParsedUrl;
use scraper::{ElementRef, Html, Selector};
use std::sync::Arc;
use url::Url;

/// Which tag declared a [`SiteIcon`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SiteIconKind {
    /// `<link rel="icon">`, including the older `rel="shortcut icon"`. The
    /// icon a browser shows in a tab. Often small, and often an `.ico` or SVG.
    Icon,
    /// `<link rel="apple-touch-icon">` or `rel="apple-touch-icon-precomposed"`.
    /// A square, opaque image for a home screen, usually 180×180 pixels.
    AppleTouchIcon,
    /// `<link rel="mask-icon">`. A single-colour SVG that Safari tints for
    /// pinned tabs. It is a shape, not an image to show as it is.
    MaskIcon,
    /// `<meta name="msapplication-TileImage">`. The image Windows shows on a
    /// Start tile, usually 144×144 pixels or larger.
    TileImage,
    /// `/favicon.ico` at the root of the page's origin. Browsers request it
    /// when a page declares no icon, so most sites have one, but the page
    /// doesn't say so: it can be missing.
    DefaultFavicon,
}

/// One entry of a `<link>` tag's `sizes` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SiteIconSize {
    /// `sizes="any"`: the icon scales to any size, as an SVG does.
    Any,
    /// `sizes="32x32"`.
    Pixels { width: u32, height: u32 },
}

/// An icon a web page declares.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SiteIcon {
    /// The icon's absolute `http` or `https` URL.
    pub url: String,
    pub kind: SiteIconKind,
    /// The sizes the tag lists, in order. Empty when it lists none.
    pub sizes: Vec<SiteIconSize>,
    /// The tag's `type` attribute, lowercased, such as `image/png`.
    pub mime_type: Option<String>,
}

/// Returns every icon `html` declares, in document order.
///
/// `page_url` is the address `html` was downloaded from, after redirects.
/// Relative icon addresses are resolved against it, or against the page's
/// `<base href>` when it has one, the way a browser resolves them. Character
/// references in attributes are decoded, so `?a=1&amp;b=2` becomes `?a=1&b=2`.
///
/// Icons whose address is empty, or isn't `http` or `https` (such as a `data:`
/// URL), are left out. The list doesn't include the implicit `/favicon.ico`;
/// [`preferred_site_icon`] falls back to it.
#[uniffi::export]
pub fn site_icons(html: String, page_url: Arc<ParsedUrl>) -> Vec<SiteIcon> {
    let document = Html::parse_document(&html);
    let base_url = document_base_url(&document, &page_url.inner);

    let selector = Selector::parse("link[rel][href], meta[name][content]")
        .expect("the icon selector is valid");
    document
        .select(&selector)
        .filter_map(|element| {
            let (kind, address) = icon_kind_and_address(element)?;
            let url = resolve(address, &base_url)?;
            let attributes = element.value();
            Some(SiteIcon {
                url: url.into(),
                kind,
                sizes: attributes
                    .attr("sizes")
                    .map(parse_sizes)
                    .unwrap_or_default(),
                mime_type: attributes
                    .attr("type")
                    .map(|t| t.trim().to_ascii_lowercase())
                    .filter(|t| !t.is_empty()),
            })
        })
        .collect()
}

/// Returns the icon in `html` that best fits a square of `size` pixels, or
/// `/favicon.ico` at the root of `page_url`'s origin when `html` declares no
/// icon an app can show.
///
/// It picks the smallest icon that is at least `size` pixels, so the image
/// is sharp without downloading more than needed. When every icon is smaller,
/// it picks the largest. An icon that doesn't list its size counts as:
///
/// - 180 pixels for an `apple-touch-icon`, the size Apple asks for.
/// - 144 pixels for a Windows tile image, the smallest size Windows asks for.
/// - 16 pixels for any other icon, the size of a classic favicon.
///
/// When two icons are the same size, an `apple-touch-icon` wins over an
/// `icon`, which wins over a tile image, and then the first in the document
/// wins. Apple touch icons are square and opaque, so they look right on any
/// background.
///
/// SVG icons and mask icons are never picked, because the image decoders on
/// Apple platforms and Android can't draw SVG. [`site_icons`] still lists them.
#[uniffi::export]
pub fn preferred_site_icon(html: String, page_url: Arc<ParsedUrl>, size: u32) -> SiteIcon {
    let candidates: Vec<(SiteIcon, u32)> = site_icons(html, page_url.clone())
        .into_iter()
        .filter(is_drawable)
        .map(|icon| {
            let pixels = effective_size(&icon);
            (icon, pixels)
        })
        .collect();

    // Ties are settled by kind, then by document order: `min_by_key` and
    // `max_by_key` disagree on which of two equal elements they return, so
    // the document position is part of the key.
    let rank = |(index, (icon, _)): &(usize, &(SiteIcon, u32))| (kind_rank(icon.kind), *index);
    let large_enough = candidates
        .iter()
        .enumerate()
        .filter(|(_, (_, pixels))| *pixels >= size)
        .min_by_key(|entry| (entry.1.1, rank(entry)));
    let best = large_enough.or_else(|| {
        candidates
            .iter()
            .enumerate()
            .min_by_key(|entry| (std::cmp::Reverse(entry.1.1), rank(entry)))
    });

    match best {
        Some((_, (icon, _))) => icon.clone(),
        None => default_favicon(&page_url.inner),
    }
}

/// `/favicon.ico` at the root of `page_url`'s origin, as a browser requests it.
fn default_favicon(page_url: &Url) -> SiteIcon {
    let url = page_url
        .join("/favicon.ico")
        .expect("joining an absolute path to a parsed URL succeeds");
    SiteIcon {
        url: url.into(),
        kind: SiteIconKind::DefaultFavicon,
        sizes: Vec::new(),
        mime_type: None,
    }
}

/// The URL relative addresses in `document` resolve against: the first
/// `<base href>`, resolved against `page_url`, or `page_url` itself.
fn document_base_url(document: &Html, page_url: &Url) -> Url {
    let selector = Selector::parse("base[href]").expect("the base selector is valid");
    document
        .select(&selector)
        .next()
        .and_then(|base| base.value().attr("href"))
        .and_then(|href| page_url.join(href).ok())
        .unwrap_or_else(|| page_url.clone())
}

fn icon_kind_and_address(element: ElementRef<'_>) -> Option<(SiteIconKind, &str)> {
    let attributes = element.value();
    match attributes.name() {
        "link" => {
            let kind = link_icon_kind(attributes.attr("rel")?)?;
            Some((kind, attributes.attr("href")?))
        }
        "meta" => attributes
            .attr("name")?
            .trim()
            .eq_ignore_ascii_case("msapplication-TileImage")
            .then(|| attributes.attr("content"))
            .flatten()
            .map(|content| (SiteIconKind::TileImage, content)),
        _ => None,
    }
}

/// The kind of icon a `<link>` tag's `rel` declares, if any. `rel` is a
/// case-insensitive list of tokens, so `rel="shortcut icon"` and
/// `rel="Icon preload"` both declare an icon.
fn link_icon_kind(rel: &str) -> Option<SiteIconKind> {
    let tokens: Vec<String> = rel
        .split_ascii_whitespace()
        .map(str::to_ascii_lowercase)
        .collect();
    let has = |token: &str| tokens.iter().any(|t| t == token);
    if has("apple-touch-icon") || has("apple-touch-icon-precomposed") {
        Some(SiteIconKind::AppleTouchIcon)
    } else if has("mask-icon") {
        Some(SiteIconKind::MaskIcon)
    } else if has("icon") {
        Some(SiteIconKind::Icon)
    } else {
        None
    }
}

/// Resolves `address` against `base_url`, keeping only `http` and `https`.
fn resolve(address: &str, base_url: &Url) -> Option<Url> {
    let address = address.trim();
    if address.is_empty() {
        return None;
    }
    base_url
        .join(address)
        .ok()
        .filter(|url| matches!(url.scheme(), "http" | "https"))
}

/// Parses a `sizes` attribute, such as `16x16 32x32` or `any`. Entries that
/// aren't valid are skipped.
fn parse_sizes(sizes: &str) -> Vec<SiteIconSize> {
    sizes
        .split_ascii_whitespace()
        .filter_map(|entry| {
            if entry.eq_ignore_ascii_case("any") {
                return Some(SiteIconSize::Any);
            }
            let (width, height) = entry.split_once(['x', 'X'])?;
            let width: u32 = width.parse().ok().filter(|w| *w > 0)?;
            let height: u32 = height.parse().ok().filter(|h| *h > 0)?;
            Some(SiteIconSize::Pixels { width, height })
        })
        .collect()
}

fn is_drawable(icon: &SiteIcon) -> bool {
    if icon.kind == SiteIconKind::MaskIcon {
        return false;
    }
    if icon.mime_type.as_deref() == Some("image/svg+xml") {
        return false;
    }
    let is_svg_file = Url::parse(&icon.url)
        .map(|url| url.path().to_ascii_lowercase().ends_with(".svg"))
        .unwrap_or(false);
    !is_svg_file
}

/// The size in pixels an icon is ranked by: the largest it lists, measured
/// by its shorter side, or a default for its kind when it lists none.
fn effective_size(icon: &SiteIcon) -> u32 {
    let listed = icon
        .sizes
        .iter()
        .filter_map(|size| match size {
            SiteIconSize::Pixels { width, height } => Some(*width.min(height)),
            SiteIconSize::Any => None,
        })
        .max();
    listed.unwrap_or(match icon.kind {
        SiteIconKind::AppleTouchIcon => 180,
        SiteIconKind::TileImage => 144,
        SiteIconKind::Icon | SiteIconKind::MaskIcon | SiteIconKind::DefaultFavicon => 16,
    })
}

fn kind_rank(kind: SiteIconKind) -> u8 {
    match kind {
        SiteIconKind::AppleTouchIcon => 0,
        SiteIconKind::Icon => 1,
        SiteIconKind::TileImage => 2,
        SiteIconKind::MaskIcon | SiteIconKind::DefaultFavicon => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn page(url: &str) -> Arc<ParsedUrl> {
        Arc::new(ParsedUrl::parse(url).expect("test URLs are valid"))
    }

    fn icons(html: &str) -> Vec<SiteIcon> {
        site_icons(html.to_string(), page("https://example.com/"))
    }

    fn urls(html: &str) -> Vec<String> {
        icons(html).into_iter().map(|icon| icon.url).collect()
    }

    fn preferred(html: &str, size: u32) -> SiteIcon {
        preferred_site_icon(html.to_string(), page("https://example.com/"), size)
    }

    /// What WordPress prints in `<head>` for a site with a site icon.
    const WORDPRESS_SITE_ICON: &str = r#"<!DOCTYPE html>
        <html><head>
        <link rel="icon" href="https://example.com/wp-content/uploads/cropped-icon-32x32.png" sizes="32x32" />
        <link rel="icon" href="https://example.com/wp-content/uploads/cropped-icon-192x192.png" sizes="192x192" />
        <link rel="apple-touch-icon" href="https://example.com/wp-content/uploads/cropped-icon-180x180.png" />
        <meta name="msapplication-TileImage" content="https://example.com/wp-content/uploads/cropped-icon-270x270.png" />
        </head><body></body></html>"#;

    #[test]
    fn lists_every_icon_wordpress_declares() {
        assert_eq!(
            icons(WORDPRESS_SITE_ICON),
            vec![
                SiteIcon {
                    url: "https://example.com/wp-content/uploads/cropped-icon-32x32.png".into(),
                    kind: SiteIconKind::Icon,
                    sizes: vec![SiteIconSize::Pixels {
                        width: 32,
                        height: 32
                    }],
                    mime_type: None,
                },
                SiteIcon {
                    url: "https://example.com/wp-content/uploads/cropped-icon-192x192.png".into(),
                    kind: SiteIconKind::Icon,
                    sizes: vec![SiteIconSize::Pixels {
                        width: 192,
                        height: 192
                    }],
                    mime_type: None,
                },
                SiteIcon {
                    url: "https://example.com/wp-content/uploads/cropped-icon-180x180.png".into(),
                    kind: SiteIconKind::AppleTouchIcon,
                    sizes: vec![],
                    mime_type: None,
                },
                SiteIcon {
                    url: "https://example.com/wp-content/uploads/cropped-icon-270x270.png".into(),
                    kind: SiteIconKind::TileImage,
                    sizes: vec![],
                    mime_type: None,
                },
            ]
        );
    }

    #[rstest]
    #[case::icon(r#"<link rel="icon" href="/a.png">"#, SiteIconKind::Icon)]
    #[case::shortcut_icon(r#"<link rel="shortcut icon" href="/a.png">"#, SiteIconKind::Icon)]
    #[case::uppercase(r#"<LINK REL="ICON" HREF="/a.png">"#, SiteIconKind::Icon)]
    #[case::extra_tokens(r#"<link rel="preload icon" href="/a.png">"#, SiteIconKind::Icon)]
    #[case::tabs_between_tokens(
        "<link rel=\"shortcut\ticon\" href=\"/a.png\">",
        SiteIconKind::Icon
    )]
    #[case::apple_touch_icon(
        r#"<link rel="apple-touch-icon" href="/a.png">"#,
        SiteIconKind::AppleTouchIcon
    )]
    #[case::apple_touch_icon_precomposed(
        r#"<link rel="apple-touch-icon-precomposed" href="/a.png">"#,
        SiteIconKind::AppleTouchIcon
    )]
    #[case::mask_icon(
        r#"<link rel="mask-icon" href="/a.png" color="red">"#,
        SiteIconKind::MaskIcon
    )]
    #[case::tile_image(
        r#"<meta name="msapplication-TileImage" content="/a.png">"#,
        SiteIconKind::TileImage
    )]
    #[case::tile_image_lowercase(
        r#"<meta name="msapplication-tileimage" content="/a.png">"#,
        SiteIconKind::TileImage
    )]
    fn recognises_icon_tags(#[case] html: &str, #[case] kind: SiteIconKind) {
        let found = icons(html);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].kind, kind);
        assert_eq!(found[0].url, "https://example.com/a.png");
    }

    #[rstest]
    #[case::stylesheet(r#"<link rel="stylesheet" href="/a.css">"#)]
    #[case::icon_as_part_of_a_word(r#"<link rel="iconic" href="/a.png">"#)]
    #[case::no_href(r#"<link rel="icon">"#)]
    #[case::empty_href(r#"<link rel="icon" href="">"#)]
    #[case::blank_href(r#"<link rel="icon" href="   ">"#)]
    #[case::data_url(r#"<link rel="icon" href="data:image/png;base64,iVBORw0KGgo=">"#)]
    #[case::javascript_url(r#"<link rel="icon" href="javascript:alert(1)">"#)]
    #[case::other_meta(r#"<meta name="theme-color" content="/a.png">"#)]
    #[case::open_graph_image(r#"<meta property="og:image" content="/a.png">"#)]
    fn ignores_tags_that_are_not_icons(#[case] html: &str) {
        assert_eq!(icons(html), vec![]);
    }

    // The quoting and attribute orders a hand-written regular expression misses.
    #[rstest]
    #[case::href_before_rel(r#"<link href="/a.png" rel="apple-touch-icon">"#)]
    #[case::single_quotes("<link rel='apple-touch-icon' href='/a.png'>")]
    #[case::unquoted("<link rel=apple-touch-icon href=/a.png>")]
    #[case::self_closing(r#"<link rel="apple-touch-icon" href="/a.png"/>"#)]
    #[case::angle_bracket_in_another_attribute(
        r#"<link title="a > b" rel="apple-touch-icon" href="/a.png">"#
    )]
    #[case::newlines_between_attributes("<link\n  rel=\"apple-touch-icon\"\n  href=\"/a.png\"\n>")]
    #[case::in_body(
        r#"<html><head></head><body><link rel="apple-touch-icon" href="/a.png"></body></html>"#
    )]
    fn reads_any_attribute_order_and_quoting(#[case] html: &str) {
        assert_eq!(urls(html), vec!["https://example.com/a.png"]);
    }

    #[test]
    fn decodes_character_references_in_the_address() {
        assert_eq!(
            urls(r#"<link rel="icon" href="/icon.php?size=32&amp;v=2">"#),
            vec!["https://example.com/icon.php?size=32&v=2"]
        );
    }

    // A tag or quote that is never closed swallows what follows it, up to the
    // next `>` or quote, in a browser too. The icons after that are still found.
    #[rstest]
    #[case::unclosed_tag(
        "<link rel=\"stylesheet\" href=\"/a.css\"\n<link rel=\"apple-touch-icon\" href=\"/lost.png\">"
    )]
    #[case::unclosed_quote(
        "<link rel=\"stylesheet\" href=\"/a.css>\n<link rel=\"apple-touch-icon\" href=\"/lost.png\">"
    )]
    fn reads_broken_markup_like_a_browser(#[case] broken: &str) {
        let html = format!("{broken}\n<p>text</p>\n<link rel=\"icon\" href=\"/after.png\">");
        assert_eq!(urls(&html), vec!["https://example.com/after.png"]);
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
        #[case] href: &str,
        #[case] expected: &str,
    ) {
        let html = format!(r#"<link rel="icon" href="{href}">"#);
        let found = site_icons(html, page(page_url));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].url, expected);
    }

    #[test]
    fn resolves_against_the_base_element() {
        let html = r#"<head>
            <base href="https://static.example.net/assets/">
            <link rel="icon" href="a.png">
            </head>"#;
        assert_eq!(urls(html), vec!["https://static.example.net/assets/a.png"]);
    }

    #[test]
    fn resolves_a_relative_base_against_the_page() {
        let html = r#"<base href="/assets/"><link rel="icon" href="a.png">"#;
        assert_eq!(urls(html), vec!["https://example.com/assets/a.png"]);
    }

    #[test]
    fn uses_only_the_first_base_element() {
        let html = r#"<base href="/first/"><base href="/second/"><link rel="icon" href="a.png">"#;
        assert_eq!(urls(html), vec!["https://example.com/first/a.png"]);
    }

    #[rstest]
    #[case::one("16x16", vec![SiteIconSize::Pixels { width: 16, height: 16 }])]
    #[case::several(
        "16x16 32x32",
        vec![SiteIconSize::Pixels { width: 16, height: 16 }, SiteIconSize::Pixels { width: 32, height: 32 }]
    )]
    #[case::uppercase_x("48X48", vec![SiteIconSize::Pixels { width: 48, height: 48 }])]
    #[case::not_square("120x60", vec![SiteIconSize::Pixels { width: 120, height: 60 }])]
    #[case::any("any", vec![SiteIconSize::Any])]
    #[case::any_uppercase("ANY", vec![SiteIconSize::Any])]
    #[case::invalid_entries_skipped(
        "big 0x0 32 x32 64x64",
        vec![SiteIconSize::Pixels { width: 64, height: 64 }]
    )]
    #[case::empty("", vec![])]
    fn parses_sizes(#[case] sizes: &str, #[case] expected: Vec<SiteIconSize>) {
        let html = format!(r#"<link rel="icon" href="/a.png" sizes="{sizes}">"#);
        assert_eq!(icons(&html)[0].sizes, expected);
    }

    #[test]
    fn keeps_the_mime_type() {
        let html = r#"<link rel="icon" href="/a.png" type=" Image/PNG ">"#;
        assert_eq!(icons(html)[0].mime_type.as_deref(), Some("image/png"));
    }

    // MARK: - Picking an icon

    #[rstest]
    #[case::smallest_large_enough(180, "cropped-icon-180x180.png", SiteIconKind::AppleTouchIcon)]
    #[case::exact_size(192, "cropped-icon-192x192.png", SiteIconKind::Icon)]
    #[case::small(24, "cropped-icon-32x32.png", SiteIconKind::Icon)]
    // The tile image is 270 pixels, but doesn't say so, so it counts as 144.
    #[case::larger_than_all(512, "cropped-icon-192x192.png", SiteIconKind::Icon)]
    fn picks_from_a_wordpress_site_icon(
        #[case] size: u32,
        #[case] file: &str,
        #[case] kind: SiteIconKind,
    ) {
        let icon = preferred(WORDPRESS_SITE_ICON, size);
        assert_eq!(
            icon.url,
            format!("https://example.com/wp-content/uploads/{file}")
        );
        assert_eq!(icon.kind, kind);
    }

    #[test]
    fn falls_back_to_favicon_ico_at_the_root() {
        let icon = preferred_site_icon(
            "<html><head><title>No icons</title></head></html>".into(),
            page("https://example.com/blog/post/?p=1"),
            180,
        );
        assert_eq!(
            icon,
            SiteIcon {
                url: "https://example.com/favicon.ico".into(),
                kind: SiteIconKind::DefaultFavicon,
                sizes: vec![],
                mime_type: None,
            }
        );
    }

    #[test]
    fn falls_back_to_favicon_ico_for_an_empty_page() {
        let icon = preferred_site_icon(String::new(), page("http://example.com:8080/"), 180);
        assert_eq!(icon.url, "http://example.com:8080/favicon.ico");
    }

    #[test]
    fn prefers_any_declared_icon_to_the_fallback() {
        let icon = preferred(r#"<link rel="shortcut icon" href="/favicon.png">"#, 180);
        assert_eq!(icon.url, "https://example.com/favicon.png");
        assert_eq!(icon.kind, SiteIconKind::Icon);
    }

    #[test]
    fn prefers_an_apple_touch_icon_to_an_icon_without_sizes() {
        let html = r#"<link rel="icon" href="/favicon.png">
            <link rel="apple-touch-icon" href="/touch.png">"#;
        assert_eq!(preferred(html, 64).url, "https://example.com/touch.png");
    }

    #[test]
    fn prefers_an_apple_touch_icon_of_the_same_size() {
        let html = r#"<link rel="icon" href="/icon.png" sizes="180x180">
            <link rel="apple-touch-icon" href="/touch.png" sizes="180x180">"#;
        assert_eq!(preferred(html, 180).url, "https://example.com/touch.png");
        assert_eq!(preferred(html, 512).url, "https://example.com/touch.png");
    }

    #[test]
    fn picks_the_first_of_two_equal_icons() {
        let html = r#"<link rel="icon" href="/first.png" sizes="64x64">
            <link rel="icon" href="/second.png" sizes="64x64">"#;
        assert_eq!(preferred(html, 32).url, "https://example.com/first.png");
        assert_eq!(preferred(html, 128).url, "https://example.com/first.png");
    }

    #[test]
    fn ranks_an_ico_by_its_largest_size() {
        let html = r#"<link rel="icon" href="/favicon.ico" sizes="16x16 32x32 48x48">
            <link rel="icon" href="/icon-40.png" sizes="40x40">"#;
        assert_eq!(preferred(html, 44).url, "https://example.com/favicon.ico");
    }

    #[rstest]
    #[case::svg_type(r#"<link rel="icon" href="/icon" type="image/svg+xml">"#)]
    #[case::svg_file(r#"<link rel="icon" href="/icon.SVG?v=2">"#)]
    #[case::svg_with_any_size(r#"<link rel="icon" href="/icon.svg" sizes="any">"#)]
    #[case::mask_icon(r#"<link rel="mask-icon" href="/mask.png" color="black">"#)]
    fn never_picks_an_icon_apps_cant_draw(#[case] html: &str) {
        assert_eq!(preferred(html, 32).kind, SiteIconKind::DefaultFavicon);
        let with_png = format!(r#"{html}<link rel="icon" href="/fallback.png" sizes="16x16">"#);
        assert_eq!(
            preferred(&with_png, 32).url,
            "https://example.com/fallback.png"
        );
    }

    #[test]
    fn picks_a_large_raster_icon_alongside_an_svg() {
        // GitHub-style markup: an SVG for browsers that support it, PNGs for the rest.
        let html = r#"<link rel="icon" type="image/svg+xml" href="/favicon.svg">
            <link rel="icon" type="image/png" href="/favicon-32.png" sizes="32x32">
            <link rel="apple-touch-icon" href="/apple-touch-icon.png" sizes="180x180">"#;
        assert_eq!(
            preferred(html, 120).url,
            "https://example.com/apple-touch-icon.png"
        );
    }

    #[test]
    fn ranks_a_wide_icon_by_its_shorter_side() {
        let html = r#"<link rel="icon" href="/wide.png" sizes="512x64">
            <link rel="icon" href="/square.png" sizes="128x128">"#;
        assert_eq!(preferred(html, 100).url, "https://example.com/square.png");
    }
}
