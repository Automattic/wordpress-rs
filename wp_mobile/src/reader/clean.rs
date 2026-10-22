use html5ever::{LocalName, QualName, ns};
use scraper::{
    ElementRef, Html, Node,
    node::{Element, Text},
};
use url::Url;

use super::embeds;
use super::getty::build_getty_embeds;
use super::gists::rebuild_gists;
use super::images::{parse_dimension, parse_size, rewrite_srcset, srcset_candidates};
use super::text::plain_text;

/// Cleans up a post from the Reader API in place, and returns the scripts its embeds need.
/// `post_url` is the post's address, for resolving relative URLs.
///
/// [`ReaderPostContent::new`](super::ReaderPostContent::new) lists what this does.
pub(super) fn clean(document: &mut Html, post_url: Option<&Url>) -> Vec<String> {
    // Getty's embeds come from an inline script, which the cleanup removes below.
    build_getty_embeds(document);
    rebuild_gists(document);

    let mut scripts = Vec::new();
    let mut removed = Vec::new();
    let mut emoji = Vec::new();
    let mut iframes = Vec::new();
    let mut videos = Vec::new();
    let mut links = Vec::new();
    let mut tiles = Vec::new();

    for node in document.tree.root().descendants() {
        match node.value() {
            Node::Comment(_) => removed.push(node.id()),
            Node::Element(element) => match element.name() {
                "script" => {
                    if let Some(script) = element.attr("src").and_then(post_script_url) {
                        push_unique(&mut scripts, script);
                    }
                    removed.push(node.id());
                }
                // The apps always run scripts, so a `<noscript>` never shows. Its markup is only text.
                "style" | "noscript" => removed.push(node.id()),
                // These act on the whole page: a `<base>` would change where its relative links go,
                // a `<link>` would load another stylesheet, and a `<meta>` can redirect it. A
                // `<meta itemprop>` is only data about the post, like an image's size.
                "base" | "link" => removed.push(node.id()),
                "meta" if !is_microdata(element) => removed.push(node.id()),
                // An SVG animation can change a link, or add an event handler, after the cleanup
                // has checked it.
                "animate" | "set" | "animateMotion" | "animateTransform"
                    if element.attr("attributeName").is_some_and(|target| {
                        let target = target.trim().trim_start_matches("xlink:");
                        is_url_attribute(target) || is_event_handler(target)
                    }) =>
                {
                    removed.push(node.id())
                }
                "div" if is_cover_video(element) => videos.push(node.id()),
                "div" => {
                    if let Some(url) = ElementRef::wrap(node).and_then(unresolved_embed_url) {
                        links.push((node.id(), url));
                    }
                    if let Some(weight) = tile_weight(element) {
                        tiles.push((node.id(), weight));
                    }
                }
                "img" => {
                    if let Some(character) = emoji_character(element) {
                        emoji.push((node.id(), character.to_owned()));
                    }
                }
                "iframe" if !loads_a_web_page(element) => removed.push(node.id()),
                "iframe" => {
                    if let Some(size) = ElementRef::wrap(node).and_then(iframe_size) {
                        iframes.push((node.id(), size));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    for script in embeds::missing_scripts(document, &scripts) {
        push_unique(&mut scripts, script.to_owned());
    }

    // Detaching a node takes its subtree with it, so a node can come up here after its parent
    // has already gone. Detaching it again is harmless.
    for id in removed {
        document
            .tree
            .get_mut(id)
            .expect("the ID came from this tree")
            .detach();
    }

    // After the removals, which can leave a paragraph empty, like one that only held a script.
    let empty: Vec<_> = document
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "p" && is_empty_paragraph(*element))
        .map(|element| element.id())
        .collect();
    for id in empty {
        document
            .tree
            .get_mut(id)
            .expect("the ID came from this tree")
            .detach();
    }

    for (id, character) in emoji {
        let mut image = document
            .tree
            .get_mut(id)
            .expect("the ID came from this tree");
        image.insert_before(Node::Text(Text {
            text: character.into(),
        }));
        image.detach();
    }

    for id in videos {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            element.name.local = LocalName::from("video");
        }
    }

    for (id, url) in links {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            element.name.local = LocalName::from("a");
            set_attribute(element, "href", &url);
        }
    }

    let elements: Vec<_> = document
        .tree
        .root()
        .descendants()
        .filter(|node| node.value().is_element())
        .map(|node| node.id())
        .collect();
    for id in elements {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            clean_attributes(element, post_url);
        }
    }

    // The cleanup removed the post's own styles, so these are the only ones.
    for (id, weight) in tiles {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            set_attribute(element, "style", &format!("flex-grow: {weight};"));
        }
    }
    for (id, size) in iframes {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            set_attribute(element, "style", &size.style());
        }
    }

    wrap_loose_inline_content(document);

    scripts
}

/// How many times [`reparse`] parses a post again before giving up on its markup.
pub(super) const REPARSE_ROUNDS: usize = 4;

/// The cleaned post, the way the web view will see it.
///
/// Some markup parses into a tree that its own HTML doesn't parse back into: a `<form>` inside a
/// `<form>`, or an element the parser puts somewhere it can't go. The web view parses the cleaned
/// HTML again, so what it sees can differ from what the cleanup checked. This parses it again, and
/// cleans the result, until parsing it changes nothing. A post that's still changing after `rounds`
/// parses keeps only its text.
pub(super) fn reparse(
    document: Html,
    scripts: &mut Vec<String>,
    post_url: Option<&Url>,
    rounds: usize,
) -> Html {
    let mut document = document;
    for _ in 0..rounds {
        let html = document.root_element().inner_html();
        let reparsed = Html::parse_fragment(&html);
        if reparsed.root_element().inner_html() == html {
            return reparsed;
        }
        document = reparsed;
        for script in clean(&mut document, post_url) {
            push_unique(scripts, script);
        }
    }

    let paragraphs: String = plain_text(document.root_element())
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let line = line
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            format!("<p>{line}</p>")
        })
        .collect();
    Html::parse_fragment(&paragraphs)
}

fn clean_attributes(element: &mut Element, post_url: Option<&Url>) {
    load_lazy_image(element);
    size_image(element);
    let is_media = matches!(element.name(), "video" | "audio");
    let is_iframe = element.name() == "iframe";

    element.attrs.retain(|(name, _)| match &*name.local {
        "style" => false,
        "autoplay" => !is_media,
        // An iframe shows its `srcdoc` rather than its `src`, and runs it as part of the page.
        "srcdoc" => !is_iframe,
        // Event handlers, like `onclick`, would run as part of the page. Some posts have them.
        name => !is_event_handler(name),
    });

    let tag = element.name.local.clone();
    for (name, value) in &mut element.attrs {
        let upgrades = upgrades_to_https(&tag, &name.local);
        let fix_url = |url: &str| {
            let fixed = https_if_protocol_relative(url).or_else(|| resolve(url, post_url?));
            if !upgrades {
                return fixed;
            }
            https_if_http(fixed.as_deref().unwrap_or(url)).or(fixed)
        };
        let fixed = match &*name.local {
            "href" if &*tag == "a" => {
                let fixed = fix_url(value);
                post_url
                    .and_then(|post_url| in_page_link(fixed.as_deref().unwrap_or(value), post_url))
                    .or(fixed)
            }
            "src" | "href" | "poster" | "data-orig-file" => fix_url(value),
            "srcset" => Some(rewrite_srcset(value, fix_url)),
            _ => None,
        };
        if let Some(fixed) = fixed {
            *value = fixed.into();
        }
    }

    // A URL that would run something, like `javascript:`, rather than load or go somewhere. Any
    // element can have one: forms submit to them, `reader.js` links images to theirs, and MathML
    // elements can be links.
    element.attrs.retain(|(name, value)| {
        !is_url_attribute(&name.local) || is_safe_url(&tag, &name.local, value)
    });

    if is_media && element.attr("controls").is_none() {
        set_attribute(element, "controls", "");
    }
    if is_iframe {
        replace_flickr_player(element);
        sandbox_iframe(element);
    }
}

