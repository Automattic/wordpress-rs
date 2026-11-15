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

use crate::parsed_url::{ParsedUrl, rfc3986_serialization};
use scraper::{ElementRef, Html, Selector};
use std::cmp::Reverse;
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
    /// The icon's absolute `http` or `https` URL. Characters RFC 3986 doesn't
    /// allow, such as `{` or `|`, are percent-encoded, so strict parsers like
    /// Foundation's `URL` and Java's `URI` accept it.
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
/// URL), are left out, as are tags a browser ignores: those inside a
/// `<template>`, an `<svg>` or a `<math>`. The list doesn't include the
/// implicit `/favicon.ico`; [`preferred_site_icon`] falls back to it.
#[uniffi::export]
pub fn site_icons(html: String, page_url: Arc<ParsedUrl>) -> Vec<SiteIcon> {
    declared_icons(&html, &page_url.inner)
        .into_iter()
        .map(|declared| declared.icon)
        .collect()
}

/// Returns the icon in `html` that best fits a square of `size` pixels, or
/// `/favicon.ico` at the root of `page_url`'s origin when `html` declares no
/// icon an app can show.
///
/// Returns `None` when `html` declares no icon an app can show and `page_url`
/// isn't `http` or `https`, such as `about:blank` or a `file:` URL, because
/// there is no origin to request `/favicon.ico` from.
///
/// It picks the smallest icon that is at least `size` pixels, so the image
/// is sharp without downloading more than needed. When every icon is smaller,
/// it picks the largest. An icon that doesn't list a size in pixels counts as
/// the size at the end of its file name, such as `cropped-icon-270x270.png`
/// (the way WordPress names them), or else as:
///
/// - 180 pixels for an `apple-touch-icon`, the size Apple asks for.
/// - 144 pixels for a Windows tile image, the smallest size Windows asks for.
/// - 16 pixels for any other icon, the size of a classic favicon.
///
/// `sizes="any"` doesn't count as a size: it's meant for SVG icons, which are
/// never picked, and an `.ico` that lists it doesn't scale.
///
/// A Windows tile image is made for a Start tile, often on a transparent or
/// coloured background, so it's only picked when no other icon is large
/// enough, or when every icon is too small and it's the largest.
///
/// When two icons are the same size, an `apple-touch-icon` wins over an
/// `icon`, which wins over a tile image. Apple touch icons are square and
/// opaque, so they look right on any background. Then, as in the HTML
/// standard, the last in the document wins.
///
/// SVG icons and mask icons are never picked, because the image decoders on
/// Apple platforms and Android can't draw SVG. [`site_icons`] still lists them.
#[uniffi::export]
pub fn preferred_site_icon(html: String, page_url: Arc<ParsedUrl>, size: u32) -> Option<SiteIcon> {
    // The lowest key wins. In order, it sorts:
    // 1. Icons of at least `size` pixels before smaller ones.
    // 2. Among those, icons before tile images.
    // 3. The smallest first among those large enough; the largest first
    //    among smaller ones.
    // 4. Then by kind, then the last in the document first.
    declared_icons(&html, &page_url.inner)
        .into_iter()
        .enumerate()
        .filter(|(_, declared)| !is_svg(declared))
        .filter_map(|(index, declared)| {
            let kind_rank = kind_rank(declared.icon.kind)?;
            let pixels = effective_size(&declared);
            let too_small = pixels < size;
            let tile_that_fits = !too_small && declared.icon.kind == SiteIconKind::TileImage;
            let fit = if too_small { u32::MAX - pixels } else { pixels };
            let key = (too_small, tile_that_fits, fit, kind_rank, Reverse(index));
            Some((key, declared.icon))
        })
        .min_by_key(|(key, _)| *key)
        .map(|(_, icon)| icon)
        .or_else(|| default_favicon(&page_url.inner))
}

/// An icon `html` declares, with its address still parsed.
struct DeclaredIcon {
    url: Url,
    icon: SiteIcon,
}

fn declared_icons(html: &str, page_url: &Url) -> Vec<DeclaredIcon> {
    let document = Html::parse_document(html);
    let base_url = document_base_url(&document, page_url);

    let selector = Selector::parse("link[rel][href], meta[name][content]")
        .expect("the icon selector is valid");
    document
        .select(&selector)
        .filter(|element| is_active_html_element(*element))
        .filter_map(|element| {
            let (kind, address) = icon_kind_and_address(element)?;
            let url = resolve(address, &base_url)?;
            let attributes = element.value();
            let icon = SiteIcon {
                url: rfc3986_serialization(&url),
                kind,
                sizes: attributes
                    .attr("sizes")
                    .map(parse_sizes)
                    .unwrap_or_default(),
                mime_type: attributes
                    .attr("type")
                    .map(|t| t.trim().to_ascii_lowercase())
                    .filter(|t| !t.is_empty()),
            };
            Some(DeclaredIcon { url, icon })
        })
        .collect()
}

