//! What a Reader post's card shows: a summary, a reading time, and which layout the post gets.
//!
//! The WordPress and Jetpack apps for iOS and Android each work these out with their own rules, so
//! the same post can look different in each. The functions here work them out one way. Where the
//! apps differ, the function's comment says what each does, which rule it follows, and why. The
//! web Reader settles a difference when it agrees with one of the apps.

use crate::wp_com::reader_posts::{ReaderPost, reader_post_display_image_url};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// The most characters a summary has, counting the `…` that ends one that was cut.
const SUMMARY_MAX_CHARS: usize = 500;

/// How much of a post's text is read to make a summary, or to tell whether a post has little
/// text, in bytes. It's many times what either needs, to leave room for what is taken out.
const TEXT_SAMPLE_MAX_BYTES: usize = 8_000;

/// How fast a reader reads.
const WORDS_PER_MINUTE: u64 = 200;

/// A gallery needs this many images that are big enough.
const GALLERY_MIN_IMAGES: usize = 5;

/// A gallery shows this many images at most.
const GALLERY_MAX_IMAGES: usize = 10;

/// An image in a gallery is at least this wide and this tall, in pixels.
const GALLERY_MIN_IMAGE_SIZE: (u32, u32) = (300, 72);

/// The image of a photo post is at least this wide, in pixels.
const PHOTO_MIN_IMAGE_WIDTH: u64 = 440;

/// A photo post has this many characters of text at most.
const PHOTO_MAX_TEXT_CHARS: usize = 85;

/// Which layout a post gets in a stream. [`reader_post_card_type`] picks it.
///
/// An app that has no layout for a type shows the post as [`Self::Default`]: every post has what
/// that layout needs.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    uniffi::Enum,
    strum_macros::EnumString,
    strum_macros::Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ReaderPostCardType {
    /// The title, the summary, and the post's image when it has one.
    Default,
    /// The post is one large image with little or no text. The image is the card.
    Photo,
    /// The post is a set of images. The card shows several of them, which
    /// [`reader_post_gallery_image_urls`] lists.
    Gallery,
    /// The post leads with a video, which is in `featured_media`, and has no image to show.
    Video,
    /// The post is a cross-post. The card is a short row that points to the original post, which
    /// is in `cross_post`.
    CrossPost,
}

/// The summary to show on a post's card: plain text on one line, or `None` when the post has no
/// text to summarize.
///
/// 1. It's the text of `excerpt`: tags are removed, character references such as `&amp;` are
///    decoded, and every run of whitespace, line breaks included, becomes one space. No-break
///    spaces inside the text are kept.
/// 2. A shortcode tag a site left in is removed when it's clearly one: `[/name]`,
///    `[name key="value"]`, `[name]` when its `[/name]` follows, and a name with an underscore,
///    such as `[vc_row]`. Other text in square brackets, such as `[1]` or `[sic]`, is kept.
/// 3. `Continue reading →`, which some sites end an excerpt with, is removed, along with anything
///    after it.
/// 4. The API ends an excerpt it cut short with `[…]`. That becomes `…`.
/// 5. When nothing is left, the summary is made the same way from `content`, leaving out
///    figures, their captions, and galleries. When a post has no text but those, as a photo with
///    a caption does, they are its summary.
/// 6. A summary is 500 characters at most. A longer one is cut at the last space before that, and
///    ends with `…`.
///
/// Where the apps differ:
///
/// - **A post with no excerpt.** iOS and the web make a summary from the content. Android shows
///   none. This follows iOS and the web.
/// - **How much of the content.** iOS takes the first paragraph, and nothing when the content has
///   no `<p>`, which is common in RSS posts. The web takes the text of the content without
///   figures and captions, and nothing when that's all the text there is. This follows the web,
///   which loses less, and then falls back to the captions, so a photo with a caption has one.
/// - **The length.** iOS cuts a summary it made from the content at 150 characters, and the web
///   keeps a 160-character version of one. Neither limits an excerpt, and neither does Android,
///   though an excerpt can run to tens of thousands of characters. This has one limit for both,
///   set past what a card has room for, so it changes nothing an app shows.
/// - **A trailing `[…]`.** iOS removes it, because it removes everything in square brackets.
///   Android keeps it. The web turns it into `…` where it shortens an excerpt, which is what this
///   does: it keeps the sign that there's more, without the brackets.
/// - **Shortcodes.** iOS removes everything in square brackets, which takes `[1]` and `[sic]`
///   with it. Android and the web remove nothing, so a card can start with
///   `[vc_row][vc_column]`. This removes only what is clearly a shortcode tag.
/// - **Line breaks.** Android keeps a line break for each `<p>` and `<br>`. iOS and the web
///   put the summary on one line, as this does.
/// - **`Continue reading →`.** Only iOS removes it, and it looks for the phrase in the app's
///   language. This looks for the English phrase, whatever the reader's language.
#[uniffi::export]
pub fn reader_post_summary(post: &ReaderPost) -> Option<String> {
    let summary_of = |html: Option<&str>, which| html.and_then(|html| summary_text(html, which));
    let content = post.content.as_deref();
    summary_of(post.excerpt.as_deref(), Text::All)
        .or_else(|| summary_of(content, Text::WithoutFigures))
        .or_else(|| summary_of(content, Text::All))
        .map(|summary| cut_to_length(summary, SUMMARY_MAX_CHARS))
}

/// How many minutes a post takes to read, or `None` when it has no words to read.
///
/// It's the post's word count at 200 words a minute, rounded up, so a post with any words takes
/// at least a minute. The word count is `word_count` when the API sent one above zero.
/// Otherwise it's the number of words in the text of `content`: a word is a run of characters
/// between spaces with a letter or a digit in it. Chinese and Japanese don't put spaces between
/// words, so each of their characters is a word, which is close to how the API counts them.
///
/// Where the apps differ:
///
/// - **The word count.** iOS and the web use `word_count`. Android counts the words in the
///   content. Tag and site streams don't send `word_count`, so most WordPress.com posts arrive
///   without one. This uses `word_count` when it's there, and counts when it isn't. For posts
///   that have one, the two counts are within a few percent of each other.
/// - **Rounding.** iOS rounds down, and shows nothing under 2 minutes. Android and the web round
///   up, to at least 1 minute. This follows Android and the web.
/// - **The reading speed.** iOS uses 250 words a minute, Android 200 and the web 189. Android is
///   the only one that shows the result to a reader, so this uses its 200.
/// - **A post shown as an excerpt.** Android shows no reading time for a post it only shows the
///   excerpt of (`use_excerpt`, on a Jetpack site). That's about the screen, not the post, so it's
///   left to the app: this returns the time for the whole post.
#[uniffi::export]
pub fn reader_post_reading_time_minutes(post: &ReaderPost) -> Option<u32> {
    let words = post.word_count.filter(|&words| words > 0).or_else(|| {
        let words = count_words(&html_text(post.content.as_deref()?, Text::All, usize::MAX));
        (words > 0).then_some(words)
    })?;
    Some(u32::try_from(words.div_ceil(WORDS_PER_MINUTE)).unwrap_or(u32::MAX))
}

/// Which layout a post gets in a stream. The first of these that fits:
///
/// 1. [`ReaderPostCardType::CrossPost`]: `cross_post` has the original post's site and post IDs.
/// 2. [`ReaderPostCardType::Gallery`]: the post has no image of its own (no `featured_image`, and
///    none picked by an editor), and its content has at least 5 images for a gallery. See
///    [`reader_post_gallery_image_urls`] for which images those are.
/// 3. [`ReaderPostCardType::Photo`]: the post has an image to show, its content has exactly one
///    image, that image is at least 440 pixels wide, and the content has 85 characters of text
///    at most, not counting captions. The width is `featured_media`'s when that's an image with
///    one, else the width the content gives the image.
/// 4. [`ReaderPostCardType::Video`]: the post has no image to show, and `featured_media` is a
///    video.
/// 5. [`ReaderPostCardType::Default`]: every other post.
///
/// "An image to show" is what [`reader_post_display_image_url`] picks.
///
/// Where the apps differ:
///
/// - **The layouts.** iOS has two: the cross-post row, and one card for everything else. Android
///   has the cross-post row, a gallery card, and a photo type it works out and no longer draws.
///   The web has all five. This has the web's five, so an app can tell them apart.
/// - **A cross-post.** Both apps go by the `xpost_origin` entry in `metadata`, which is where
///   `cross_post` comes from. Android needs both of its IDs, to open the original, and so does
///   this. The web goes by the `p2-xpost` tag, which nearly always comes with that entry.
/// - **A gallery.** Android wants no image to show at all, and 4 images at least 144 pixels
///   wide. That counts the image the API picks out of the content as an image to show, so a post
///   that is nothing but photos is almost never a gallery there. The web wants no `featured_image`,
///   and 5 images at least 300 by 72 pixels, none more than three times as long as it is wide.
///   This follows the web. It can't measure an image as the web does, so it goes by the size the
///   content gives it.
/// - **A photo.** Android wants an image to show and 100 characters of text at most. The web wants
///   exactly one image in the content, at least 440 pixels wide, and 85 characters at most. This
///   follows the web: its rule is the one that has a layout behind it, and it keeps a small image
///   from being shown as a large one.
/// - **A video.** Only the web has the layout, for a post with no featured image whose content
///   starts with a video rather than an image. `featured_media` is the API's version of that pick.
/// - **A post Discover picked.** None of the three has a layout for it. iOS adds a line of
///   attribution to the post once it's open.
#[uniffi::export]
pub fn reader_post_card_type(post: &ReaderPost) -> ReaderPostCardType {
    if is_cross_post(post) {
        return ReaderPostCardType::CrossPost;
    }

    let content = post.content.as_deref().unwrap_or_default();
    let images = content_images(content);

    let has_own_image = post.editorial_image_url.is_some()
        || post.featured_image.as_deref().is_some_and(is_not_empty);
    if !has_own_image && gallery_images(&images).count() >= GALLERY_MIN_IMAGES {
        return ReaderPostCardType::Gallery;
    }

    let featured_media = post.featured_media.as_ref();
    let media_is = |media_type: &str| {
        featured_media.is_some_and(|media| media.media_type.as_deref() == Some(media_type))
    };
    let has_image_to_show = post.display_image_url.as_deref().is_some_and(is_not_empty)
        || reader_post_display_image_url(
            post.editorial_image_url.as_deref(),
            post.featured_image.as_deref(),
            featured_media,
        )
        .is_some();

    if has_image_to_show && let [image] = images.as_slice() {
        let width = featured_media
            .filter(|_| media_is("image"))
            .and_then(|media| media.width)
            .or(image.size.map(|(width, _)| u64::from(width)));
        if width.is_some_and(|width| width >= PHOTO_MIN_IMAGE_WIDTH) && has_little_text(content) {
            return ReaderPostCardType::Photo;
        }
    }

    if !has_image_to_show && media_is("video") {
        return ReaderPostCardType::Video;
    }

    ReaderPostCardType::Default
}