/// Points Flickr's photo player, which Jetpack's `[flickr]` shortcode and Flickr's older embed code
/// use, at Flickr's embed of the same photo. In Chromium, the player reloads itself a few times a
/// second where Flickr asks for cookie consent: its consent script can't keep what it saves in an
/// iframe on another site, so it saves it again and reloads. The embed has no consent script.
fn replace_flickr_player(element: &mut Element) {
    if let Some(src) = element.attr("src").and_then(flickr_embed_url) {
        set_attribute(element, "src", &src);
    }
}

/// The embed for a Flickr player URL, like `https://www.flickr.com/photos/nasa/123/player/`. The
/// photo's secret, which older embed code puts after `player/`, isn't needed.
fn flickr_embed_url(src: &str) -> Option<String> {
    let url = Url::parse(src).ok()?;
    if !matches!(url.host_str()?, "www.flickr.com" | "flickr.com") {
        return None;
    }
    let segments: Vec<_> = url.path_segments()?.collect();
    let ["photos", _, id, "player", ..] = segments.as_slice() else {
        return None;
    };
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("https://embedr.flickr.com/photos/{id}"))
}

/// What an iframe without a `sandbox` of its own may do: everything an iframe can, apart from
/// navigating the whole page. Its links still open in a new window, which the app can handle like
/// the post's own links.
pub(super) const IFRAME_SANDBOX: &str = "allow-downloads allow-forms allow-modals allow-orientation-lock \
    allow-pointer-lock allow-popups allow-popups-to-escape-sandbox allow-presentation \
    allow-same-origin allow-scripts allow-storage-access-by-user-activation";

/// Stops an iframe from navigating the whole page. An embed that could would replace the post in
/// the app's web view with a page of its own, with no address bar to show whose.
///
/// An iframe without a `sandbox` gets [`IFRAME_SANDBOX`], so it keeps everything else it could do.
/// One with a `sandbox`, as WordPress gives YouTube's and its own post embeds, keeps it, without
/// any `allow-top-navigation`. None of 448 iframes in real posts allowed it.
fn sandbox_iframe(element: &mut Element) {
    let sandbox = match element.attr("sandbox") {
        Some(sandbox) => sandbox
            .split_ascii_whitespace()
            .filter(|token| {
                !token
                    .to_ascii_lowercase()
                    .starts_with("allow-top-navigation")
            })
            .collect::<Vec<_>>()
            .join(" "),
        None => IFRAME_SANDBOX.to_owned(),
    };
    set_attribute(element, "sandbox", &sandbox);
}

/// Gives an image its `width` and `height` when the post leaves them out, but they're known. The page
/// then keeps the image's space while it loads, rather than moving the text below when it arrives.
///
/// They're the size the page shows, which isn't always the original's. Giving a smaller copy the
/// original's size would stretch it. Of 773 images without a size in 769 real posts, 54 have one
/// this finds. The rest are mostly on other hosts, with nothing in the post that says.
///
/// Of the 53 sized by their file names, 52 matched the file, and one couldn't be fetched. An
/// original whose name happens to end like a size, like the Theme Unit Test's
/// `image-alignment-1200x4002.jpg`, gets the wrong space until it loads, and then its own.
fn size_image(element: &mut Element) {
    if element.name() != "img" {
        return;
    }
    let dimension = |name| {
        element
            .attr(name)
            .and_then(parse_dimension)
            .filter(|size| *size > 0)
    };
    let (width, height) = (dimension("width"), dimension("height"));
    if width.is_some() && height.is_some() {
        return;
    }
    let original = element
        .attr("data-orig-size")
        .and_then(parse_size)
        .filter(|(width, height)| *width > 0 && *height > 0);
    let shown = match element.attr("src").map(file_size) {
        Some(FileSize::Known(size)) => Some(size),
        Some(FileSize::Uploaded) => original,
        Some(FileSize::Unknown) | None => None,
    };
    // An image with one of the two only needs the other, which its shape gives.
    let Some((shape_width, shape_height)) = shown.or(original) else {
        return;
    };
    let scale = |size: u32, to: u32, from: u32| {
        (f64::from(size) * f64::from(to) / f64::from(from)).round() as u32
    };
    let size = match (width, height) {
        (Some(width), _) => Some((width, scale(width, shape_height, shape_width))),
        (_, Some(height)) => Some((scale(height, shape_width, shape_height), height)),
        (None, None) => shown,
    };
    if let Some((width, height)) = size.filter(|(width, height)| *width > 0 && *height > 0) {
        set_attribute(element, "width", &width.to_string());
        set_attribute(element, "height", &height.to_string());
    }
}

/// What an image URL says about the size of the file it loads.
enum FileSize {
    /// A smaller copy WordPress made, which has its size in its file name, like
    /// `photo-1024x768.jpg`, or Photon's `?resize=1024,768`, which crops to it.
    Known((u32, u32)),
    /// Photon's or WordPress.com's other sizes, like `?w=1024` or `?fit=1024,768`, which don't
    /// say what the file comes out at.
    Unknown,
    /// The file as it was uploaded.
    Uploaded,
}

fn file_size(src: &str) -> FileSize {
    let url = Url::parse(src)
        .or_else(|_| Url::parse("https://example.invalid/").and_then(|base| base.join(src)));
    let Ok(url) = url else {
        return FileSize::Unknown;
    };
    let mut sized = false;
    for (key, value) in url.query_pairs() {
        match &*key {
            "resize" => return parse_size(&value).map_or(FileSize::Unknown, FileSize::Known),
            "w" | "h" | "fit" | "zoom" => sized = true,
            _ => {}
        }
    }
    if sized {
        return FileSize::Unknown;
    }
    let name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back());
    let suffix = name
        .and_then(|name| name.rsplit_once('.'))
        .and_then(|(stem, _)| stem.rsplit_once('-'))
        .and_then(|(_, suffix)| suffix.split_once('x'));
    match suffix.and_then(|(width, height)| Some((width.parse().ok()?, height.parse().ok()?))) {
        Some(size) => FileSize::Known(size),
        None => FileSize::Uploaded,
    }
}

/// Where lazy-loading plugins put an image's URL, for their script to load it. The page doesn't run
/// their scripts.
const LAZY_SRC_ATTRIBUTES: &[&str] = &["data-src", "data-lazy-src", "data-original"];
const LAZY_SRCSET_ATTRIBUTES: &[&str] = &["data-srcset", "data-lazy-srcset"];

/// Gives a lazily-loaded image the URL a plugin's script would have, when its `src` is only a
/// placeholder. An image whose `src` is a real `data:` image has no such URL, so it keeps its `src`.
fn load_lazy_image(element: &mut Element) {
    let lazy = |element: &Element, names: &[&str]| {
        names
            .iter()
            .filter_map(|name| element.attr(name))
            .map(str::trim)
            .find(|value| !value.is_empty() && !value.starts_with("data:"))
            .map(str::to_owned)
    };
    if element.name() == "img" {
        let is_placeholder = element.attr("src").is_none_or(|src| {
            let src = src.trim();
            src.is_empty() || src.starts_with("data:")
        });
        if let Some(src) = lazy(element, LAZY_SRC_ATTRIBUTES).filter(|_| is_placeholder) {
            set_attribute(element, "src", &src);
        }
    }
    if matches!(element.name(), "img" | "source")
        && element
            .attr("srcset")
            .is_none_or(|srcset| srcset.trim().is_empty())
        && let Some(srcset) = lazy(element, LAZY_SRCSET_ATTRIBUTES)
    {
        set_attribute(element, "srcset", &srcset);
    }
}