/// `/favicon.ico` at the root of `page_url`'s origin, as a browser requests it,
/// or `None` when `page_url` isn't `http` or `https`.
fn default_favicon(page_url: &Url) -> Option<SiteIcon> {
    let url = resolve("/favicon.ico", page_url)?;
    Some(SiteIcon {
        url: rfc3986_serialization(&url),
        kind: SiteIconKind::DefaultFavicon,
        sizes: Vec::new(),
        mime_type: None,
    })
}

/// The URL relative addresses in `document` resolve against: the first
/// `<base href>`, resolved against `page_url`, or `page_url` itself.
///
/// As in the HTML standard, a base that doesn't parse, or is a `data:` or
/// `javascript:` URL, is ignored, and so is one a browser ignores because
/// it's inside a `<template>` or an `<svg>`.
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
/// MathML one, and it isn't inside a `<template>`, whose contents are inert.
fn is_active_html_element(element: ElementRef<'_>) -> bool {
    let is_html = &*element.value().name.ns == "http://www.w3.org/1999/xhtml";
    let in_template = element.ancestors().any(|node| {
        node.value()
            .as_element()
            .is_some_and(|ancestor| ancestor.name() == "template")
    });
    is_html && !in_template
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
            let (width, height) = parse_dimensions(entry)?;
            Some(SiteIconSize::Pixels { width, height })
        })
        .collect()
}

/// Parses `WxH`, such as `32x32` or `310X150`, into a non-zero width and height.
fn parse_dimensions(text: &str) -> Option<(u32, u32)> {
    let (width, height) = text.split_once(['x', 'X'])?;
    let width: u32 = width.parse().ok().filter(|w| *w > 0)?;
    let height: u32 = height.parse().ok().filter(|h| *h > 0)?;
    Some((width, height))
}

/// Whether an icon is an SVG, by its `type` or its file extension.
fn is_svg(declared: &DeclaredIcon) -> bool {
    let icon = &declared.icon;
    // Compare the MIME type's essence, without parameters such as
    // `; charset=utf-8`. `mime_type` is already lowercased.
    let is_svg_type = icon
        .mime_type
        .as_deref()
        .and_then(|mime_type| mime_type.split(';').next())
        .is_some_and(|essence| essence.trim() == "image/svg+xml");
    let path = declared.url.path().to_ascii_lowercase();
    let is_svg_file = path.ends_with(".svg") || path.ends_with(".svgz");
    is_svg_type || is_svg_file
}

/// The size in pixels an icon is ranked by, measured by its shorter side.
/// In order:
///
/// 1. The largest size in pixels the icon lists.
/// 2. The size at the end of its file name, such as `icon-270x270.png` or
///    `icon-32x32@2x.png`.
/// 3. A default for its kind.
fn effective_size(declared: &DeclaredIcon) -> u32 {
    let icon = &declared.icon;
    let listed = icon
        .sizes
        .iter()
        .filter_map(|size| match size {
            SiteIconSize::Pixels { width, height } => Some(*width.min(height)),
            SiteIconSize::Any => None,
        })
        .max();
    listed
        .or_else(|| size_in_file_name(&declared.url))
        .unwrap_or(match icon.kind {
            SiteIconKind::AppleTouchIcon => 180,
            SiteIconKind::TileImage => 144,
            SiteIconKind::Icon | SiteIconKind::MaskIcon | SiteIconKind::DefaultFavicon => 16,
        })
}

/// The shorter side of a `-WxH` or `_WxH` size at the end of `url`'s file
/// name, before the extension: 270 for `cropped-icon-270x270.png`.
///
/// A trailing `@2x` or `@3x` follows Apple's convention, where the size is in
/// points and the scale multiplies it: 64 for `icon-32x32@2x.png`.
fn size_in_file_name(url: &Url) -> Option<u32> {
    let file_name = url.path_segments()?.next_back()?;
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    let (stem, scale) = match stem.rsplit_once('@') {
        Some((stem, scale)) => {
            let scale: u32 = scale.strip_suffix(['x', 'X'])?.parse().ok()?;
            (stem, scale)
        }
        None => (stem, 1),
    };
    let (_, size) = stem.rsplit_once(['-', '_'])?;
    let (width, height) = parse_dimensions(size)?;
    width
        .min(height)
        .checked_mul(scale)
        .filter(|pixels| *pixels > 0)
}