/// The images to show for a post whose card is a [`ReaderPostCardType::Gallery`], out of its
/// `content`, in the order they appear. There are 10 at most.
///
/// An image counts when the content gives its size, with `width` and `height` or with
/// `data-orig-size`, and it is at least 300 pixels wide and 72 tall, and no more than three times
/// as long as it is wide. An image that appears twice counts once. Avatars, smileys, tracking
/// pixels and `data:` images don't count.
///
/// These are the web Reader's rules. The web loads each image to measure it. This goes by the
/// size the content gives, so an image with no size there doesn't count.
#[uniffi::export]
pub fn reader_post_gallery_image_urls(content: &str) -> Vec<String> {
    let images = content_images(content);
    gallery_images(&images)
        .take(GALLERY_MAX_IMAGES)
        .map(|image| image.src.clone())
        .collect()
}

/// The addresses of the images in a post's `content`, in the order they appear and once each,
/// for an app or the library to store so the post can be read with no connection.
///
/// Every `<img>` with an `http` or `https` address counts, whatever its size, and one that starts
/// with `//` is read as `https`. A tracking pixel doesn't count: fetching it would count a view.
/// Neither does a `data:` image, which is already in the content, or an address that isn't whole,
/// such as `/images/a.png`.
#[uniffi::export]
pub fn reader_post_content_image_urls(content: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    HtmlTokens::new(content)
        .filter_map(|token| match token {
            HtmlToken::Open(tag) if tag.is("img") => storable_image_url(&tag),
            _ => None,
        })
        .filter(|url| seen.insert(url.clone()))
        .collect()
}

/// A post's `content` for reading with no connection: each image in `stored`, which maps an
/// address from [`reader_post_content_image_urls`] to where its copy is, points at that copy.
///
/// Such an image loses its `srcset` and `sizes`, which would send a web view back to the network
/// for another size. So does every `<source>` once any image is stored, so that a `<picture>`
/// shows its `<img>`. An image that isn't in `stored` is left as it is. The rest of the content
/// is returned as it was written, byte for byte.
#[uniffi::export]
pub fn reader_post_content_with_stored_images(
    content: &str,
    stored: std::collections::HashMap<String, String>,
) -> String {
    if stored.is_empty() {
        return content.to_string();
    }
    let mut rewritten = String::with_capacity(content.len());
    // How much of `content` is in `rewritten` so far.
    let mut copied = 0;
    for token in HtmlTokens::new(content) {
        let HtmlToken::Open(tag) = token else {
            continue;
        };
        let copy = if tag.is("img") {
            storable_image_url(&tag).and_then(|url| stored.get(&url))
        } else {
            None
        };
        let drops_sizes = copy.is_some() || tag.is("source");
        if !drops_sizes {
            continue;
        }
        let mut attributes = String::with_capacity(tag.attributes.len());
        let mut changed = false;
        for (name, value) in HtmlAttributes::new(tag.attributes) {
            if is_any_of(name, &["srcset", "sizes"]) {
                changed = true;
                continue;
            }
            attributes.push(' ');
            attributes.push_str(name);
            match copy {
                Some(copy) if name.eq_ignore_ascii_case("src") => {
                    changed = true;
                    attributes.push_str("=\"");
                    attributes.push_str(&htmlize::escape_attribute(copy.as_str()));
                    attributes.push('"');
                }
                _ => push_attribute_value(&mut attributes, value),
            }
        }
        if !changed {
            continue;
        }
        if tag.is_self_closing {
            attributes.push_str(" /");
        }
        // The tag's attributes are a slice of `content`, so this is where they start in it.
        let start = tag.attributes.as_ptr() as usize - content.as_ptr() as usize;
        rewritten.push_str(&content[copied..start]);
        rewritten.push_str(&attributes);
        copied = start + tag.attributes.len();
    }
    rewritten.push_str(&content[copied..]);
    rewritten
}

/// Writes an attribute's value as it was written, in quotes that it doesn't contain.
fn push_attribute_value(attributes: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    attributes.push('=');
    if !value.contains('"') {
        attributes.push('"');
        attributes.push_str(value);
        attributes.push('"');
    } else if !value.contains('\'') {
        attributes.push('\'');
        attributes.push_str(value);
        attributes.push('\'');
    } else {
        attributes.push('"');
        attributes.push_str(&value.replace('"', "&quot;"));
        attributes.push('"');
    }
}

/// The address an `<img>` loads, if it's one to store for reading with no connection.
fn storable_image_url(tag: &HtmlTag) -> Option<String> {
    let src = tag.attribute("src")?;
    let src = src.trim();
    let url = match src.strip_prefix("//") {
        Some(rest) => format!("https://{rest}"),
        None => src.to_string(),
    };
    let lowercase = url.to_ascii_lowercase();
    if !(lowercase.starts_with("https://") || lowercase.starts_with("http://")) {
        return None;
    }
    // WordPress.com's and Jetpack's stats pixels, which count a view when they're fetched.
    if lowercase.contains("://pixel.wp.com/") || lowercase.contains("://stats.wp.com/") {
        return None;
    }
    let pixels = |name: &str| tag.attribute(name)?.trim().parse::<u32>().ok();
    if let (Some(width), Some(height)) = (pixels("width"), pixels("height"))
        // A tracking pixel is 1 pixel wide or tall, and nothing much the other way.
        && (1..=2).contains(&(u64::from(width) + u64::from(height)))
    {
        return None;
    }
    Some(url)
}

fn is_not_empty(text: &str) -> bool {
    !text.is_empty()
}

/// How many words `text` has. A run of characters between spaces is a word when it has a letter
/// or a digit in it, so a dash on its own isn't one. A Chinese or Japanese character is a word
/// by itself.
fn count_words(text: &str) -> u64 {
    let is_han_or_kana = |c: char| {
        matches!(c,
            '\u{3040}'..='\u{30ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{f900}'..='\u{faff}'
            | '\u{20000}'..='\u{2fa1f}')
    };
    let mut words = 0;
    let mut is_in_counted_word = false;
    for c in text.chars() {
        if c.is_whitespace() {
            is_in_counted_word = false;
        } else if is_han_or_kana(c) {
            words += 1;
            is_in_counted_word = false;
        } else if !is_in_counted_word && c.is_alphanumeric() {
            words += 1;
            is_in_counted_word = true;
        }
    }
    words
}

/// Whether `content` has little enough text, not counting captions, for a photo post.
fn has_little_text(content: &str) -> bool {
    let text = single_line(&html_text(
        content,
        Text::WithoutFigures,
        TEXT_SAMPLE_MAX_BYTES,
    ));
    text.chars().count() <= PHOTO_MAX_TEXT_CHARS
}

fn is_cross_post(post: &ReaderPost) -> bool {
    post.cross_post.as_ref().is_some_and(|cross_post| {
        cross_post.site_id.is_some_and(|site_id| site_id.0 != 0)
            && cross_post.post_id.is_some_and(|post_id| post_id != 0)
    })
}

/// The text of `html` as a summary, before it's cut to length. `None` when it has no text.
fn summary_text(html: &str, which: Text) -> Option<String> {
    let text = html_text(html, which, TEXT_SAMPLE_MAX_BYTES);
    let text = single_line(&without_shortcode_tags(&text));
    let text = without_continue_reading(&text);
    let text = with_ellipsis_for_more_marker(text);
    (!text.is_empty()).then(|| text.into_owned())
}

/// `text` up to where `Continue reading →` starts, in any letter case.
fn without_continue_reading(text: &str) -> &str {
    const CONTINUE_READING: &str = "continue reading →";
    let start = text.char_indices().map(|(index, _)| index).find(|&index| {
        text.get(index..index + CONTINUE_READING.len())
            .is_some_and(|found| found.eq_ignore_ascii_case(CONTINUE_READING))
    });
    match start {
        Some(start) => trim_spaces(&text[..start]),
        None => text,
    }
}

/// `text` with the `[…]` or `[...]` it ends with turned into `…`.
fn with_ellipsis_for_more_marker(text: &str) -> Cow<'_, str> {
    let Some(before) = text
        .strip_suffix("[…]")
        .or_else(|| text.strip_suffix("[...]"))
    else {
        return text.into();
    };
    let before = trim_spaces(before);
    if before.is_empty() || before.ends_with('…') || before.ends_with("...") {
        before.into()
    } else {
        format!("{before}…").into()
    }
}

/// `text` without the shortcode tags in it. Each leaves a space, so the words either side of it
/// stay apart.
fn without_shortcode_tags(text: &str) -> String {
    let mut without = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        without.push_str(&rest[..open]);
        rest = &rest[open..];
        match shortcode_tag_len(rest) {
            Some(len) => {
                without.push(' ');
                rest = &rest[len..];
            }
            None => {
                without.push('[');
                rest = &rest[1..];
            }
        }
    }
    without.push_str(rest);
    without
}

/// The length of the shortcode tag `text` starts with. `None` when what it starts with isn't
/// clearly one: a `[` is as likely to open `[1]` or `[sic]`.
///
/// A tag is `[/name]`, `[name key=value]`, `[name]` when `[/name]` follows it, or `[name]` when
/// the name has an underscore.
fn shortcode_tag_len(text: &str) -> Option<usize> {
    // Up to the first bracket of either kind, so a run of `[` isn't searched once for each.
    let close = 1 + text[1..].find(['[', ']'])?;
    if !text[close..].starts_with(']') {
        return None;
    }
    let inside = &text[1..close];
    let (is_closing, inside) = match inside.strip_prefix('/') {
        Some(inside) => (true, inside),
        None => (false, inside),
    };
    let name_len = inside
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(inside.len());
    let (name, attributes) = inside.split_at(name_len);
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let is_tag = if is_closing {
        attributes.trim().is_empty()
    } else if attributes.is_empty() {
        name.contains('_') || text[close..].contains(&format!("[/{name}]"))
    } else {
        attributes.starts_with(char::is_whitespace) && attributes.contains('=')
    };
    is_tag.then_some(close + 1)
}