/// A link to somewhere in the post itself, written with the post's full address, as just its
/// `#fragment`, or `None` for any other link. The Reader API writes in-page links this way for posts
/// it reads from a site's feed, like footnotes. The page's address isn't the post's, so they would
/// leave the page.
fn in_page_link(href: &str, post_url: &Url) -> Option<String> {
    let url = Url::parse(href.trim()).ok()?;
    let fragment = url.fragment().filter(|fragment| !fragment.is_empty())?;
    // The scheme and a trailing slash don't make it a different post.
    let is_post = url.host_str() == post_url.host_str()
        && url.port() == post_url.port()
        && url.path().trim_end_matches('/') == post_url.path().trim_end_matches('/')
        && url.query() == post_url.query();
    is_post.then(|| format!("#{fragment}"))
}

/// Whether the URL in an `attribute` of a `tag` loads or goes somewhere, rather than running
/// something. Links can go to web pages, email addresses and phone numbers. Images and media can
/// also be `data:` URLs, which can't run anything there. Everything else has to be a web page. A
/// relative URL is part of the post's site, or the page itself.
fn is_safe_url(tag: &str, attribute: &str, url: &str) -> bool {
    let schemes: &[&str] = match (tag, attribute) {
        ("a" | "area", "href") => &["http", "https", "mailto", "tel", "sms"],
        ("img" | "source", "src" | "srcset")
        | ("audio" | "video" | "track", "src")
        | ("video", "poster")
        | ("image", "href") => &["http", "https", "data"],
        _ => &["http", "https"],
    };
    let is_safe = |url: &str| url_scheme(url).is_none_or(|scheme| schemes.contains(&&*scheme));
    if attribute == "srcset" {
        srcset_candidates(url)
            .into_iter()
            .all(|(url, _)| is_safe(url))
    } else {
        is_safe(url)
    }
}

/// Attributes that a browser loads, or goes to when they're tapped. `xlink:href` is `href` here.
fn is_url_attribute(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "href"
            | "src"
            | "srcset"
            | "action"
            | "formaction"
            | "data"
            | "poster"
            | "background"
            | "codebase"
            | "lowsrc"
            | "dynsrc"
    )
}

fn is_event_handler(name: &str) -> bool {
    name.get(..2)
        .is_some_and(|start| start.eq_ignore_ascii_case("on"))
}

/// A URL's scheme, lowercased, the way a browser reads it, or `None` if it's relative. Browsers
/// ignore spaces and control characters around a URL, and tabs and line breaks in it, so
/// `java\tscript:` is `javascript:`. A URL that doesn't parse can still have a scheme to a browser.
fn url_scheme(url: &str) -> Option<String> {
    let url: String = url
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let (scheme, _) = url.split_once(':')?;
    let mut chars = scheme.chars();
    let is_scheme = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    is_scheme.then(|| scheme.to_ascii_lowercase())
}

/// Whether a `<meta>` is only data about the post, like an image's size, rather than something
/// that acts on the page, like a redirect.
fn is_microdata(meta: &Element) -> bool {
    meta.attr("itemprop").is_some()
        && meta.attr("http-equiv").is_none()
        && meta.attr("charset").is_none()
}

/// Whether an attribute holds a URL that should load over HTTPS even when the post has an `http:`
/// URL for it: an image's or an iframe's. Of 500 `http:` image URLs sampled from Reader posts, only
/// one loaded over HTTP but not HTTPS. The other failures failed both ways. An `http:` iframe
/// wouldn't load at all in an HTTPS page.
fn upgrades_to_https(tag: &str, attribute: &str) -> bool {
    matches!(
        (tag, attribute),
        ("img", "src" | "srcset" | "data-orig-file")
            | ("source", "srcset")
            | ("video", "poster")
            | ("iframe", "src")
    )
}

/// Whether an iframe loads a web page, rather than nothing, or a `javascript:` or `data:` URL. A
/// relative URL counts: the cleanup resolves it against the post.
fn loads_a_web_page(iframe: &Element) -> bool {
    iframe
        .attr("src")
        .map(str::trim)
        .filter(|src| !src.is_empty())
        .is_some_and(|src| is_safe_url("iframe", "src", src))
}

/// A URL relative to the post, made absolute, or `None` for any other URL. The page's base URL
/// isn't the post's site, so a relative URL would go somewhere else.
fn resolve(url: &str, post_url: &Url) -> Option<String> {
    let url = url.trim();
    let is_relative = !url.is_empty() && !url.starts_with('#') && Url::parse(url).is_err();
    is_relative.then(|| post_url.join(url).ok().map(String::from))?
}

/// A cover block's background video, which the Reader API turns into a `<div>` with the video's
/// attributes. It doesn't play until it's a `<video>` again.
fn is_cover_video(element: &Element) -> bool {
    element.attr("src").is_some()
        && element
            .classes()
            .any(|class| class == "wp-block-cover__video-background")
}

/// The URL in an embed block that never became an embed, which holds nothing but the URL. It's
/// better as a link.
fn unresolved_embed_url(wrapper: ElementRef) -> Option<String> {
    let is_wrapper = wrapper
        .value()
        .classes()
        .any(|class| class == "wp-block-embed__wrapper");
    let mut children = wrapper.children().filter(|child| match child.value() {
        Node::Text(text) => !text.trim().is_empty(),
        _ => true,
    });
    let (Some(only), None) = (children.next(), children.next()) else {
        return None;
    };
    let url = only.value().as_text()?.trim();
    let is_web_url = Url::parse(url).is_ok_and(|url| matches!(url.scheme(), "http" | "https"));
    (is_wrapper && is_web_url).then(|| url.to_owned())
}

/// How wide a column of a Jetpack tiled gallery is, relative to the others in its row, so the
/// stylesheet can lay the gallery out without Jetpack's styles. Jetpack gives each column its width
/// inline, which the cleanup removes with the post's other styles: the block as a `flex-basis`, the
/// older shortcode gallery as a `width`, which it also leaves in `data-original-width`.
fn tile_weight(element: &Element) -> Option<f64> {
    let weight = if element.classes().any(|class| class == "tiled-gallery__col") {
        element.attr("style")?.split(';').find_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            let value = value.trim();
            match property.trim() {
                "flex-basis" => value.strip_suffix('%')?.trim().parse::<f64>().ok(),
                // The cleanup's own, in a post it has already cleaned.
                "flex-grow" => value.parse::<f64>().ok(),
                _ => None,
            }
        })?
    } else if element.classes().any(|class| class == "gallery-group") {
        element
            .attr("data-original-width")?
            .trim()
            .parse::<f64>()
            .ok()?
    } else {
        return None;
    };
    (weight.is_finite() && weight > 0.0).then_some(weight)
}

/// The emoji an emoji image shows, from its text alternative, so it can show as text instead: in the
/// platform's own style, and offline. Classic smileys like `:)` stay images.
fn emoji_character(image: &Element) -> Option<&str> {
    let alt = image.attr("alt")?.trim();
    let is_emoji_image = image
        .classes()
        .any(|class| class == "emoji" || class == "wp-smiley");
    let is_emoji = alt.chars().any(|c| u32::from(c) >= 0x2000)
        && !alt.chars().any(|c| c.is_ascii_alphabetic());
    (is_emoji_image && is_emoji).then_some(alt)
}

/// Sets an attribute, adding it if the element doesn't have it.
pub(super) fn set_attribute(element: &mut Element, name: &str, value: &str) {
    let name = QualName::new(None, ns!(), LocalName::from(name));
    // `scraper` keeps attributes sorted, and finds them with a binary search.
    match element
        .attrs
        .binary_search_by(|(existing, _)| existing.cmp(&name))
    {
        Ok(index) => element.attrs[index].1 = value.into(),
        Err(index) => element.attrs.insert(index, (name, value.into())),
    }
}