/// How a kind ranks when two icons are the same size, lowest first, or `None`
/// for a kind that is never picked. A mask icon is a shape for Safari to tint,
/// not an image to show, and the default favicon is only the fallback.
fn kind_rank(kind: SiteIconKind) -> Option<u8> {
    match kind {
        SiteIconKind::AppleTouchIcon => Some(0),
        SiteIconKind::Icon => Some(1),
        SiteIconKind::TileImage => Some(2),
        SiteIconKind::MaskIcon | SiteIconKind::DefaultFavicon => None,
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
            .expect("an https page always has an icon")
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
    // The tile image doesn't list its size, but its file name says 270 pixels.
    #[case::between_icon_and_tile(256, "cropped-icon-270x270.png", SiteIconKind::TileImage)]
    #[case::larger_than_all(512, "cropped-icon-270x270.png", SiteIconKind::TileImage)]
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
            Some(SiteIcon {
                url: "https://example.com/favicon.ico".into(),
                kind: SiteIconKind::DefaultFavicon,
                sizes: vec![],
                mime_type: None,
            })
        );
    }

    #[test]
    fn falls_back_to_favicon_ico_for_an_empty_page() {
        let icon = preferred_site_icon(String::new(), page("http://example.com:8080/"), 180);
        assert_eq!(
            icon.map(|icon| icon.url),
            Some("http://example.com:8080/favicon.ico".to_string())
        );
    }

    #[rstest]
    #[case::about_blank("about:blank")]
    #[case::data("data:text/html,hi")]
    #[case::mailto("mailto:someone@example.com")]
    #[case::file("file:///tmp/page.html")]
    fn has_no_fallback_for_a_page_that_isnt_on_the_web(#[case] page_url: &str) {
        assert_eq!(preferred_site_icon(String::new(), page(page_url), 32), None);
    }

    #[test]
    fn picks_a_declared_icon_on_a_page_that_isnt_on_the_web() {
        let icon = preferred_site_icon(
            r#"<link rel="icon" href="https://example.com/a.png">"#.into(),
            page("about:blank"),
            32,
        );
        assert_eq!(
            icon.map(|icon| icon.url),
            Some("https://example.com/a.png".to_string())
        );
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
    fn picks_the_last_of_two_equal_icons() {
        // The HTML standard says to use the last of several equally good icons.
        let html = r#"<link rel="icon" href="/first.png" sizes="64x64">
            <link rel="icon" href="/second.png" sizes="64x64">"#;
        assert_eq!(preferred(html, 32).url, "https://example.com/second.png");
        assert_eq!(preferred(html, 128).url, "https://example.com/second.png");
    }

    #[test]
    fn picks_a_site_icon_printed_after_a_theme_default() {
        let html = r#"<link rel="shortcut icon" href="/theme-default.ico">
            <link rel="icon" href="/site.png">"#;
        assert_eq!(preferred(html, 32).url, "https://example.com/site.png");
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
    #[case::svg_type_with_parameters(
        r#"<link rel="icon" href="/icon" type="image/svg+xml; charset=utf-8" sizes="64x64">"#
    )]
    #[case::svgz_file(r#"<link rel="icon" href="/icon.svgz" sizes="64x64">"#)]
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

    #[rstest]
    #[case::data("data:text/plain,x")]
    #[case::javascript("javascript:void(0)")]
    #[case::data_in_capitals("DATA:text/plain,x")]
    fn ignores_a_data_or_javascript_base(#[case] base: &str) {
        // The HTML standard ignores these, and resolves against the page instead.
        let html = format!(r#"<base href="{base}"><link rel="icon" href="/a.png">"#);
        assert_eq!(urls(&html), vec!["https://example.com/a.png"]);
    }

    #[rstest]
    #[case(32)]
    #[case(64)]
    #[case(120)]
    #[case(256)]
    fn ignores_any_on_an_icon_that_doesnt_scale(#[case] size: u32) {
        // From "How to Favicon in 2021": `sizes="any"` on the `.ico` only tells
        // browsers to prefer the SVG. An `.ico` doesn't scale.
        let html = r#"<link rel="icon" href="/favicon.ico" sizes="any">
            <link rel="icon" href="/icon.svg" type="image/svg+xml">
            <link rel="apple-touch-icon" href="/apple-touch-icon.png">"#;
        assert_eq!(
            preferred(html, size).url,
            "https://example.com/apple-touch-icon.png"
        );
    }

    #[rstest]
    #[case(64)]
    #[case(120)]
    #[case(144)]
    fn prefers_an_icon_to_a_tile_image_that_also_fits(#[case] size: u32) {
        // RealFaviconGenerator's markup.
        let html = r##"<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
            <link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png">
            <link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png">
            <link rel="mask-icon" href="/safari-pinned-tab.svg" color="#5bbad5">
            <meta name="msapplication-TileImage" content="/mstile-144x144.png">"##;
        assert_eq!(
            preferred(html, size).url,
            "https://example.com/apple-touch-icon.png"
        );
    }

    #[rstest]
    // No icon fits, so the largest wins, even though it's a tile image.
    #[case::larger_than_all(512)]
    // Only the tile image fits.
    #[case::only_the_tile_fits(200)]
    fn picks_a_tile_image_when_nothing_else_will_do(#[case] size: u32) {
        assert_eq!(
            preferred(WORDPRESS_SITE_ICON, size).url,
            "https://example.com/wp-content/uploads/cropped-icon-270x270.png"
        );
    }

    #[test]
    fn ignores_icons_inside_template_and_svg() {
        let html = r#"<template><link rel="icon" href="/template.png"></template>
            <svg><link rel="icon" href="/svg.png"></svg>
            <math><link rel="icon" href="/math.png"></math>
            <link rel="icon" href="/real.png">"#;
        assert_eq!(urls(html), vec!["https://example.com/real.png"]);
    }

    #[test]
    fn ignores_a_base_inside_template() {
        let html = r#"<template><base href="https://evil.example/x/"></template>
            <link rel="icon" href="real.png">"#;
        assert_eq!(urls(html), vec!["https://example.com/real.png"]);
    }

    #[rstest]
    #[case::wordpress("https://example.com/cropped-icon-270x270.png", 270)]
    #[case::underscore("https://example.com/favicon_96x96.png", 96)]
    #[case::capital_x("https://example.com/tile-310X150.png", 150)]
    #[case::query_ignored("https://example.com/tile-558x558.png?v=2", 558)]
    #[case::no_size_in_name("https://example.com/mstile.png", 144)]
    #[case::size_in_directory("https://example.com/64x64/tile.png", 144)]
    #[case::no_separator("https://example.com/tile64x64.png", 144)]
    // Apple's convention: the size is in points, and `@2x` doubles it.
    #[case::at_1x("https://example.com/icon-32x32@1x.png", 32)]
    #[case::at_2x("https://example.com/icon-32x32@2x.png", 64)]
    #[case::at_3x("https://example.com/icon-32x32@3x.png", 96)]
    #[case::scale_without_size("https://example.com/icon@2x.png", 144)]
    #[case::scale_of_zero("https://example.com/icon-32x32@0x.png", 144)]
    #[case::scale_that_overflows("https://example.com/icon-4294967295x4294967295@2x.png", 144)]
    fn ranks_an_unsized_icon_by_the_size_in_its_file_name(
        #[case] url: &str,
        #[case] expected: u32,
    ) {
        let declared = DeclaredIcon {
            url: Url::parse(url).expect("test URLs are valid"),
            icon: SiteIcon {
                url: url.into(),
                kind: SiteIconKind::TileImage,
                sizes: vec![],
                mime_type: None,
            },
        };
        assert_eq!(effective_size(&declared), expected);
    }

    #[test]
    fn prefers_listed_sizes_to_the_file_name() {
        let html = r#"<link rel="icon" href="/icon-512x512.png" sizes="32x32">
            <link rel="icon" href="/icon-64.png" sizes="64x64">"#;
        assert_eq!(preferred(html, 48).url, "https://example.com/icon-64.png");
    }

    #[rstest]
    #[case::query(
        r#"<link rel="icon" href="/icon.png?v={hash}|2^x">"#,
        "https://example.com/icon.png?v=%7Bhash%7D%7C2%5Ex"
    )]
    #[case::host_placeholder(
        r#"<link rel="icon" href="https://{{cdn}}/favicon.png">"#,
        "https://%7B%7Bcdn%7D%7D/favicon.png"
    )]
    fn encodes_characters_rfc3986_doesnt_allow(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(urls(html), vec![expected]);
    }
}