/// `text` cut to `max_chars` characters at most, at the last space that leaves room for the `…`
/// it then ends with.
fn cut_to_length(text: String, max_chars: usize) -> String {
    // Where the text ends when it's a character short of the limit: one is kept back for the `…`.
    let Some((end, _)) = text.char_indices().nth(max_chars.saturating_sub(1)) else {
        return text;
    };
    if text[end..].chars().nth(1).is_none() {
        // Exactly `max_chars` long.
        return text;
    }
    let kept = match text[..end].rfind(' ') {
        Some(space) if space > 0 => &text[..space],
        // One long word, or a script that doesn't use spaces.
        _ => before_whole_character(&text, end),
    };
    let kept = kept.trim_end_matches(|c: char| is_space(c) || matches!(c, '.' | '…' | ',' | ';'));
    format!("{kept}…")
}

/// `text` up to `end`, or a little before it when that would cut a character from the marks or
/// joiners that follow it, such as an accent or an emoji's skin tone.
fn before_whole_character(text: &str, mut end: usize) -> &str {
    let joins_previous = |c: char| {
        matches!(c,
            '\u{200d}'
            | '\u{0300}'..='\u{036f}'
            | '\u{20d0}'..='\u{20ff}'
            | '\u{fe00}'..='\u{fe0f}'
            | '\u{1f3fb}'..='\u{1f3ff}'
            | '\u{e0020}'..='\u{e007f}')
    };
    while end > 0 {
        let next = text[end..].chars().next();
        let previous = text[..end].chars().next_back();
        if !next.is_some_and(joins_previous) && previous != Some('\u{200d}') {
            break;
        }
        end -= previous.map_or(0, char::len_utf8);
    }
    &text[..end]
}

/// A space that can be collapsed and trimmed. No-break spaces aren't: an author puts them
/// between words that belong on one line.
fn is_space(c: char) -> bool {
    c.is_whitespace() && !is_no_break_space(c)
}

fn is_no_break_space(c: char) -> bool {
    matches!(c, '\u{a0}' | '\u{202f}')
}

/// Some feeds start a post with one. It shows as nothing, and isn't whitespace, so it would
/// otherwise stay at the start of a summary.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// `text` on one line: each run of whitespace is one space, and there's none at either end. A
/// no-break space is kept, unless it's all there is between two spaces or at an end.
fn single_line(text: &str) -> String {
    let words: Vec<&str> = text
        .split(is_space)
        .filter(|word| !word.chars().all(is_no_break_space))
        .collect();
    trim_spaces(&words.join(" ")).to_string()
}

/// `text` without the whitespace at its ends, no-break spaces included.
fn trim_spaces(text: &str) -> &str {
    text.trim_matches(char::is_whitespace)
}

/// Which of a document's text [`html_text`] returns.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Text {
    /// Everything a reader reads.
    All,
    /// Without figures, their captions, and galleries, which the web Reader leaves out of a
    /// summary.
    WithoutFigures,
}

/// The text of `html`, up to `max_bytes` of it: tags are removed, character references are
/// decoded, and each run of whitespace is one space. A space stands in for each tag that ends a
/// line or a block, so `<p>One</p><p>Two</p>` and `One<br>Two` don't run their words together.
fn html_text(html: &str, which: Text, max_bytes: usize) -> String {
    let mut text = String::new();
    // The element whose content is being left out, and how many elements of its name are open.
    let mut hidden: Option<(&str, usize)> = None;
    for token in HtmlTokens::new(html) {
        if let Some((name, depth)) = &mut hidden {
            match token {
                HtmlToken::Open(tag) if tag.is(name) && !tag.is_self_closing => *depth += 1,
                HtmlToken::Close(closed) if closed.eq_ignore_ascii_case(name) => {
                    *depth -= 1;
                    if *depth == 0 {
                        hidden = None;
                    }
                }
                _ => {}
            }
            continue;
        }
        match token {
            HtmlToken::Text(encoded) => {
                for c in htmlize::unescape(encoded).chars() {
                    if is_space(c) {
                        if !text.ends_with(' ') {
                            text.push(' ');
                        }
                    } else if c != BYTE_ORDER_MARK {
                        text.push(c);
                    }
                }
            }
            HtmlToken::Open(tag) => {
                if separates_words(tag.name) && !text.ends_with(' ') {
                    text.push(' ');
                }
                if !tag.is_self_closing && hides_text(&tag, which) {
                    hidden = Some((tag.name, 1));
                }
            }
            HtmlToken::Close(name) => {
                if separates_words(name) && !text.ends_with(' ') {
                    text.push(' ');
                }
            }
        }
        if text.len() >= max_bytes {
            break;
        }
    }
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text
}

fn is_any_of(name: &str, names: &[&str]) -> bool {
    names.iter().any(|other| name.eq_ignore_ascii_case(other))
}

/// Whether the text either side of a `name` tag belongs to different words.
fn separates_words(name: &str) -> bool {
    is_any_of(
        name,
        &[
            "address",
            "article",
            "aside",
            "blockquote",
            "br",
            "dd",
            "div",
            "dl",
            "dt",
            "figcaption",
            "figure",
            "footer",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "header",
            "hr",
            "iframe",
            "li",
            "main",
            "nav",
            "ol",
            "p",
            "pre",
            "section",
            "table",
            "td",
            "th",
            "tr",
            "ul",
        ],
    )
}

/// Whether what the element `tag` opens holds is left out of the text.
fn hides_text(tag: &HtmlTag, which: Text) -> bool {
    // Not text a reader sees.
    if is_any_of(tag.name, &["script", "style", "noscript", "iframe", "svg"]) {
        return true;
    }
    // A template isn't shown, unless it's a declarative shadow root: a browser shows what that
    // holds in place of the element around it. A post can be nothing else.
    if tag.is("template") {
        return tag.attribute("shadowrootmode").is_none();
    }
    if which == Text::All {
        return false;
    }
    if is_any_of(tag.name, &["figure", "figcaption"]) {
        return true;
    }
    // An element that can't hold anything.
    if is_any_of(tag.name, &["img", "br", "hr", "input", "source", "embed"]) {
        return false;
    }
    let Some(classes) = tag.attribute("class") else {
        return false;
    };
    let mut classes = classes.split_ascii_whitespace();
    if tag.is("blockquote") {
        classes.any(|class| class.starts_with("instagram-"))
    } else {
        classes.any(|class| matches!(class, "wp-caption" | "tiled-gallery"))
    }
}

/// An `<img>` in a post's content.
struct ContentImage {
    src: String,
    /// The width and height the content gives it, in pixels.
    size: Option<(u32, u32)>,
}

/// The images in `html` that could be shown for the post, in order. It leaves out avatars,
/// smileys, tracking pixels and `data:` images, as the web Reader does.
fn content_images(html: &str) -> Vec<ContentImage> {
    HtmlTokens::new(html)
        .filter_map(|token| match token {
            HtmlToken::Open(tag) if tag.is("img") => Some(tag),
            _ => None,
        })
        .filter_map(|tag| {
            let src = tag.attribute("src")?;
            let src = src.trim();
            let lowercase = src.to_ascii_lowercase();
            if src.is_empty()
                || lowercase.starts_with("data:")
                || lowercase.contains("gravatar.com")
                || lowercase.contains("/wpcom-smileys/")
            {
                return None;
            }
            let pixels = |name: &str| tag.attribute(name)?.trim().parse::<u32>().ok();
            let size = match (pixels("width"), pixels("height")) {
                // A tracking pixel is 1 pixel wide or tall, and nothing much the other way.
                (Some(width), Some(height))
                    if (1..=2).contains(&(u64::from(width) + u64::from(height))) =>
                {
                    return None;
                }
                (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
                // WordPress.com records the original's size here: `"1024,768"`.
                _ => tag.attribute("data-orig-size").and_then(|size| {
                    let (width, height) = size.split_once(',')?;
                    let width = width.trim().parse::<u32>().ok()?;
                    let height = height.trim().parse::<u32>().ok()?;
                    (width > 0 && height > 0).then_some((width, height))
                }),
            };
            Some(ContentImage {
                src: src.to_string(),
                size,
            })
        })
        .collect()
}

/// The images of `images` that are big enough, and the right shape, for a gallery. An image that
/// appears twice counts once.
fn gallery_images(images: &[ContentImage]) -> impl Iterator<Item = &ContentImage> {
    let (min_width, min_height) = GALLERY_MIN_IMAGE_SIZE;
    let mut seen = std::collections::HashSet::new();
    images.iter().filter(move |image| {
        let Some((width, height)) = image.size else {
            return false;
        };
        let (wide, tall) = (u64::from(width), u64::from(height));
        width >= min_width
            && height >= min_height
            && tall * 3 >= wide
            && tall <= wide * 3
            && seen.insert(image.src.as_str())
    })
}

/// A piece of HTML, as [`HtmlTokens`] reads it.
enum HtmlToken<'a> {
    /// The text between two tags, with its character references not yet decoded.
    Text(&'a str),
    /// `<name …>`.
    Open(HtmlTag<'a>),
    /// `</name>`, as its name.
    Close(&'a str),
}

/// An opening tag.
struct HtmlTag<'a> {
    /// As written, in any letter case.
    name: &'a str,
    /// What follows the name, up to the `>`.
    attributes: &'a str,
    /// Whether it ends in `/>`. An RSS feed's XHTML closes an empty element this way.
    is_self_closing: bool,
}

impl<'a> HtmlTag<'a> {
    fn is(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    /// The value of the tag's first `name` attribute, with its character references decoded.
    fn attribute(&self, name: &str) -> Option<Cow<'a, str>> {
        HtmlAttributes::new(self.attributes)
            .find(|(found, _)| found.eq_ignore_ascii_case(name))
            .map(|(_, value)| htmlize::unescape_attribute(value))
    }
}

/// Reads HTML as its tags and the text between them, start to end, in one pass.
///
/// It isn't an HTML parser. A parser builds the tree of elements, and the time that takes grows
/// with how deeply they nest: a post that is 50,000 `<div>`s inside each other, which is half a
/// megabyte that anyone can publish to a tag, keeps a parser busy for seconds. Text and images
/// are in the tags and between them, whatever the nesting, so this reads those and builds
/// nothing.
///
/// It reads a tag the way a browser does: a `>` inside a quoted attribute doesn't end the tag,
/// comments and what's inside `<script>` and `<style>` are passed over, and a `<` that doesn't
/// start a tag is text.
struct HtmlTokens<'a> {
    rest: &'a str,
}

impl<'a> HtmlTokens<'a> {
    fn new(html: &'a str) -> Self {
        Self { rest: html }
    }

    /// Where the next tag, comment or declaration starts in what's left.
    fn next_markup(&self) -> Option<usize> {
        let mut from = 0;
        while let Some(found) = self.rest[from..].find('<') {
            let start = from + found;
            match self.rest.as_bytes().get(start + 1) {
                Some(next) if next.is_ascii_alphabetic() || matches!(next, b'/' | b'!' | b'?') => {
                    return Some(start);
                }
                _ => from = start + 1,
            }
        }
        None
    }

    /// Moves past `end`, the index of a `>` in what's left, or to the end when there is none.
    fn skip_past(&mut self, end: Option<usize>) {
        self.rest = end.map_or("", |end| &self.rest[end + 1..]);
    }
}

impl<'a> Iterator for HtmlTokens<'a> {
    type Item = HtmlToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.rest.is_empty() {
                return None;
            }
            let Some(start) = self.next_markup() else {
                return Some(HtmlToken::Text(std::mem::take(&mut self.rest)));
            };
            if start > 0 {
                let (text, rest) = self.rest.split_at(start);
                self.rest = rest;
                return Some(HtmlToken::Text(text));
            }

            // What's left starts with markup.
            self.rest = &self.rest[1..];
            if let Some(comment) = self.rest.strip_prefix("!--") {
                self.rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
                continue;
            }
            if self.rest.starts_with(['!', '?']) {
                self.skip_past(self.rest.find('>'));
                continue;
            }
            if let Some(closing) = self.rest.strip_prefix('/') {
                let name = tag_name(closing);
                self.rest = closing;
                self.skip_past(self.rest.find('>'));
                return Some(HtmlToken::Close(name));
            }

            let name = tag_name(self.rest);
            let after_name = &self.rest[name.len()..];
            let mut attributes = HtmlAttributes::new(after_name);
            attributes.by_ref().for_each(drop);
            let attributes_len = after_name.len() - attributes.rest.len();
            let attributes = &after_name[..attributes_len];
            self.rest = &after_name[attributes_len..];
            self.skip_past(self.rest.find('>'));
            let tag = HtmlTag {
                name,
                attributes,
                is_self_closing: attributes.trim_end().ends_with('/'),
            };

            // Script and style aren't markup: a `<` in them starts nothing. Pass over them, up to
            // the tag that closes them.
            if !tag.is_self_closing && is_any_of(name, &["script", "style"]) {
                let closing = find_closing_tag(self.rest, name);
                self.rest = closing.map_or("", |start| &self.rest[start..]);
            }
            return Some(HtmlToken::Open(tag));
        }
    }
}