/// How the page sizes an iframe so it fits the screen and keeps its shape.
#[derive(Debug, Clone, Copy, PartialEq)]
enum IframeSize {
    /// As wide as the screen, up to `width`, with the shape `width` by `height`.
    Fixed { width: u32, height: u32 },
    /// As wide as the screen, with a height that's `percent` of its width.
    Ratio { percent: f64 },
    /// As wide as the screen, at a fixed height.
    Height(u32),
}

impl IframeSize {
    fn style(self) -> String {
        match self {
            Self::Fixed { width, height } => format!(
                "width: 100%; max-width: {width}px; height: auto; aspect-ratio: {width} / {height};"
            ),
            Self::Ratio { percent } => {
                format!("width: 100%; height: auto; aspect-ratio: 100 / {percent};")
            }
            Self::Height(height) => format!("width: 100%; height: {height}px;"),
        }
    }
}

/// The size an iframe asks for, if it needs one to keep its shape at the screen's width. The page
/// can't size an iframe from its attributes the way it does an image: it would keep the iframe's
/// height while narrowing it, and squash it.
///
/// An iframe gives its size in its attributes, or in inline styles, which the cleanup removes: its
/// own, or a wrapper's padding that's a share of its width, the way the Jetpack GIF block does it.
/// The cleanup's own style, in a post it has already cleaned, gives the same size again.
///
/// YouTube and Vimeo embeds are already in a wrapper that keeps their shape.
fn iframe_size(iframe: ElementRef) -> Option<IframeSize> {
    let wrapped = iframe
        .ancestors()
        .filter_map(ElementRef::wrap)
        .any(|ancestor| {
            ancestor
                .value()
                .classes()
                .any(|class| class == "embed-youtube" || class == "embed-vimeo")
        });
    if wrapped {
        return None;
    }

    let element = iframe.value();
    let style = |property: &str| style_value(element, property);
    let pixels = |property: &str| {
        style(property)?
            .strip_suffix("px")?
            .trim()
            .parse::<u32>()
            .ok()
    };
    let width = element
        .attr("width")
        .and_then(parse_dimension)
        .or_else(|| pixels("width"))
        .filter(|width| *width > 0);
    let height = element
        .attr("height")
        .and_then(parse_dimension)
        .or_else(|| pixels("height"))
        .filter(|height| *height > 0);

    if let (Some(width), Some(height)) = (width, height) {
        return Some(IframeSize::Fixed { width, height });
    }
    if let Some((ratio_width, ratio_height)) = style("aspect-ratio").and_then(parse_ratio) {
        return Some(match pixels("max-width") {
            Some(width) if width > 0 => IframeSize::Fixed {
                width,
                height: (f64::from(width) * ratio_height / ratio_width).round() as u32,
            },
            _ => IframeSize::Ratio {
                percent: rounded_percent(ratio_height / ratio_width * 100.0)?,
            },
        });
    }
    if let Some(percent) = wrapper_padding_percent(iframe) {
        return Some(IframeSize::Ratio { percent });
    }
    // A width attribute that isn't a number, like `100%`, already fits the screen.
    match (element.attr("width"), height) {
        (None, Some(height)) => Some(IframeSize::Height(height)),
        _ => None,
    }
}

/// The last value an element's inline style gives `property`, the one that applies.
fn style_value<'a>(element: &'a Element, property: &str) -> Option<&'a str> {
    element
        .attr("style")?
        .split(';')
        .filter_map(|declaration| declaration.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case(property))
        .map(|(_, value)| value.trim())
        .next_back()
}

/// An `aspect-ratio` like `16 / 9`.
fn parse_ratio(value: &str) -> Option<(f64, f64)> {
    let (width, height) = value.split_once('/')?;
    let width = width.trim().parse::<f64>().ok()?;
    let height = height.trim().parse::<f64>().ok()?;
    (width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0)
        .then_some((width, height))
}

/// A share of a width, to two decimal places, if it could be an embed's height.
fn rounded_percent(percent: f64) -> Option<f64> {
    let percent = (percent * 100.0).round() / 100.0;
    (percent.is_finite() && percent > 0.0 && percent <= 1000.0).then_some(percent)
}

/// The height of the box an iframe fills, as a share of its width, when a wrapper sets it with a
/// percentage padding: the iframe's parent, or an empty element next to it.
fn wrapper_padding_percent(iframe: ElementRef) -> Option<f64> {
    let parent = iframe.parent().and_then(ElementRef::wrap)?;
    let spacers = parent
        .children()
        .filter_map(ElementRef::wrap)
        .filter(|sibling| sibling.id() != iframe.id() && sibling.children().next().is_none());
    std::iter::once(parent)
        .chain(spacers)
        .find_map(|element| padding_percent(element.value()))
}

/// The top or bottom padding an element's inline style gives as a percentage of its width.
fn padding_percent(element: &Element) -> Option<f64> {
    let percent = |value: &str| value.trim().strip_suffix('%')?.trim().parse::<f64>().ok();
    let from_shorthand = || {
        let values: Vec<&str> = style_value(element, "padding")?
            .split_whitespace()
            .collect();
        // `padding: top`, `top sides`, `top sides bottom` or `top right bottom left`.
        let top = values.first().and_then(|value| percent(value));
        let bottom = values.get(2).and_then(|value| percent(value));
        top.filter(|top| *top > 0.0).or(bottom)
    };
    let padding = style_value(element, "padding-top")
        .and_then(percent)
        .filter(|top| *top > 0.0)
        .or_else(|| style_value(element, "padding-bottom").and_then(percent))
        .or_else(from_shorthand)?;
    rounded_percent(padding)
}

/// Whether a paragraph shows nothing, like the `<p>&nbsp;</p>` or `<p><strong> </strong></p>`
/// authors add for space, or the `<p><!-- wp:paragraph --></p>` block markup can leave behind. The
/// page puts the same space between blocks, so an empty paragraph would only leave a gap.
fn is_empty_paragraph(paragraph: ElementRef) -> bool {
    shows_nothing(paragraph)
}

/// Elements that show something without any text.
const MEDIA: &[&str] = &[
    "img", "picture", "iframe", "video", "audio", "svg", "canvas", "embed", "object", "math",
];

/// Whether an element shows nothing, and nothing links to it: it holds only spaces, `&nbsp;`, line
/// breaks, comments and empty elements, like an icon font's `<i>`. One with an anchor anywhere in
/// it stays, since a link can point at it. So does one with a known embed in it, like Pinterest's
/// empty link, which its script fills in.
fn shows_nothing(element: ElementRef) -> bool {
    element.descendants().all(|node| match node.value() {
        Node::Text(text) => text.trim().is_empty(),
        Node::Element(element) => {
            !is_anchor(element)
                && !MEDIA.contains(&element.name())
                && !ElementRef::wrap(node).is_some_and(embeds::is_known_embed)
        }
        _ => true,
    })
}

/// Whether a link can point at an element: it has an `id`, or it's an `<a name>`, which is how
/// anchors were made before `id`. `anchors.rs` finds anchors the same way.
fn is_anchor(element: &Element) -> bool {
    element.id().is_some() || (element.name() == "a" && element.attr("name").is_some())
}

/// Elements that sit in a line of text, rather than starting a block, as HTML defines them: images,
/// iframes and video included.
const PHRASING: &[&str] = &[
    "a", "abbr", "audio", "b", "bdi", "bdo", "big", "br", "canvas", "cite", "code", "data", "del",
    "dfn", "em", "embed", "font", "i", "iframe", "img", "ins", "kbd", "label", "mark", "math",
    "object", "param", "picture", "q", "s", "samp", "small", "source", "span", "strike", "strong",
    "sub", "sup", "svg", "time", "track", "tt", "u", "var", "video", "wbr",
];

/// Elements the page hides. They end a run of inline content, since a paragraph can't hold them,
/// but they aren't blocks to space it from.
const HIDDEN_BLOCKS: &[&str] = &[
    "form", "template", "noscript", "button", "input", "select", "textarea",
];

/// Whether an element sits in a line of text, with nothing that starts a block inside it. A link
/// around a heading starts a block. What's inside an SVG or MathML element is drawing, not text.
fn is_phrasing(element: ElementRef) -> bool {
    let name = element.value().name();
    matches!(name, "svg" | "math")
        || (PHRASING.contains(&name)
            && element
                .children()
                .filter_map(ElementRef::wrap)
                .all(is_phrasing))
}

/// Classes of the blocks that lay their own blocks out one after another, as the stylesheet spaces
/// them.
const STACK_CLASSES: &[&str] = &[
    "is-layout-flow",
    "is-layout-constrained",
    "wp-block-group",
    "wp-block-group__inner-container",
    "wp-block-column",
    "wp-block-cover__inner-container",
    "wp-block-media-text__content",
];

/// Whether an element lays its blocks out one after another, with the stylesheet's space between
/// them: a quote, a group or a plain `<div>`, among others.
fn is_stack(element: &Element) -> bool {
    matches!(element.name(), "blockquote" | "details")
        || (element.name() == "div" && element.attr("class").is_none())
        || element
            .classes()
            .any(|class| STACK_CLASSES.contains(&class))
}

/// Puts each run of inline content between blocks in a paragraph: a sentence or a linked image
/// between two blocks of a post, or a quote's citation after its paragraphs. WordPress does the same
/// for posts without blocks. The page spaces blocks, and inline content outside one wouldn't be
/// spaced. A run of only line breaks was there for space, so it goes.
///
/// This is for the post, and for each block in it that lays its blocks out one after another. One
/// that holds only inline content stays as it is: there's nothing to space it from.
fn wrap_loose_inline_content(document: &mut Html) {
    let root = document.root_element();
    let stacks: Vec<_> = std::iter::once(root.id())
        .chain(
            root.descendants()
                .filter_map(ElementRef::wrap)
                .filter(|element| is_stack(element.value()))
                .map(|element| element.id()),
        )
        .collect();
    for stack in stacks {
        let mut runs = vec![Vec::new()];
        let mut has_block = false;
        for child in document
            .tree
            .get(stack)
            .expect("the ID came from this tree")
            .children()
        {
            match child.value() {
                Node::Element(element) if HIDDEN_BLOCKS.contains(&element.name()) => {
                    runs.push(Vec::new());
                }
                // An empty block, like a spacer, ends a run, but there's nothing to space it from.
                Node::Element(_) if !ElementRef::wrap(child).is_some_and(is_phrasing) => {
                    has_block |= !ElementRef::wrap(child).is_some_and(shows_nothing);
                    runs.push(Vec::new());
                }
                _ => runs
                    .last_mut()
                    .expect("there's always a run")
                    .push(child.id()),
            }
        }
        if !has_block {
            continue;
        }

        for run in runs {
            let shows = |id| {
                let node = document.tree.get(id).expect("the ID came from this tree");
                match node.value() {
                    Node::Text(text) => !text.trim().is_empty(),
                    Node::Element(_) => !ElementRef::wrap(node).is_some_and(shows_nothing),
                    _ => false,
                }
            };
            // Spaces around the run stay outside the paragraph.
            let (Some(first), Some(last)) = (
                run.iter().position(|id| shows(*id)),
                run.iter().rposition(|id| shows(*id)),
            ) else {
                for id in run {
                    let mut node = document
                        .tree
                        .get_mut(id)
                        .expect("the ID came from this tree");
                    if matches!(node.value(), Node::Element(_)) {
                        node.detach();
                    }
                }
                continue;
            };
            let paragraph = document
                .tree
                .get_mut(run[first])
                .expect("the ID came from this tree")
                .insert_before(Node::Element(Element::new(
                    QualName::new(None, ns!(html), LocalName::from("p")),
                    Vec::new(),
                )))
                .id();
            for id in &run[first..=last] {
                document
                    .tree
                    .get_mut(paragraph)
                    .expect("the paragraph was just added")
                    .append_id(*id);
            }
        }
    }
}

/// The URL the page should load a post's `<script src>` from, if it should load it at all.
///
/// Relative URLs are dropped: the page's base URL isn't the post's site, so they'd load something
/// else.
fn post_script_url(src: &str) -> Option<String> {
    let src = src.trim();
    let src = https_if_protocol_relative(src).unwrap_or_else(|| src.to_owned());
    let url = Url::parse(&src).ok()?;
    (matches!(url.scheme(), "http" | "https") && embeds::is_allowed(&url)).then_some(src)
}

/// A protocol-relative URL (`//example.com`) with `https:` in front of it, or `None` for any other
/// URL.
fn https_if_protocol_relative(url: &str) -> Option<String> {
    url.strip_prefix("//").map(|rest| format!("https://{rest}"))
}

/// An `http:` URL with `https:` instead, or `None` for any other URL.
fn https_if_http(url: &str) -> Option<String> {
    let scheme = url.get(..7)?;
    scheme
        .eq_ignore_ascii_case("http://")
        .then(|| format!("https://{}", &url[7..]))
}

fn push_unique(scripts: &mut Vec<String>, script: String) {
    if !scripts.contains(&script) {
        scripts.push(script);
    }
}

#[cfg(test)]
mod tests {
    use super::IFRAME_SANDBOX;
    use crate::reader::ReaderPostContent;
    use crate::reader::safety_tests::safety_problems;
    use rstest::rstest;