/// The name a tag starts with, once past its `<` or `</`.
fn tag_name(tag: &str) -> &str {
    let end = tag
        .find(|c: char| c.is_ascii_whitespace() || matches!(c, '/' | '>'))
        .unwrap_or(tag.len());
    &tag[..end]
}

/// Where `</name` starts in `html`, in any letter case.
fn find_closing_tag(html: &str, name: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = html[from..].find("</") {
        let start = from + found;
        let is_name = html
            .get(start + 2..start + 2 + name.len())
            .is_some_and(|found| found.eq_ignore_ascii_case(name));
        if is_name {
            return Some(start);
        }
        from = start + 2;
    }
    None
}

/// Reads the attributes of a tag, as names and values, from what follows its name. It stops at
/// the `>` that ends the tag, which it leaves in `rest`.
///
/// A value is as written, without its quotes, and with character references not yet decoded. An
/// attribute without one has an empty value.
struct HtmlAttributes<'a> {
    rest: &'a str,
}

impl<'a> HtmlAttributes<'a> {
    fn new(after_name: &'a str) -> Self {
        Self { rest: after_name }
    }

    fn skip_while(&mut self, matches: impl Fn(u8) -> bool) -> &'a str {
        let end = self
            .rest
            .bytes()
            .position(|byte| !matches(byte))
            .unwrap_or(self.rest.len());
        let (skipped, rest) = self.rest.split_at(end);
        self.rest = rest;
        skipped
    }
}

impl<'a> Iterator for HtmlAttributes<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_while(|byte| byte.is_ascii_whitespace() || byte == b'/');
        if self.rest.is_empty() || self.rest.starts_with('>') {
            return None;
        }
        // A name runs to a space, `=`, `/` or `>`. A `=` it starts with is part of it.
        let first = self.rest.chars().next().map_or(0, char::len_utf8);
        let name_len = self.rest[first..]
            .bytes()
            .position(|byte| byte.is_ascii_whitespace() || matches!(byte, b'=' | b'/' | b'>'))
            .map_or(self.rest.len(), |position| first + position);
        let (name, rest) = self.rest.split_at(name_len);
        self.rest = rest;

        self.skip_while(|byte| byte.is_ascii_whitespace());
        let Some(after_equals) = self.rest.strip_prefix('=') else {
            return Some((name, ""));
        };
        self.rest = after_equals;
        self.skip_while(|byte| byte.is_ascii_whitespace());
        let value = match self.rest.chars().next() {
            Some(quote @ ('"' | '\'')) => {
                let quoted = &self.rest[1..];
                let end = quoted.find(quote);
                let value = &quoted[..end.unwrap_or(quoted.len())];
                self.rest = end.map_or("", |end| &quoted[end + 1..]);
                value
            }
            _ => self.skip_while(|byte| !byte.is_ascii_whitespace() && byte != b'>'),
        };
        Some((name, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    #[test]
    fn test_content_image_urls_are_every_image_once_in_order() {
        let content = concat!(
            r#"<p><img src="https://example.com/a.jpg" width="20" height="20"></p>"#,
            r#"<img src='//example.com/b.png'/>"#,
            r#"<img src="https://example.com/a.jpg">"#,
            r#"<img src="http://example.com/c.gif?w=1&amp;h=2">"#,
        );
        assert_eq!(
            reader_post_content_image_urls(content),
            [
                "https://example.com/a.jpg",
                "https://example.com/b.png",
                "http://example.com/c.gif?w=1&h=2",
            ]
        );
    }

    #[rstest]
    #[case::a_data_image(r#"<img src="data:image/png;base64,AAAA">"#)]
    #[case::an_address_that_is_not_whole(r#"<img src="/images/a.png">"#)]
    #[case::no_address(r#"<img alt="nothing">"#)]
    #[case::an_empty_address(r#"<img src=" ">"#)]
    #[case::a_tracking_pixel(r#"<img src="https://example.com/t.gif" width="1" height="1">"#)]
    #[case::a_stats_pixel(r#"<img src="https://pixel.wp.com/b.gif?host=example.com">"#)]
    #[case::a_jetpack_stats_pixel(r#"<img src="//stats.wp.com/g.gif?v=ext">"#)]
    #[case::an_image_in_a_script(
        r#"<script>var a = '<img src="https://example.com/a.jpg">';</script>"#
    )]
    #[case::an_image_in_a_comment(r#"<!-- <img src="https://example.com/a.jpg"> -->"#)]
    fn test_content_image_urls_leave_out(#[case] content: &str) {
        assert_eq!(
            reader_post_content_image_urls(content),
            Vec::<String>::new()
        );
    }

    fn stored(images: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        images
            .iter()
            .map(|(url, copy)| (url.to_string(), copy.to_string()))
            .collect()
    }

    #[test]
    fn test_stored_images_replace_their_addresses() {
        let content = concat!(
            "<p>Before <b>text</b> &amp; more.</p>",
            r#"<img class="wide" src="https://example.com/a.jpg" srcset="https://example.com/a.jpg 1x, https://example.com/a-2x.jpg 2x" sizes="100vw" alt='An "A"'>"#,
            r#"<img src="https://example.com/b.jpg" />"#,
            "<p>After.</p>",
        );
        assert_eq!(
            reader_post_content_with_stored_images(
                content,
                stored(&[("https://example.com/a.jpg", "file:///saved/1.jpg")])
            ),
            concat!(
                "<p>Before <b>text</b> &amp; more.</p>",
                r#"<img class="wide" src="file:///saved/1.jpg" alt='An "A"'>"#,
                r#"<img src="https://example.com/b.jpg" />"#,
                "<p>After.</p>",
            )
        );
    }

    #[test]
    fn test_stored_images_keep_a_tag_that_closes_itself() {
        assert_eq!(
            reader_post_content_with_stored_images(
                r#"<img src="//example.com/a.jpg" hidden/>"#,
                stored(&[("https://example.com/a.jpg", "reader://a b&c.jpg")])
            ),
            r#"<img src="reader://a b&amp;c.jpg" hidden />"#
        );
    }

    /// A `<picture>` would otherwise load one of its sources from the network, and never show
    /// the stored `<img>`.
    #[test]
    fn test_stored_images_take_the_sizes_off_a_picture() {
        let content = concat!(
            r#"<picture><source srcset="https://example.com/a.webp" type="image/webp">"#,
            r#"<img src="https://example.com/a.jpg"></picture>"#,
            r#"<video><source src="https://example.com/v.mp4"></video>"#,
        );
        assert_eq!(
            reader_post_content_with_stored_images(
                content,
                stored(&[("https://example.com/a.jpg", "1.jpg")])
            ),
            concat!(
                r#"<picture><source type="image/webp">"#,
                r#"<img src="1.jpg"></picture>"#,
                r#"<video><source src="https://example.com/v.mp4"></video>"#,
            )
        );
    }

    #[rstest]
    #[case::nothing_stored(stored(&[]))]
    #[case::another_image_stored(stored(&[("https://example.com/other.jpg", "1.jpg")]))]
    fn test_content_without_a_stored_image_is_unchanged(
        #[case] stored: std::collections::HashMap<String, String>,
    ) {
        let content = r#"<p>Text</p><img src="https://example.com/a.jpg" srcset="a 1x"><br/>"#;
        assert_eq!(
            reader_post_content_with_stored_images(content, stored),
            content
        );
    }

    /// Text that isn't ASCII keeps its bytes, since the content is copied by position.
    #[test]
    fn test_stored_images_keep_text_that_is_not_ascii() {
        let content = "<p>Čaj — 茶</p><img src=\"https://example.com/a.jpg\" alt=\"茶\"><p>🍵</p>";
        assert_eq!(
            reader_post_content_with_stored_images(
                content,
                stored(&[("https://example.com/a.jpg", "1.jpg")])
            ),
            "<p>Čaj — 茶</p><img src=\"1.jpg\" alt=\"茶\"><p>🍵</p>"
        );
    }

    fn post(fields: serde_json::Value) -> ReaderPost {
        let mut post = json!({ "ID": 1, "site_ID": 2, "is_external": false });
        post.as_object_mut()
            .unwrap()
            .extend(fields.as_object().unwrap().clone());
        serde_json::from_value(post).expect("the post parses")
    }

    fn summary(excerpt: &str, content: &str) -> Option<String> {
        reader_post_summary(&post(json!({ "excerpt": excerpt, "content": content })))
    }

    fn summary_of_excerpt(excerpt: &str) -> String {
        summary(excerpt, "").expect("the excerpt has text")
    }

    fn summary_of_content(content: &str) -> String {
        summary("", content).expect("the content has text")
    }

    /// An excerpt as the API sends it: a paragraph, with character references, cut with `[…]`.
    #[test]
    fn test_summary_of_an_excerpt_the_api_cut() {
        assert_eq!(
            summary_of_excerpt(
                "<p>Here&#8217;s what&#8217;s new in WordPress 6.5 &#8211; fonts, and more [&hellip;]</p>\n"
            ),
            "Here’s what’s new in WordPress 6.5 – fonts, and more…"
        );
    }

    #[rstest]
    #[case::a_paragraph("<p>One paragraph.</p>\n", "One paragraph.")]
    #[case::plain_text("Plain text.", "Plain text.")]
    #[case::inline_tags(
        "<p>Some <b>bold</b> and <a href=\"https://example.com\">linked</a> text.</p>",
        "Some bold and linked text."
    )]
    #[case::a_tag_inside_a_word("<p><strong>W</strong>ord</p>", "Word")]
    #[case::paragraphs("<p>One.</p><p>Two.</p>", "One. Two.")]
    #[case::line_breaks("<p>One<br>Two<br />Three</p>", "One Two Three")]
    #[case::list_items("<ul><li>One</li><li>Two</li></ul>", "One Two")]
    #[case::a_heading("<h2>Heading</h2><p>Text.</p>", "Heading Text.")]
    #[case::newlines_and_tabs("<p>One\n\ttwo   three</p>\n", "One two three")]
    #[case::character_references(
        "Tom &amp; Jerry &#8211; &lt;em&gt; &hellip;",
        "Tom & Jerry – <em> …"
    )]
    #[case::a_script("<p>Text.</p><script>alert('no');</script>", "Text.")]
    #[case::a_style("<style>p { color: red; }</style><p>Text.</p>", "Text.")]
    #[case::a_comment("<!-- wp:paragraph --><p>Text.</p><!-- /wp:paragraph -->", "Text.")]
    #[case::an_unclosed_tag("<p>Text <b>bold", "Text bold")]
    #[case::not_a_tag("1 < 2 and 3 > 2", "1 < 2 and 3 > 2")]
    fn test_summary_is_plain_text_on_one_line(#[case] excerpt: &str, #[case] expected: &str) {
        assert_eq!(summary_of_excerpt(excerpt), expected);
    }

    /// The iOS app's tests of the summary it makes from an excerpt.
    #[rstest]
    #[case::is_plain_text(
        "<h1>Sample <b>text</b> &amp; sample text</h1>",
        "Sample text & sample text"
    )]
    #[case::converts_line_breaks_to_spaces("<p>Yes,<br>look behind</p>", "Yes, look behind")]
    fn test_summary_as_the_ios_app_tests_it(#[case] excerpt: &str, #[case] expected: &str) {
        assert_eq!(summary_of_excerpt(excerpt), expected);
    }

    /// The iOS app's tests of the text it takes from a paragraph of content.
    #[rstest]
    #[case::br_tags(
        r#"<p class="wp-block-paragraph">Yes,<br>look behind<br><br>in remembrance and with gratitude.</p>"#,
        "Yes, look behind in remembrance and with gratitude."
    )]
    #[case::br_tags_with_attributes(
        r#"<p>foo<br class="clear">bar<br clear="all">baz<br />qux</p>"#,
        "foo bar baz qux"
    )]
    #[case::whitespace_around_br_tags(
        "<p>Hello <br> world<br>  <br>again</p>",
        "Hello world again"
    )]
    #[case::a_no_break_space("<p>some contents&nbsp;go here</p>", "some contents\u{a0}go here")]
    #[case::a_gallery_before_the_text(
        r#"<!-- wp:gallery {"ids":[2315,2309]} --><figure class="wp-block-gallery columns-3 is-cropped"><ul class="blocks-gallery-grid"><li class="blocks-gallery-item"><figure><img src="https://example.com/1.jpg" data-id="2315" class="wp-image-2315"/><figcaption class="blocks-gallery-item__caption">Asdasdasd</figcaption></figure></li><li class="blocks-gallery-item"><figure><img src="https://example.com/2.jpg" data-id="2309" class="wp-image-2309"/><figcaption class="blocks-gallery-item__caption">Asdasdasd</figcaption></figure></li></ul></figure><!-- /wp:gallery --><p>Some Content</p>"#,
        "Some Content"
    )]
    fn test_summary_of_content_as_the_ios_app_tests_it(
        #[case] content: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(summary_of_content(content), expected);
    }

    /// Where iOS takes the first paragraph, this takes the text around a figure too.
    #[test]
    fn test_summary_of_content_leaves_out_a_video_block() {
        let content = "<p>Before</p>\n<!-- wp:videopress/video {\"id\":5297} -->\n<figure class=\"wp-block-videopress-video wp-block-jetpack-videopress jetpack-videopress-player\"><div class=\"jetpack-videopress-player__wrapper\">\nhttps://videopress.com/v/AbCDe?resizeToParent=true&amp;cover=true\n</div></figure>\n<!-- /wp:videopress/video -->\n<p>After</p>";
        assert_eq!(summary_of_content(content), "Before After");
    }

    #[rstest]
    #[case::a_figure(
        r#"<figure class="wp-block-image"><img src="https://example.com/a.jpg"/><figcaption>A caption</figcaption></figure><p>Text.</p>"#,
        "Text."
    )]
    #[case::a_classic_caption(
        r#"<div class="wp-caption alignnone"><img src="https://example.com/a.jpg"/><p class="wp-caption-text">A caption</p></div><p>Text.</p>"#,
        "Text."
    )]
    #[case::a_tiled_gallery(
        r#"<div class="tiled-gallery type-rectangular"><div class="tiled-gallery-caption">A caption</div></div><p>Text.</p>"#,
        "Text."
    )]
    #[case::an_instagram_embed(
        r#"<blockquote class="instagram-media"><p>A post shared by someone</p></blockquote><p>Text.</p>"#,
        "Text."
    )]
    #[case::a_quote_is_kept(
        "<blockquote><p>A quote.</p></blockquote><p>Text.</p>",
        "A quote. Text."
    )]
    #[case::code_is_kept(
        r#"<pre class="wp-block-code"><code>let x = 1</code></pre><p>Text.</p>"#,
        "let x = 1 Text."
    )]
    #[case::no_paragraphs(
        "Text an RSS feed sent<br/>with no paragraphs.",
        "Text an RSS feed sent with no paragraphs."
    )]
    fn test_summary_of_content(#[case] content: &str, #[case] expected: &str) {
        assert_eq!(summary_of_content(content), expected);
    }

    /// A post that is a photo and its caption has no text outside the figure.
    #[rstest]
    #[case::a_figure(
        r#"<figure class="wp-block-image"><img src="https://example.com/a.jpg"/><figcaption>A heron on the river</figcaption></figure>"#,
        "A heron on the river"
    )]
    #[case::a_classic_caption(
        r#"<div class="wp-caption aligncenter"><a href="https://example.com/a.jpg"><img src="https://example.com/a.jpg"/></a><p class="wp-caption-text">The harbour at dawn</p></div>"#,
        "The harbour at dawn"
    )]
    fn test_summary_of_content_that_is_only_a_caption(
        #[case] content: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(summary_of_content(content), expected);
    }

    /// The iOS app's test of falling back to the content.
    #[test]
    fn test_summary_falls_back_to_the_content_when_the_excerpt_is_empty() {
        assert_eq!(
            summary("", "<p>Generated from the post content.</p>").as_deref(),
            Some("Generated from the post content.")
        );
    }

    #[rstest]
    #[case::empty("")]
    #[case::whitespace(" \n ")]
    #[case::an_empty_paragraph("<p></p>\n")]
    #[case::a_no_break_space("<p>&nbsp;</p>")]
    #[case::only_the_more_marker("<p>[&hellip;]</p>")]
    #[case::only_an_image(r#"<p><img src="https://example.com/a.jpg" alt="Alt text"/></p>"#)]
    fn test_summary_falls_back_when_the_excerpt_has_no_text(#[case] excerpt: &str) {
        assert_eq!(
            summary(excerpt, "<p>From the content.</p>").as_deref(),
            Some("From the content.")
        );
    }

    #[test]
    fn test_summary_prefers_the_excerpt() {
        assert_eq!(
            summary("<p>The excerpt.</p>", "<p>The content.</p>").as_deref(),
            Some("The excerpt.")
        );
    }

    #[rstest]
    #[case::neither(json!({}))]
    #[case::both_empty(json!({ "excerpt": "", "content": "" }))]
    #[case::only_an_image(json!({
        "excerpt": "",
        "content": r#"<figure><img src="https://example.com/a.jpg" alt="Alt text"/></figure>"#,
    }))]
    #[case::only_a_video(json!({
        "excerpt": "<p>\u{a0}</p>\n",
        "content": r#"<span class="embed-youtube"><iframe src="https://www.youtube.com/embed/abc">Your browser can't show frames.</iframe></span>"#,
    }))]
    fn test_no_summary_without_text(#[case] fields: serde_json::Value) {
        assert_eq!(reader_post_summary(&post(fields)), None);
    }

    #[rstest]
    #[case::the_api_marker("<p>It goes on [&hellip;]</p>\n", "It goes on…")]
    #[case::decoded("It goes on […]", "It goes on…")]
    #[case::three_dots("It goes on [...]", "It goes on…")]
    #[case::no_space_before("It goes on[…]", "It goes on…")]
    #[case::after_an_ellipsis("It goes on… […]", "It goes on…")]
    #[case::after_three_dots("It goes on... […]", "It goes on...")]
    #[case::an_ellipsis_is_kept("It goes on…", "It goes on…")]
    #[case::not_at_the_end("It […] goes on", "It […] goes on")]
    fn test_summary_more_marker(#[case] excerpt: &str, #[case] expected: &str) {
        assert_eq!(summary_of_excerpt(excerpt), expected);
    }

    #[rstest]
    #[case::at_the_end(
        "<p>Working on a strategy for the year&hellip; <a href=\"https://example.com/p\">Continue reading <span class=\"meta-nav\">&rarr;</span></a></p>",
        "Working on a strategy for the year…"
    )]
    #[case::any_letter_case("Some text. CONTINUE READING →", "Some text.")]
    #[case::and_what_follows("Some text. Continue reading → Share this", "Some text.")]
    #[case::after_the_more_marker("Some text [&hellip;] Continue reading &rarr;", "Some text…")]
    #[case::without_the_arrow_is_kept(
        "Continue reading to find out more.",
        "Continue reading to find out more."
    )]
    fn test_summary_continue_reading(#[case] excerpt: &str, #[case] expected: &str) {
        assert_eq!(summary_of_excerpt(excerpt), expected);
    }

    #[rstest]
    #[case::with_an_attribute("[template id=”123″] The text.", "The text.")]
    #[case::with_straight_quotes(r#"[gallery ids="1,2,3" columns="2"] The text."#, "The text.")]
    #[case::closing("The text.[/vc_column_text]", "The text.")]
    #[case::a_pair("[time]10:30[/time] The text.", "10:30 The text.")]
    #[case::a_name_with_an_underscore("[vc_row][vc_column]The text.", "The text.")]
    #[case::between_words("One[/b]two", "One two")]
    #[case::and_the_more_marker("[vc_row]The text [&hellip;]", "The text…")]
    #[case::the_ios_apps_example(
        r#"Lorem ipsum dolor sit amet, [shortcode param="value"]consectetur[/shortcode] adipiscing elit."#,
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit."
    )]
    fn test_summary_without_shortcode_tags(#[case] excerpt: &str, #[case] expected: &str) {
        assert_eq!(summary_of_excerpt(excerpt), expected);
    }

    /// Text in square brackets that the recorded posts had, and that isn't a shortcode.
    #[rstest]
    #[case("A footnote [1] in the text.")]
    #[case("They wrote it thier [sic] way.")]
    #[case("[C] Some chords [G] here")]
    #[case("[Verse 1] The first line")]
    #[case("Send a [GET] request")]
    #[case("An empty [] pair and a lone [ bracket")]
    #[case("A list [a, b, c] of things")]
    #[case("Was [REDACTED] by them")]
    #[case("B[Math.floor(Math.random()*B.length)]")]
    #[case("[2026] In review")]
    #[case("[more...]")]
    #[case("[link text = here")]
    fn test_summary_keeps_other_text_in_square_brackets(#[case] excerpt: &str) {
        assert_eq!(summary_of_excerpt(excerpt), excerpt);
    }

    #[test]
    fn test_summary_no_break_spaces() {
        // Kept between words, as in French punctuation.
        assert_eq!(
            summary_of_excerpt("«&nbsp;Bonjour&nbsp;»"),
            "«\u{a0}Bonjour\u{a0}»"
        );
        assert_eq!(summary_of_excerpt("10&#8239;km"), "10\u{202f}km");
        // Not kept where it's only padding.
        assert_eq!(
            summary_of_excerpt("&nbsp;One. &nbsp; Two.&nbsp;"),
            "One. Two."
        );
    }

    #[test]
    fn test_summary_shorter_than_the_limit_is_not_cut() {
        let exactly = "a".repeat(SUMMARY_MAX_CHARS);
        assert_eq!(summary_of_excerpt(&exactly), exactly);
        let words = "word ".repeat(100);
        assert_eq!(summary_of_excerpt(&words), words.trim_end());
    }

    #[test]
    fn test_summary_is_cut_at_a_space() {
        // 101 words of 4 letters and a space: 504 characters once trimmed.
        let summary = summary_of_excerpt(&"word ".repeat(101));
        assert_eq!(summary, format!("{}…", "word ".repeat(99).trim_end()));
        assert!(summary.chars().count() <= SUMMARY_MAX_CHARS);
    }

    #[test]
    fn test_summary_cut_from_the_content_too() {
        let summary = summary_of_content(&format!("<p>{}</p>", "word ".repeat(400)));
        assert_eq!(summary, format!("{}…", "word ".repeat(99).trim_end()));
    }

    #[test]
    fn test_summary_cut_leaves_no_punctuation_before_the_ellipsis() {
        let excerpt = format!("{}end. Then more words follow here.", "word ".repeat(98));
        assert_eq!(
            summary_of_excerpt(&excerpt),
            format!("{}end…", "word ".repeat(98))
        );
    }

    #[test]
    fn test_summary_of_one_long_word_is_cut_to_the_limit() {
        let summary = summary_of_excerpt(&"a".repeat(SUMMARY_MAX_CHARS + 1));
        assert_eq!(summary, format!("{}…", "a".repeat(SUMMARY_MAX_CHARS - 1)));
    }

    /// Characters are counted, not bytes, so text with no spaces is cut between characters.
    #[test]
    fn test_summary_of_text_without_spaces_is_cut_between_characters() {
        let summary = summary_of_excerpt(&"日本語".repeat(200));
        assert_eq!(summary.chars().count(), SUMMARY_MAX_CHARS);
        assert!(
            summary.ends_with("語日…") || summary.ends_with("本語…") || summary.ends_with("日本…")
        );
    }

    #[rstest]
    #[case::a_skin_tone("👍\u{1f3fd}")]
    #[case::a_joined_emoji("👩\u{200d}💻")]
    #[case::an_accent("e\u{301}")]
    fn test_summary_cut_keeps_a_character_whole(#[case] character: &str) {
        // The limit falls inside the character that follows 498 letters.
        let excerpt = format!("{}{}{}", "a".repeat(498), character, "b".repeat(20));
        assert_eq!(
            summary_of_excerpt(&excerpt),
            format!("{}…", "a".repeat(498))
        );
    }

    #[test]
    fn test_summary_of_deeply_nested_content() {
        let content = format!("{}Deep{}", "<div>".repeat(20_000), "</div>".repeat(20_000));
        assert_eq!(summary_of_content(&content), "Deep");
    }

    fn reading_time(fields: serde_json::Value) -> Option<u32> {
        reader_post_reading_time_minutes(&post(fields))
    }

    /// The word counts the iOS app and the web Reader test theirs with, at 200 words a minute and
    /// rounded up.
    #[rstest]
    #[case(1, 1)]
    #[case(50, 1)]
    #[case(100, 1)]
    #[case(189, 1)]
    #[case(190, 1)]
    #[case(200, 1)]
    #[case(201, 2)]
    #[case(250, 2)]
    #[case(500, 3)]
    #[case(700, 4)]
    #[case(1000, 5)]
    #[case(100_000, 500)]
    fn test_reading_time_from_the_word_count(#[case] word_count: u64, #[case] minutes: u32) {
        assert_eq!(
            reading_time(json!({ "word_count": word_count })),
            Some(minutes)
        );
    }

    #[test]
    fn test_reading_time_from_a_word_count_sent_as_a_string() {
        assert_eq!(reading_time(json!({ "word_count": "450" })), Some(3));
    }

    #[rstest]
    #[case::nothing(json!({}))]
    #[case::no_words(json!({ "word_count": 0 }))]
    #[case::no_words_and_no_content(json!({ "word_count": 0, "content": "" }))]
    #[case::only_an_image(json!({ "content": r#"<p><img src="https://example.com/a.jpg" alt="Alt text"/></p>"# }))]
    #[case::only_a_script(json!({ "content": "<script>var words = 'not for reading';</script>" }))]
    fn test_no_reading_time_without_words(#[case] fields: serde_json::Value) {
        assert_eq!(reading_time(fields), None);
    }

    #[rstest]
    #[case::not_sent(json!({ "content": format!("<p>{}</p>", "word ".repeat(450)) }), 3)]
    #[case::zero(json!({ "word_count": 0, "content": format!("<p>{}</p>", "word ".repeat(450)) }), 3)]
    #[case::a_few_words(json!({ "content": "<p>Just a few words.</p>" }), 1)]
    #[case::tags_are_not_words(
        json!({ "content": format!(r#"<p class="a b c d e f">{}</p><img src="x.jpg" alt="not counted"/>"#, "word ".repeat(200)) }),
        1
    )]
    #[case::paragraphs_are_separate_words(
        json!({ "content": "<p>word</p>".repeat(201) }),
        2
    )]
    #[case::captions_are_read_too(
        json!({ "content": format!("<figure><figcaption>{}</figcaption></figure>", "word ".repeat(201)) }),
        2
    )]
    // A post written as a page of its own, inside a declarative shadow root. Recorded from a
    // public post, with its text replaced.
    #[case::a_shadow_root(
        json!({ "content": format!(
            concat!(
                r#"<div class="is-reader"><div class="example-html" style="display: block;">"#,
                r#"<template shadowrootmode="open"><div style='all: initial; display: block;'>"#,
                "<!-- Title: An example -->",
                r#"<header class="hero"><h1>An example</h1></header><section><p>{}</p>"#,
                r#"<svg viewbox="0 0 10 10" role="img"><title>A chart</title></svg></section>"#,
                r#"<script type="application/ld+json">{{"@type": "BlogPosting"}}</script>"#,
                "</div></template></div></div>",
            ),
            "word ".repeat(400),
        ) }),
        3
    )]
    fn test_reading_time_counts_the_words_in_the_content(
        #[case] fields: serde_json::Value,
        #[case] minutes: u32,
    ) {
        assert_eq!(reading_time(fields), Some(minutes));
    }

    #[rstest]
    #[case::words("One two three", 3)]
    #[case::punctuation_is_not_a_word("One – two & three … !", 3)]
    #[case::punctuation_in_a_word("it's well-known, e.g. 3.5", 4)]
    #[case::no_break_spaces("One\u{a0}two", 2)]
    #[case::chinese("我们今天去公园。", 7)]
    #[case::japanese("今日はいい天気ですね", 10)]
    #[case::chinese_and_english("使用 WordPress 的方法", 6)]
    #[case::accented_letters("Ünïcödé çà", 2)]
    #[case::an_emoji_is_not_a_word("Done 🎉", 1)]
    #[case::nothing("", 0)]
    #[case::only_punctuation("… — !", 0)]
    fn test_count_words(#[case] text: &str, #[case] words: u64) {
        assert_eq!(count_words(text), words);
    }

    /// Chinese has no spaces between words: counted between spaces, this would be one word.
    #[test]
    fn test_reading_time_of_chinese_content() {
        let fields = json!({ "content": format!("<p>{}</p>", "字".repeat(450)) });
        assert_eq!(reading_time(fields), Some(3));
    }

    #[test]
    fn test_reading_time_prefers_the_word_count() {
        let fields = json!({ "word_count": 1000, "content": "<p>A few words.</p>" });
        assert_eq!(reading_time(fields), Some(5));
    }

    fn card_type(fields: serde_json::Value) -> ReaderPostCardType {
        reader_post_card_type(&post(fields))
    }

    fn image(name: &str, width: u32, height: u32) -> String {
        format!(r#"<img src="https://example.com/{name}.jpg" width="{width}" height="{height}"/>"#)
    }

    /// The five images the web Reader tests its gallery rule with.
    fn gallery_content() -> String {
        [
            image("foo1", 800, 600),
            image("foo2", 1024, 768),
            image("foo3", 640, 480),
            image("foo4", 1024, 768),
            image("foo5", 1024, 768),
        ]
        .join("\n")
    }

    const XPOST_METADATA: &str = r#"[
        { "id": "1", "key": "xpost_origin", "value": "123:456" },
        { "id": "2", "key": "_xpost_original_permalink", "value": "https://example.com/p" }
    ]"#;

    fn xpost_metadata() -> serde_json::Value {
        serde_json::from_str(XPOST_METADATA).unwrap()
    }

    #[test]
    fn test_card_type_of_an_empty_post() {
        assert_eq!(card_type(json!({})), ReaderPostCardType::Default);
    }

    #[test]
    fn test_card_type_of_a_post_with_text_and_an_image() {
        let fields = json!({
            "featured_image": "https://example.com/featured.jpg",
            "content": format!("<p>{}</p>", "word ".repeat(100)),
        });
        assert_eq!(card_type(fields), ReaderPostCardType::Default);
    }

    #[test]
    fn test_card_type_of_a_cross_post() {
        let fields = json!({ "metadata": xpost_metadata() });
        assert_eq!(card_type(fields), ReaderPostCardType::CrossPost);
    }

    /// A cross-post is a cross-post whatever else it has.
    #[test]
    fn test_card_type_of_a_cross_post_with_images() {
        let fields = json!({ "metadata": xpost_metadata(), "content": gallery_content() });
        assert_eq!(card_type(fields), ReaderPostCardType::CrossPost);
    }

    #[rstest]
    #[case::no_post_id(json!([{ "key": "xpost_origin", "value": "123" }]))]
    #[case::not_ids(json!([{ "key": "xpost_origin", "value": "site:post" }]))]
    #[case::zero(json!([{ "key": "xpost_origin", "value": "0:0" }]))]
    #[case::only_the_permalink(json!([{ "key": "_xpost_original_permalink", "value": "https://example.com/p" }]))]
    fn test_card_type_of_a_cross_post_without_the_original_posts_ids(
        #[case] metadata: serde_json::Value,
    ) {
        assert_eq!(
            card_type(json!({ "metadata": metadata })),
            ReaderPostCardType::Default
        );
    }

    #[test]
    fn test_card_type_of_a_gallery() {
        let fields = json!({ "content": gallery_content() });
        assert_eq!(card_type(fields), ReaderPostCardType::Gallery);
    }

    /// The API picks an image out of the content as `featured_media`. That doesn't make the post
    /// any less of a gallery.
    #[test]
    fn test_card_type_of_a_gallery_with_an_image_the_api_picked() {
        let fields = json!({
            "content": gallery_content(),
            "featured_media": { "uri": "https://example.com/foo1.jpg", "type": "image", "width": 800, "height": 600 },
        });
        assert_eq!(card_type(fields), ReaderPostCardType::Gallery);
    }

    #[rstest]
    #[case::a_featured_image(json!({ "featured_image": "https://example.com/featured.jpg" }))]
    #[case::an_image_an_editor_picked(json!({ "editorial": { "image": "https://example.com/picked.jpg" } }))]
    fn test_card_type_is_not_a_gallery_when_the_post_has_its_own_image(
        #[case] mut fields: serde_json::Value,
    ) {
        fields["content"] = json!(gallery_content());
        assert_eq!(card_type(fields), ReaderPostCardType::Default);
    }

    #[rstest]
    #[case::four_images([image("a", 800, 600), image("b", 800, 600), image("c", 800, 600), image("d", 800, 600)].join(""))]
    #[case::one_too_narrow(format!("{}{}", gallery_content().replace("foo5", "foo4"), image("small", 299, 200)))]
    #[case::one_too_short(format!("{}{}", gallery_content().replace("foo5", "foo4"), image("short", 300, 71)))]
    #[case::one_too_wide(format!("{}{}", gallery_content().replace("foo5", "foo4"), image("banner", 1000, 300)))]
    #[case::one_too_tall(format!("{}{}", gallery_content().replace("foo5", "foo4"), image("tower", 300, 1000)))]
    #[case::one_twice(gallery_content().replace("foo5", "foo4"))]
    #[case::no_sizes(gallery_content().replace("width=", "data-w=").replace("height=", "data-h="))]
    fn test_card_type_is_not_a_gallery_without_five_images_that_count(#[case] content: String) {
        assert_eq!(
            card_type(json!({ "content": content })),
            ReaderPostCardType::Default
        );
    }

    /// The smallest image that counts, and the longest shapes that do.
    #[test]
    fn test_gallery_images_at_the_limits() {
        let content = [
            image("smallest", 300, 100),
            image("tallest", 300, 900),
            image("widest", 900, 300),
            image("d", 800, 600),
            image("e", 800, 600),
        ]
        .join("");
        assert_eq!(
            card_type(json!({ "content": content })),
            ReaderPostCardType::Gallery
        );
    }

    #[test]
    fn test_gallery_image_urls() {
        let content = format!(
            r#"<p>Some photos.</p>
            {}
            <img src="https://example.com/no-size.jpg"/>
            <img src="https://example.com/small.jpg" width="100" height="100"/>
            <img src="https://example.com/foo1.jpg" width="800" height="600"/>
            <img src="https://example.com/orig.jpg?w=150" data-orig-size="1024,768"/>
            <img src="https://example.com/percent.jpg" width="100%" height="auto" data-orig-size="640,480"/>
            <img src="https://2.gravatar.com/avatar/abc?s=400" width="400" height="400"/>
            <img src="https://s0.wp.com/wp-content/mu-plugins/wpcom-smileys/twemoji/smile.png" width="400" height="400"/>
            <img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" width="400" height="400"/>
            <img src="https://example.com/a.jpg?w=640&amp;h=480" width="640" height="480"/>"#,
            gallery_content()
        );
        assert_eq!(
            reader_post_gallery_image_urls(&content),
            [
                "https://example.com/foo1.jpg",
                "https://example.com/foo2.jpg",
                "https://example.com/foo3.jpg",
                "https://example.com/foo4.jpg",
                "https://example.com/foo5.jpg",
                "https://example.com/orig.jpg?w=150",
                "https://example.com/percent.jpg",
                "https://example.com/a.jpg?w=640&h=480",
            ]
        );
    }

    #[test]
    fn test_gallery_image_urls_are_ten_at_most() {
        let content: String = (0..15)
            .map(|n| image(&format!("photo{n}"), 800, 600))
            .collect();
        let urls = reader_post_gallery_image_urls(&content);
        assert_eq!(urls.len(), 10);
        assert_eq!(urls[9], "https://example.com/photo9.jpg");
    }

    #[rstest]
    #[case::no_content("")]
    #[case::no_images("<p>Text.</p>")]
    fn test_gallery_image_urls_of_content_without_images(#[case] content: &str) {
        assert!(reader_post_gallery_image_urls(content).is_empty());
    }

    fn photo_fields() -> serde_json::Value {
        json!({
            "featured_image": "https://example.com/photo.jpg",
            "featured_media": { "uri": "https://example.com/photo.jpg", "type": "image", "width": 1000, "height": 750 },
            "content": r#"<figure><img src="https://example.com/photo.jpg"/><figcaption>A caption that can be as long as it likes, because the photo card doesn't count captions as the post's text.</figcaption></figure><p>no no no no no</p>"#,
        })
    }

    #[test]
    fn test_card_type_of_a_photo() {
        assert_eq!(card_type(photo_fields()), ReaderPostCardType::Photo);
    }

    #[test]
    fn test_card_type_of_a_photo_with_no_text() {
        let mut fields = photo_fields();
        fields["content"] = json!(r#"<img src="https://example.com/photo.jpg"/>"#);
        assert_eq!(card_type(fields), ReaderPostCardType::Photo);
    }

    /// Without a width from `featured_media`, the one the content gives the image is used.
    #[rstest]
    #[case::wide_enough(440, ReaderPostCardType::Photo)]
    #[case::too_narrow(439, ReaderPostCardType::Default)]
    fn test_card_type_of_a_photo_by_the_width_in_the_content(
        #[case] width: u32,
        #[case] expected: ReaderPostCardType,
    ) {
        let fields = json!({
            "featured_image": "https://example.com/photo.jpg",
            "content": image("photo", width, 300),
        });
        assert_eq!(card_type(fields), expected);
    }

    #[rstest]
    #[case::too_much_text(json!({ "content": format!("{}<p>{}</p>", image("photo", 1000, 750), "no ".repeat(29)) }))]
    #[case::more_than_one_image(json!({ "content": format!("{}{}", image("photo", 1000, 750), image("other", 50, 50)) }))]
    #[case::no_image_in_the_content(json!({ "content": "<p>no no no</p>" }))]
    #[case::a_narrow_image(json!({ "featured_media": { "uri": "https://example.com/photo.jpg", "type": "image", "width": 439, "height": 300 } }))]
    #[case::no_width_known(json!({ "featured_media": null, "content": r#"<img src="https://example.com/photo.jpg"/>"# }))]
    #[case::no_image_to_show(json!({ "featured_image": "", "featured_media": null, "content": image("photo", 1000, 750) }))]
    fn test_card_type_is_not_a_photo(#[case] changes: serde_json::Value) {
        let mut fields = photo_fields();
        fields
            .as_object_mut()
            .unwrap()
            .extend(changes.as_object().unwrap().clone());
        assert_eq!(card_type(fields), ReaderPostCardType::Default);
    }

    /// 85 characters of text is a photo, and 86 isn't.
    #[rstest]
    #[case(85, ReaderPostCardType::Photo)]
    #[case(86, ReaderPostCardType::Default)]
    fn test_card_type_of_a_photo_by_the_length_of_its_text(
        #[case] chars: usize,
        #[case] expected: ReaderPostCardType,
    ) {
        let mut fields = photo_fields();
        fields["content"] = json!(format!(
            "{}<p>{}</p>",
            image("photo", 1000, 750),
            "é".repeat(chars)
        ));
        assert_eq!(card_type(fields), expected);
    }

    #[test]
    fn test_card_type_of_a_video() {
        let fields = json!({
            "featured_media": { "uri": "https://www.youtube.com/embed/abc", "type": "video" },
            "content": r#"<figure><iframe src="https://www.youtube.com/embed/abc"></iframe></figure><p>Watch this.</p>"#,
        });
        assert_eq!(card_type(fields), ReaderPostCardType::Video);
    }

    /// A post with a featured image shows it, and plays the video once it's open.
    #[test]
    fn test_card_type_of_a_video_with_a_featured_image() {
        let fields = json!({
            "featured_image": "https://example.com/featured.jpg",
            "featured_media": { "uri": "https://www.youtube.com/embed/abc", "type": "video" },
        });
        assert_eq!(card_type(fields), ReaderPostCardType::Default);
    }

    /// A post built by hand, without the `display_image_url` that parsing a response sets.
    #[test]
    fn test_card_type_of_a_post_built_without_its_display_image() {
        let mut post = post(photo_fields());
        post.display_image_url = None;
        assert_eq!(reader_post_card_type(&post), ReaderPostCardType::Photo);
    }

    #[rstest]
    #[case(ReaderPostCardType::Default, "default")]
    #[case(ReaderPostCardType::Photo, "photo")]
    #[case(ReaderPostCardType::Gallery, "gallery")]
    #[case(ReaderPostCardType::Video, "video")]
    #[case(ReaderPostCardType::CrossPost, "cross_post")]
    fn test_card_type_as_text(#[case] card_type: ReaderPostCardType, #[case] text: &str) {
        assert_eq!(card_type.to_string(), text);
        assert_eq!(text.parse::<ReaderPostCardType>().unwrap(), card_type);
        assert_eq!(serde_json::to_value(card_type).unwrap(), json!(text));
        assert_eq!(
            serde_json::from_value::<ReaderPostCardType>(json!(text)).unwrap(),
            card_type
        );
    }

    #[test]
    fn test_summary_without_a_byte_order_mark() {
        assert_eq!(summary_of_content("\u{feff}<p>Text.</p>"), "Text.");
        assert_eq!(summary_of_excerpt("\u{feff} Text."), "Text.");
    }

    fn text(html: &str) -> String {
        single_line(&html_text(html, Text::All, usize::MAX))
    }

    /// Markup the scanner reads the way a browser does.
    #[rstest]
    #[case::a_quoted_greater_than(r#"<a title="a > b" href="/x">Link</a> after"#, "Link after")]
    #[case::a_single_quoted_greater_than("<a title='a > b'>Link</a> after", "Link after")]
    #[case::an_apostrophe_in_an_unquoted_value("<a title=it's>Link</a> after", "Link after")]
    #[case::uppercase_tags("<P>One</P><P>Two<BR>Three</P>", "One Two Three")]
    #[case::a_comment_with_tags_in_it("One<!-- <p>not shown</p> --> two", "One two")]
    #[case::a_comment_that_never_ends("One <!-- two", "One")]
    #[case::a_doctype("<!DOCTYPE html><p>Text</p>", "Text")]
    #[case::cdata("One <![CDATA[ two ]]> three", "One three")]
    #[case::a_processing_instruction("<?xml version=\"1.0\"?><p>Text</p>", "Text")]
    #[case::a_script_with_a_less_than(
        "<script>if (a < b && c > d) { x = '<p>'; }</script>Text",
        "Text"
    )]
    #[case::an_uppercase_script("<SCRIPT>var a = '</p>';</SCRIPT>Text", "Text")]
    #[case::a_script_that_never_ends("Text<script>var a = 1;", "Text")]
    #[case::a_style("<style>p > a { color: red }</style>Text", "Text")]
    #[case::a_tag_that_never_ends("Text <a href=\"x", "Text")]
    #[case::a_less_than_that_is_text("1 < 2, 3 <4 and a <= b", "1 < 2, 3 <4 and a <= b")]
    #[case::a_less_than_at_the_end("a <", "a <")]
    #[case::an_escaped_tag("&lt;p&gt;Not a tag&lt;/p&gt;", "<p>Not a tag</p>")]
    #[case::a_self_closed_block("One<div/>Two", "One Two")]
    #[case::a_self_closed_frame("One<iframe src=\"x\"/>Two", "One Two")]
    #[case::a_frame("One<iframe src=\"x\">Not shown</iframe>Two", "One Two")]
    #[case::no_script("One<noscript>Turn on JavaScript</noscript> two", "One two")]
    #[case::a_template("One<template><p>Not shown</p></template> two", "One two")]
    #[case::a_shadow_root(
        r#"<div><template shadowrootmode="open"><p>Shown</p></template></div> too"#,
        "Shown too"
    )]
    #[case::an_icon("One <svg><title>Icon</title><path d=\"M0 0\"/></svg>two", "One two")]
    #[case::nested_hidden_elements("One<svg><svg>in</svg>still in</svg> two", "One two")]
    #[case::non_ascii(
        "<p lang=\"ja\" title=\"日本\">日本語</p><p>Ünïcödé</p>",
        "日本語 Ünïcödé"
    )]
    fn test_html_text(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(text(html), expected);
    }

    #[test]
    fn test_html_text_up_to_a_length() {
        assert_eq!(
            html_text("<p>One two</p><p>three</p>", Text::All, 5),
            " One "
        );
        // Not cut inside a character.
        assert_eq!(html_text("日本語", Text::All, 4), "日");
    }

    #[rstest]
    #[case::double_quoted(r#"<img src="https://example.com/a.jpg" width="640" height="480">"#)]
    #[case::single_quoted("<img src='https://example.com/a.jpg' width='640' height='480'>")]
    #[case::unquoted("<img src=https://example.com/a.jpg width=640 height=480>")]
    #[case::self_closed(r#"<img src="https://example.com/a.jpg" width="640" height="480"/>"#)]
    #[case::uppercase(r#"<IMG SRC="https://example.com/a.jpg" WIDTH="640" HEIGHT="480">"#)]
    #[case::spaces_around_equals(
        r#"<img src = "https://example.com/a.jpg" width = "640" height = "480">"#
    )]
    #[case::on_several_lines(
        "<img\n\tsrc=\"https://example.com/a.jpg\"\n\twidth=\"640\"\n\theight=\"480\"\n/>"
    )]
    #[case::other_attributes_first(r#"<img loading="lazy" alt="a > b" data-src="no" src="https://example.com/a.jpg" width="640" height="480">"#)]
    #[case::the_first_of_two(r#"<img src="https://example.com/a.jpg" src="https://example.com/b.jpg" width="640" height="480">"#)]
    #[case::in_a_noscript(
        r#"<noscript><img src="https://example.com/a.jpg" width="640" height="480"></noscript>"#
    )]
    fn test_content_image(#[case] html: &str) {
        let images = content_images(html);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].src, "https://example.com/a.jpg");
        assert_eq!(images[0].size, Some((640, 480)));
    }

    #[rstest]
    #[case::no_src(r#"<img alt="none">"#)]
    #[case::an_empty_src(r#"<img src="">"#)]
    #[case::a_data_image(r#"<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=">"#)]
    #[case::an_avatar(r#"<img src="https://1.gravatar.com/avatar/abc?s=96">"#)]
    #[case::a_smiley(r#"<img src="https://s0.wp.com/wp-content/mu-plugins/wpcom-smileys/twemoji/2/72x72/1f642.png">"#)]
    #[case::a_tracking_pixel(
        r#"<img src="https://pixel.wp.com/b.gif?host=example.com" width="1" height="1">"#
    )]
    #[case::a_tracking_pixel_with_no_height(
        r#"<img src="https://example.com/t.gif" width="1" height="0">"#
    )]
    #[case::in_a_script(
        r#"<script>document.write('<img src="https://example.com/a.jpg">');</script>"#
    )]
    #[case::in_a_comment(r#"<!-- <img src="https://example.com/a.jpg"> -->"#)]
    #[case::not_an_image_tag(r#"<imgur src="https://example.com/a.jpg">"#)]
    fn test_not_a_content_image(#[case] html: &str) {
        assert!(content_images(html).is_empty());
    }

    #[test]
    fn test_content_image_url_is_decoded() {
        let images =
            content_images(r#"<img src="https://example.com/a.jpg?w=640&#038;h=480&amp;crop=1">"#);
        assert_eq!(
            images[0].src,
            "https://example.com/a.jpg?w=640&h=480&crop=1"
        );
        assert_eq!(images[0].size, None);
    }

    /// Markup that would take a parser seconds takes this no longer than anything else its size.
    #[rstest]
    #[case::nested_blocks("<div>", "</div>")]
    #[case::nested_and_never_closed("<blockquote>", "")]
    #[case::nested_hidden_elements("<figure>", "</figure>")]
    #[case::open_brackets("[", "")]
    #[case::bare_shortcode_tags("[a]", "")]
    #[case::less_thans("<", "")]
    #[case::unclosed_tags("<a b=\"", "")]
    fn test_markup_that_repeats_is_read_quickly(#[case] open: &str, #[case] close: &str) {
        let content = format!("{}Deep{}", open.repeat(200_000), close.repeat(200_000));
        let post = post(json!({ "content": content }));
        let started = std::time::Instant::now();
        reader_post_summary(&post);
        reader_post_reading_time_minutes(&post);
        reader_post_card_type(&post);
        // A parser takes minutes over the first of these. This takes milliseconds: the limit is
        // only there to fail if the time ever grows with the square of the size again.
        assert!(started.elapsed() < std::time::Duration::from_secs(10));
    }
}