    #[rstest]
    #[case::inline_style(r#"<p style="color: red">Hi</p>"#, "<p>Hi</p>")]
    #[case::style_element("<style>p { color: red }</style><p>Hi</p>", "<p>Hi</p>")]
    #[case::script("<p>Hi</p><script>alert(1)</script>", "<p>Hi</p>")]
    #[case::comment("<p>Hi<!-- note --></p>", "<p>Hi</p>")]
    #[case::paragraph_of_comments("<p> <!-- wp:paragraph --> </p><p>Hi</p>", "<p>Hi</p>")]
    #[case::empty_paragraph("<p></p><p>Hi</p>", "<p>Hi</p>")]
    // Paragraphs authors add for space. The page spaces blocks itself.
    #[case::spacer_paragraph(
        "<p>Hi</p><p>&nbsp;</p><p> <br> </p><p>Bye</p>",
        "<p>Hi</p><p>Bye</p>"
    )]
    #[case::paragraph_of_empty_elements(
        r#"<p>Hi</p><p><strong>&nbsp;</strong></p><p><i class="icon"></i></p>"#,
        "<p>Hi</p>"
    )]
    #[case::empty_paragraph_with_an_id(
        r#"<p id="here"></p><p>Hi</p>"#,
        r#"<p id="here"></p><p>Hi</p>"#
    )]
    #[case::paragraph_of_a_named_anchor(
        r#"<p>One</p><p><a name="top"></a></p><p>Two</p>"#,
        r#"<p>One</p><p><a name="top"></a></p><p>Two</p>"#
    )]
    #[case::loose_named_anchor(
        r#"<p>One</p><a name="top"></a><p>Two</p>"#,
        r#"<p>One</p><p><a name="top"></a></p><p>Two</p>"#
    )]
    // Pinterest's embed is an empty link until its script runs.
    #[case::paragraph_of_an_embed(
        r#"<p><a data-pin-do="embedBoard" href="https://www.pinterest.com/pinterest/official-news/"></a></p><p>Hi</p>"#,
        r#"<p><a data-pin-do="embedBoard" href="https://www.pinterest.com/pinterest/official-news/"></a></p><p>Hi</p>"#
    )]
    // Inline content between blocks gets a paragraph of its own, without the spaces around it.
    #[case::loose_text(
        "<p>One</p>\nTwo <a href=\"https://example.com/\">three</a>\n<p>Four</p>",
        "<p>One</p><p>\nTwo <a href=\"https://example.com/\">three</a></p>\n<p>Four</p>"
    )]
    #[case::loose_line_breaks("<p>One</p><br><br><p>Two</p>", "<p>One</p><p>Two</p>")]
    // A quote's citation, after its paragraph.
    #[case::loose_citation(
        "<blockquote><p>Words</p><cite>Someone</cite></blockquote>",
        "<blockquote><p>Words</p><p><cite>Someone</cite></p></blockquote>"
    )]
    #[case::link_around_a_heading(
        r#"<p>One</p><a href="https://example.com/"><h2>Two</h2></a>"#,
        r#"<p>One</p><a href="https://example.com/"><h2>Two</h2></a>"#
    )]
    #[case::only_inline_content(
        "Hi <strong>there</strong><br>you",
        "Hi <strong>there</strong><br>you"
    )]
    #[case::protocol_relative_src(
        r#"<img src="//example.com/a.jpg">"#,
        r#"<img src="https://example.com/a.jpg">"#
    )]
    #[case::protocol_relative_href(
        r#"<a href="//example.com/">Hi</a>"#,
        r#"<a href="https://example.com/">Hi</a>"#
    )]
    #[case::relative_href(r#"<a href="/about/">Hi</a>"#, r#"<a href="/about/">Hi</a>"#)]
    #[case::http_image(
        r#"<img src="http://example.com/a.jpg" srcset="http://example.com/a.jpg 1x, HTTP://example.com/b.jpg 2x" data-orig-file="http://example.com/a.jpg">"#,
        r#"<img data-orig-file="https://example.com/a.jpg" src="https://example.com/a.jpg" srcset="https://example.com/a.jpg 1x, https://example.com/b.jpg 2x">"#
    )]
    #[case::http_picture_source(
        r#"<picture><source srcset="http://example.com/a.webp"><img src="https://example.com/a.jpg"></picture>"#,
        r#"<picture><source srcset="https://example.com/a.webp"><img src="https://example.com/a.jpg"></picture>"#
    )]
    #[case::http_poster(
        r#"<video poster="http://example.com/a.jpg" src="http://example.com/a.mp4"></video>"#,
        r#"<video controls="" poster="https://example.com/a.jpg" src="http://example.com/a.mp4"></video>"#
    )]
    #[case::http_link(
        r#"<a href="http://example.com/a.jpg">Hi</a>"#,
        r#"<a href="http://example.com/a.jpg">Hi</a>"#
    )]
    #[case::autoplaying_video(
        r#"<video autoplay muted src="a.mp4"></video>"#,
        r#"<video controls="" muted="" src="a.mp4"></video>"#
    )]
    #[case::audio_with_controls(
        r#"<audio controls="controls" src="a.mp3"></audio>"#,
        r#"<audio controls="controls" src="a.mp3"></audio>"#
    )]
    #[case::emoji_image(
        r#"<p>Hi <img class="emoji" alt="😀" src="https://s.w.org/images/core/emoji/1f600.svg"></p>"#,
        "<p>Hi 😀</p>"
    )]
    #[case::keycap_emoji(r#"<img class="wp-smiley" alt="1️⃣" src="1.svg">"#, "1️⃣")]
    #[case::classic_smiley(
        r#"<img class="wp-smiley" alt=":)" src="smile.png">"#,
        r#"<img alt=":)" class="wp-smiley" src="smile.png">"#
    )]
    #[case::emoji_alt_on_an_ordinary_image(
        r#"<img alt="😀" src="a.jpg">"#,
        r#"<img alt="😀" src="a.jpg">"#
    )]
    #[case::iframe(
        r#"<iframe src="https://example.com/embed" width="640" height="360"></iframe>"#,
        r#"<iframe height="360" sandbox="{sandbox}" src="https://example.com/embed" style="width: 100%; max-width: 640px; height: auto; aspect-ratio: 640 / 360;" width="640"></iframe>"#
    )]
    #[case::iframe_in_a_wrapper(
        r#"<span class="embed-youtube"><iframe src="https://youtube.com/embed/a" width="640" height="360"></iframe></span>"#,
        r#"<span class="embed-youtube"><iframe height="360" sandbox="{sandbox}" src="https://youtube.com/embed/a" width="640"></iframe></span>"#
    )]
    #[case::iframe_without_a_number(
        r#"<iframe src="https://example.com/embed" width="100%" height="152"></iframe>"#,
        r#"<iframe height="152" sandbox="{sandbox}" src="https://example.com/embed" width="100%"></iframe>"#
    )]
    #[case::iframe_sized_by_its_style(
        r#"<iframe src="https://example.com/embed" style="border: 0; width: 350px; height: 470px;"></iframe>"#,
        r#"<iframe sandbox="{sandbox}" src="https://example.com/embed" style="width: 100%; max-width: 350px; height: auto; aspect-ratio: 350 / 470;"></iframe>"#
    )]
    #[case::iframe_with_only_a_height(
        r#"<iframe src="https://example.com/embed" height="300" style="width: 100%;"></iframe>"#,
        r#"<iframe height="300" sandbox="{sandbox}" src="https://example.com/embed" style="width: 100%; height: 300px;"></iframe>"#
    )]
    #[case::iframe_in_a_padding_wrapper(
        r#"<div class="wp-block-jetpack-gif-wrapper" style="padding-top:56%"><iframe src="https://giphy.com/embed/a"></iframe></div>"#,
        r#"<div class="wp-block-jetpack-gif-wrapper"><iframe sandbox="{sandbox}" src="https://giphy.com/embed/a" style="width: 100%; height: auto; aspect-ratio: 100 / 56;"></iframe></div>"#
    )]
    #[case::iframe_next_to_a_padding_spacer(
        r#"<div style="position: relative;"><div style="padding-bottom: 42.857142857142854%;"></div><iframe src="https://pca.st/embed/a" style="position: absolute; width: 100%; height: 100%;"></iframe></div>"#,
        r#"<div><div></div><iframe sandbox="{sandbox}" src="https://pca.st/embed/a" style="width: 100%; height: auto; aspect-ratio: 100 / 42.86;"></iframe></div>"#
    )]
    #[case::iframe_in_a_padding_shorthand_wrapper(
        r#"<div style="padding:56.25% 0 0 0;position:relative;"><iframe src="https://example.com/embed" style="position:absolute;top:0;left:0;width:100%;height:100%;"></iframe></div>"#,
        r#"<div><iframe sandbox="{sandbox}" src="https://example.com/embed" style="width: 100%; height: auto; aspect-ratio: 100 / 56.25;"></iframe></div>"#
    )]
    #[case::iframe_with_no_size_anywhere(
        r#"<div style="padding: 1em"><iframe src="https://example.com/embed"></iframe></div>"#,
        r#"<div><iframe sandbox="{sandbox}" src="https://example.com/embed"></iframe></div>"#
    )]
    #[case::cover_video_the_reader_api_left_a_video(
        r#"<video class="wp-block-cover__video-background" autoplay muted loop playsinline src="a.mp4"></video>"#,
        r#"<video class="wp-block-cover__video-background" controls="" loop="" muted="" playsinline="" src="a.mp4"></video>"#
    )]
    #[case::embed_the_reader_api_made_a_link(
        r#"<div class="wp-block-embed__wrapper"><a href="https://example.com/a">https://example.com/a</a></div>"#,
        r#"<div class="wp-block-embed__wrapper"><a href="https://example.com/a">https://example.com/a</a></div>"#
    )]
    #[case::event_handlers(r#"<p onclick="alert(1)" onMouseOver="alert(2)">Hi</p>"#, "<p>Hi</p>")]
    #[case::javascript_link(
        r#"<a href="javascript:alert(1)">Hi</a><a href=" JavaScript:alert(2)">Hi</a>"#,
        "<a>Hi</a><a>Hi</a>"
    )]
    #[case::svg_javascript_link(
        r#"<svg><a xlink:href="javascript:alert(1)"><text>Hi</text></a></svg>"#,
        "<svg><a><text>Hi</text></a></svg>"
    )]
    #[case::links_that_go_somewhere(
        r##"<a href="mailto:a@example.com">A</a><a href="tel:+15550100">B</a><a href="#notes">C</a><a href="/about/">D</a>"##,
        r##"<a href="mailto:a@example.com">A</a><a href="tel:+15550100">B</a><a href="#notes">C</a><a href="/about/">D</a>"##
    )]
    #[case::elements_that_act_on_the_page(
        r#"<base href="https://example.org/"><link rel="stylesheet" href="https://example.org/a.css"><meta http-equiv="refresh" content="0;url=https://example.org/"><meta itemprop="width" content="300"><p>Hi</p>"#,
        r#"<meta content="300" itemprop="width"><p>Hi</p>"#
    )]
    #[case::http_iframe(
        r#"<iframe src="http://example.com/embed"></iframe>"#,
        r#"<iframe sandbox="{sandbox}" src="https://example.com/embed"></iframe>"#
    )]
    #[case::iframe_srcdoc(
        r#"<iframe src="https://example.com/embed" srcdoc="<script>alert(1)</script>"></iframe>"#,
        r#"<iframe sandbox="{sandbox}" src="https://example.com/embed"></iframe>"#
    )]
    #[case::iframe_with_its_own_sandbox(
        r#"<iframe sandbox="allow-scripts" src="https://example.com/embed"></iframe>"#,
        r#"<iframe sandbox="allow-scripts" src="https://example.com/embed"></iframe>"#
    )]
    // Flickr's player reloads itself over and over in Chromium. Its embed doesn't.
    #[case::flickr_player(
        r#"<iframe sandbox="allow-scripts" src="https://www.flickr.com/photos/nasa/53638095797/player/"></iframe>"#,
        r#"<iframe sandbox="allow-scripts" src="https://embedr.flickr.com/photos/53638095797"></iframe>"#
    )]
    #[case::flickr_player_with_the_photo_secret(
        r#"<iframe sandbox="allow-scripts" src="//www.flickr.com/photos/12345678@N00/53638095797/player/9468ec1c25"></iframe>"#,
        r#"<iframe sandbox="allow-scripts" src="https://embedr.flickr.com/photos/53638095797"></iframe>"#
    )]
    #[case::flickr_album_player(
        r#"<iframe sandbox="allow-scripts" src="https://www.flickr.com/photos/nasa/albums/72157/player/"></iframe>"#,
        r#"<iframe sandbox="allow-scripts" src="https://www.flickr.com/photos/nasa/albums/72157/player/"></iframe>"#
    )]
    #[case::iframe_with_a_sandbox_that_allows_top_navigation(
        r#"<iframe sandbox="allow-scripts allow-top-navigation allow-same-origin ALLOW-TOP-NAVIGATION-BY-USER-ACTIVATION allow-top-navigation-to-custom-protocols" src="https://example.com/embed"></iframe>"#,
        r#"<iframe sandbox="allow-scripts allow-same-origin" src="https://example.com/embed"></iframe>"#
    )]
    #[case::iframe_with_an_empty_sandbox(
        r#"<iframe sandbox src="https://example.com/embed"></iframe>"#,
        r#"<iframe sandbox="" src="https://example.com/embed"></iframe>"#
    )]
    #[case::iframe_without_a_web_page(
        r#"<p>Hi</p><iframe src="javascript:alert(1)"></iframe><iframe srcdoc="<p>Hi</p>"></iframe><iframe src="data:text/html,Hi"></iframe>"#,
        "<p>Hi</p>"
    )]
    #[case::tiled_gallery_block_column(
        r#"<div class="tiled-gallery__row"><div class="tiled-gallery__col" style="flex-basis:62.5%"></div><div class="tiled-gallery__col" style="flex-basis: 37.5% ;"></div></div>"#,
        r#"<div class="tiled-gallery__row"><div class="tiled-gallery__col" style="flex-grow: 62.5;"></div><div class="tiled-gallery__col" style="flex-grow: 37.5;"></div></div>"#
    )]
    #[case::tiled_gallery_classic_group(
        r#"<div class="gallery-group images-1" style="width: 250px; height: 137px;" data-original-width="250" data-original-height="137"></div>"#,
        r#"<div class="gallery-group images-1" data-original-height="137" data-original-width="250" style="flex-grow: 250;"></div>"#
    )]
    #[case::tiled_gallery_column_cleaned_before(
        r#"<div class="tiled-gallery__col" style="flex-grow: 62.5;"></div>"#,
        r#"<div class="tiled-gallery__col" style="flex-grow: 62.5;"></div>"#
    )]
    #[case::tiled_gallery_column_without_a_width(
        r#"<div class="tiled-gallery__col" style="color: red"></div>"#,
        r#"<div class="tiled-gallery__col"></div>"#
    )]
    #[case::autoplay_on_other_elements(
        r#"<iframe autoplay src="a.html"></iframe>"#,
        r#"<iframe autoplay="" sandbox="{sandbox}" src="a.html"></iframe>"#
    )]
    #[case::javascript_url_with_a_tab(r#"<a href="java&#9;script:alert(1)">Hi</a>"#, "<a>Hi</a>")]
    #[case::javascript_urls_on_other_elements(
        r#"<img src="javascript:alert(1)"><form action="javascript:alert(1)"><button formaction="javascript:alert(1)">Go</button></form>"#,
        "<img><form><button>Go</button></form>"
    )]
    #[case::javascript_url_in_a_srcset(
        r#"<img src="a.jpg" srcset="a.jpg 1x, javascript:alert(1) 2x">"#,
        r#"<img src="a.jpg">"#
    )]
    #[case::mathml_link(
        r#"<math href="javascript:alert(1)"><mi>x</mi></math>"#,
        "<math><mi>x</mi></math>"
    )]
    #[case::data_urls_that_arent_images(
        r#"<object data="data:text/html,Hi"></object><embed src="data:text/html,Hi">"#,
        "<object></object><embed>"
    )]
    #[case::data_urls_for_media(
        r#"<audio src="data:audio/mpeg;base64,AA=="></audio>"#,
        r#"<audio controls="" src="data:audio/mpeg;base64,AA=="></audio>"#
    )]
    #[case::svg_animation_of_a_link(
        r#"<svg><a><set attributeName="href" to="javascript:alert(1)"/><text>Hi</text></a></svg>"#,
        "<svg><a><text>Hi</text></a></svg>"
    )]
    #[case::svg_animation_of_something_else(
        r#"<svg><circle r="5"><animate attributeName="r" values="5;10;5" dur="1s"/></circle></svg>"#,
        r#"<svg><circle r="5"><animate attributeName="r" dur="1s" values="5;10;5"></animate></circle></svg>"#
    )]
    #[case::microdata_that_redirects(
        r#"<meta itemprop="a" http-equiv="refresh" content="0;url=https://example.org/"><p>Hi</p>"#,
        "<p>Hi</p>"
    )]
    // The parser puts the second `<form>` inside the first, which HTML can't say: parsing it again
    // drops the inner `<form>`. The web view would see that, so the cleanup does too.
    #[case::form_inside_a_form(
        "<form><div></form><form><p>Hi</p></form>",
        "<form><div><p>Hi</p></div></form>"
    )]
    #[case::image_size_from_its_file_name(
        r#"<img src="https://example.com/a-1024x768.jpg">"#,
        r#"<img height="768" src="https://example.com/a-1024x768.jpg" width="1024">"#
    )]
    #[case::image_size_from_photon(
        r#"<img src="https://i0.wp.com/example.com/a.jpg?resize=600%2C400">"#,
        r#"<img height="400" src="https://i0.wp.com/example.com/a.jpg?resize=600%2C400" width="600">"#
    )]
    #[case::image_that_is_the_original(
        r#"<img src="https://example.com/a.jpg" data-orig-size="4032,3024">"#,
        r#"<img data-orig-size="4032,3024" height="3024" src="https://example.com/a.jpg" width="4032">"#
    )]
    // Photon's `?w=300` doesn't upscale, so the file could be narrower than 300.
    #[case::image_resized_to_an_unknown_size(
        r#"<img src="https://example.com/a.jpg?w=300" data-orig-size="4032,3024">"#,
        r#"<img data-orig-size="4032,3024" src="https://example.com/a.jpg?w=300">"#
    )]
    #[case::image_with_only_a_width(
        r#"<img src="https://example.com/a.jpg?w=300" width="300" data-orig-size="4032,3024">"#,
        r#"<img data-orig-size="4032,3024" height="225" src="https://example.com/a.jpg?w=300" width="300">"#
    )]
    #[case::image_with_a_size_already(
        r#"<img src="https://example.com/a-1024x768.jpg" width="512" height="300">"#,
        r#"<img height="300" src="https://example.com/a-1024x768.jpg" width="512">"#
    )]
    #[case::image_without_anything_that_says(
        r#"<img src="https://example.com/a.jpg">"#,
        r#"<img src="https://example.com/a.jpg">"#
    )]
    // `{sandbox}` stands for `IFRAME_SANDBOX`, which an iframe without a sandbox of its own gets.
    fn cleans_html(#[case] html: &str, #[case] expected: &str) {
        let content = ReaderPostContent::new(html, None);
        assert_eq!(
            content.html(),
            expected.replace("{sandbox}", IFRAME_SANDBOX)
        );
        assert_eq!(safety_problems(&content), Vec::<String>::new());
    }

    /// The cleanup removes the styles an iframe's size came from, so a second cleanup has to read
    /// the size back from the style the first one wrote.
    #[rstest]
    #[case::attributes(r#"<iframe src="https://example.com/a" width="640" height="360"></iframe>"#)]
    #[case::style(
        r#"<iframe src="https://example.com/a" style="width: 350px; height: 470px"></iframe>"#
    )]
    #[case::height(r#"<iframe src="https://example.com/a" height="300"></iframe>"#)]
    #[case::style_height(r#"<iframe src="https://example.com/a" style="height: 52px"></iframe>"#)]
    #[case::wrapper(
        r#"<div style="padding-bottom: 42.857142857142854%"><iframe src="https://example.com/a"></iframe></div>"#
    )]
    fn keeps_an_iframe_size_when_cleaned_again(#[case] html: &str) {
        let cleaned = ReaderPostContent::new(html, None).html();
        assert!(cleaned.contains("style="), "{cleaned}");
        assert_eq!(ReaderPostContent::new(&cleaned, None).html(), cleaned);
    }

    #[rstest]
    #[case::inline("<script>alert(1)</script>", &[])]
    #[case::post_script(
        r#"<script src="https://platform.twitter.com/widgets.js"></script>"#,
        &["https://platform.twitter.com/widgets.js"]
    )]
    #[case::protocol_relative(
        r#"<script src="//platform.instagram.com/en_US/embeds.js"></script>"#,
        &["https://platform.instagram.com/en_US/embeds.js"]
    )]
    #[case::relative(r#"<script src="/wp-includes/js/a.js"></script>"#, &[])]
    #[case::not_http(r#"<script src="data:text/javascript,alert(1)"></script>"#, &[])]
    #[case::blocked(
        r#"<script src="https://v0.wordpress.com/js/next/videopress-iframe.js?m=1"></script>"#,
        &[]
    )]
    #[case::duplicate(
        r#"<script src="https://platform.x.com/widgets.js"></script><script src="https://platform.x.com/widgets.js"></script>"#,
        &["https://platform.x.com/widgets.js"]
    )]
    #[case::ads(
        r#"<script async src="https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"></script>"#,
        &[]
    )]
    #[case::bluesky(
        r#"<script async src="https://embed.bsky.app/static/embed.js" charset="utf-8"></script>"#,
        &["https://embed.bsky.app/static/embed.js"]
    )]
    #[case::unknown_host(r#"<script src="https://example.com/a.js"></script>"#, &[])]
    fn lists_post_scripts(#[case] html: &str, #[case] expected: &[&str]) {
        assert_eq!(ReaderPostContent::new(html, None).scripts(), expected);
    }

    #[rstest]
    #[case::same_post("https://example.com/2026/10/post/#notes", "#notes")]
    #[case::other_scheme_and_no_slash("http://example.com/2026/10/post#notes", "#notes")]
    #[case::percent_encoded("https://example.com/2026/10/post/#caf%C3%A9", "#caf%C3%A9")]
    #[case::another_post(
        "https://example.com/2026/10/other/#notes",
        "https://example.com/2026/10/other/#notes"
    )]
    #[case::another_site(
        "https://example.org/2026/10/post/#notes",
        "https://example.org/2026/10/post/#notes"
    )]
    #[case::the_post_itself(
        "https://example.com/2026/10/post/",
        "https://example.com/2026/10/post/"
    )]
    #[case::already_in_page("#notes", "#notes")]
    fn makes_links_within_the_post_in_page_links(#[case] href: &str, #[case] expected: &str) {
        let html = format!(r#"<a href="{href}">Notes</a>"#);
        let content =
            ReaderPostContent::new(&html, Some("https://example.com/2026/10/post/".to_owned()));
        assert_eq!(content.html(), format!(r#"<a href="{expected}">Notes</a>"#));
    }

    #[rstest]
    #[case::data_placeholder(
        r#"<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" data-src="/a.jpg">"#,
        r#"<img data-src="/a.jpg" src="https://example.com/a.jpg">"#
    )]
    #[case::no_src(
        r#"<img data-lazy-src="http://example.com/a.jpg" data-lazy-srcset="http://example.com/a-300.jpg 300w">"#,
        r#"<img data-lazy-src="http://example.com/a.jpg" data-lazy-srcset="http://example.com/a-300.jpg 300w" src="https://example.com/a.jpg" srcset="https://example.com/a-300.jpg 300w">"#
    )]
    #[case::picture_source(
        r#"<picture><source data-srcset="https://example.com/a.webp"><img src="https://example.com/a.jpg"></picture>"#,
        r#"<picture><source data-srcset="https://example.com/a.webp" srcset="https://example.com/a.webp"><img src="https://example.com/a.jpg"></picture>"#
    )]
    #[case::real_src(
        r#"<img src="https://example.com/low.jpg" data-src="https://example.com/high.jpg">"#,
        r#"<img data-src="https://example.com/high.jpg" src="https://example.com/low.jpg">"#
    )]
    #[case::real_data_image(
        r#"<img src="data:image/png;base64,iVBORw0KGgo=" alt="Pasted">"#,
        r#"<img alt="Pasted" src="data:image/png;base64,iVBORw0KGgo=">"#
    )]
    fn loads_lazy_images(#[case] html: &str, #[case] expected: &str) {
        let content =
            ReaderPostContent::new(html, Some("https://example.com/2026/10/post/".to_owned()));
        assert_eq!(content.html(), expected);
    }

    #[test]
    fn keeps_only_the_text_of_a_post_that_keeps_changing() {
        let document = scraper::Html::parse_fragment("<p>Hi <b>there</b></p><p>a &lt; b</p>");
        let document = super::reparse(document, &mut Vec::new(), None, 0);
        assert_eq!(
            document.root_element().inner_html(),
            "<p>Hi there</p><p>a &lt; b</p>"
        );
    }

    #[test]
    fn finds_anchors_linked_with_the_post_address() {
        let content = ReaderPostContent::new(
            r#"<p>A note.<sup><a href="https://example.com/post/#fn-1">1</a></sup></p><div class="footnotes"><ol><li id="fn-1">The note. <a href="https://example.com/post/#fnref-1">↩</a></li></ol></div>"#,
            Some("https://example.com/post/".to_owned()),
        );
        let anchors = content.anchors();
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].id, "fn-1");
        assert_eq!(anchors[0].text.as_deref(), Some("The note."));
    }
}
