//! Which gallery images open in the app's image viewer when tapped, even though they're links.
//!
//! Gallery images often link to their attachment page: a page on the site showing just the image.
//! The apps open those in the image viewer instead, so the reader can page through the gallery.
//! Links to the image file already open in the viewer.
//!
//! Most galleries can only link an image to its attachment page or its file: Jetpack's tiled
//! galleries, the classic `[gallery]`, and the Gallery block before WordPress 5.9. In the newer
//! Gallery block, each image is an Image block, which can link anywhere. Which one the author
//! chose is in the block's `linkDestination`, but the Reader API only sends the rendered HTML
//! (READ-874), so this goes by the link instead:
//!
//! - Jetpack Carousel gives each image its attachment page, in `data-permalink`. When it's there,
//!   only a link to that page counts.
//! - `?attachment_id=<id>` is the attachment page of a site without pretty permalinks.
//! - Otherwise, an attachment page's address ends in a name made from the file's, or follows the
//!   post's address. Of 948 linked images in newer Gallery blocks in 284k Reader posts, this left
//!   7 attachment pages as links, and none of the custom links became images.

use scraper::{ElementRef, Html, Node, Selector};
use std::sync::LazyLock;
use url::Url;

use super::anchors::percent_decode;
use super::clean::set_attribute;
use super::content::GALLERIES;

/// The attribute `reader.js` looks for on a link, to open its image in the image viewer.
const VIEWER_LINK_ATTRIBUTE: &str = "data-reader-image";

static LINKED_IMAGES: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a[href] img").expect("the selector is valid"));

/// Marks the links on gallery images that go to the image's attachment page, so `reader.js`
/// opens the image viewer instead. `post_url` is the post's address.
pub(super) fn mark_gallery_image_links(document: &mut Html, post_url: Option<&Url>) {
    let links: Vec<_> = document
        .root_element()
        .select(&LINKED_IMAGES)
        .filter_map(|img| {
            let link = img
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|ancestor| ancestor.value().name() == "a")?;
            let gallery = img
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|ancestor| GALLERIES.matches(ancestor))?;
            let href = link.value().attr("href")?;
            let opens =
                !allows_custom_links(img, gallery) || links_to_attachment_page(href, img, post_url);
            opens.then_some(link.id())
        })
        .collect();

    for id in links {
        if let Some(mut node) = document.tree.get_mut(id)
            && let Node::Element(element) = node.value()
        {
            set_attribute(element, VIEWER_LINK_ATTRIBUTE, "");
        }
    }
}

/// Whether the gallery can link an image to any address: the Gallery block since WordPress 5.9,
/// where each image is an Image block.
fn allows_custom_links(img: ElementRef, gallery: ElementRef) -> bool {
    img.ancestors()
        .filter_map(ElementRef::wrap)
        .take_while(|ancestor| ancestor.id() != gallery.id())
        .any(|ancestor| {
            let element = ancestor.value();
            element.name() == "figure" && element.classes().any(|class| class == "wp-block-image")
        })
}

fn links_to_attachment_page(href: &str, img: ElementRef, post_url: Option<&Url>) -> bool {
    let Some(link) = parse(href, post_url) else {
        return false;
    };
    let element = img.value();

    let ids: Vec<&str> = element
        .attr("data-attachment-id")
        .into_iter()
        .chain(element.attr("data-id"))
        .chain(
            element
                .classes()
                .filter_map(|class| class.strip_prefix("wp-image-")),
        )
        .collect();
    if link
        .query_pairs()
        .any(|(name, value)| name == "attachment_id" && ids.contains(&value.as_ref()))
    {
        return true;
    }

    if let Some(permalink) = element.attr("data-permalink") {
        return parse(permalink, post_url).is_some_and(|permalink| same_page(&link, &permalink));
    }

    let Some(name) = link.path_segments().and_then(|mut segments| {
        segments
            .rfind(|segment| !segment.is_empty())
            .map(|segment| percent_decode(segment).to_lowercase())
    }) else {
        return false;
    };
    let named_after_the_file = [element.attr("data-orig-file"), element.attr("src")]
        .into_iter()
        .flatten()
        .filter_map(|url| file_slug(url, post_url))
        .any(|file| is_named_after(&name, &file));
    named_after_the_file || follows_the_post(&link, post_url)
}

fn parse(url: &str, post_url: Option<&Url>) -> Option<Url> {
    match post_url {
        Some(post_url) => post_url.join(url).ok(),
        None => Url::parse(url).ok(),
    }
}

/// Whether two addresses are the same page, whatever their scheme, `www.` or trailing slash. Sites
/// on their own domain often give their attachment pages as `http:`.
fn same_page(a: &Url, b: &Url) -> bool {
    fn key(url: &Url) -> Option<(String, &str)> {
        let host = url.host_str()?.to_ascii_lowercase();
        let host = host.strip_prefix("www.").map(str::to_owned).unwrap_or(host);
        Some((host, url.path().trim_end_matches('/')))
    }
    key(a).is_some_and(|key_a| Some(key_a) == key(b))
}

/// The name WordPress gives an image's attachment page, from its file: `IMG_3495.jpeg` is
/// `img_3495`. Smaller copies (`-1024x768`), large originals (`-scaled`) and edited images
/// (`-e1371913901440`) have the same attachment page.
fn file_slug(url: &str, post_url: Option<&Url>) -> Option<String> {
    let url = parse(url, post_url)?;
    let file = percent_decode(url.path_segments()?.next_back()?);
    let (stem, _) = file.rsplit_once('.')?;
    let stem = stem.strip_suffix("-scaled").unwrap_or(stem);
    let stem = strip_size(stem).unwrap_or(stem);
    let stem = match stem.rsplit_once("-e") {
        Some((stem, edit)) if edit.len() >= 10 && edit.bytes().all(|b| b.is_ascii_digit()) => stem,
        _ => stem,
    };
    let slug = slugify(stem);
    (!slug.is_empty()).then_some(slug)
}

/// `photo` from `photo-1024x768`.
fn strip_size(stem: &str) -> Option<&str> {
    let (stem, size) = stem.rsplit_once('-')?;
    let (width, height) = size.split_once('x')?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    (digits(width) && digits(height)).then_some(stem)
}

/// Roughly what `sanitize_title()` does to a file name: lowercase, with runs of anything that isn't
/// a letter, a digit, `_` or `-` as one `-`.
fn slugify(text: &str) -> String {
    let mut slug = String::new();
    for character in text.to_lowercase().chars() {
        if character.is_alphanumeric() || character == '_' || character == '-' {
            slug.push(character);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_owned()
}

/// Whether an attachment page's name is the file's slug. WordPress adds `-2` when the name is
/// taken, and some importers add the extension, as `-jpg`.
fn is_named_after(name: &str, file: &str) -> bool {
    let strip_number = |name: &str| -> String {
        match name.rsplit_once('-') {
            Some((rest, number))
                if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) =>
            {
                rest.to_owned()
            }
            _ => name.to_owned(),
        }
    };
    let strip_extension = |name: &str| -> String {
        ["-jpg", "-jpeg", "-png", "-gif", "-webp", "-heic"]
            .iter()
            .find_map(|extension| name.strip_suffix(extension))
            .unwrap_or(name)
            .to_owned()
    };
    let without_number = strip_number(name);
    let without_extension = strip_extension(&without_number);
    [
        name.to_owned(),
        without_number.clone(),
        without_extension.clone(),
        strip_number(&without_extension),
    ]
    .iter()
    .any(|candidate| candidate == file)
}

/// Whether the link is the post's address with one more name on the end, the way WordPress
/// addresses the attachment pages of a post's images.
fn follows_the_post(link: &Url, post_url: Option<&Url>) -> bool {
    let Some(post_url) = post_url else {
        return false;
    };
    let segments = |url: &Url| -> Vec<String> {
        url.path_segments()
            .map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (link_segments, post_segments) = (segments(link), segments(post_url));
    let same_site = same_page(
        &link.join("/").unwrap_or_else(|_| link.clone()),
        &post_url.join("/").unwrap_or_else(|_| post_url.clone()),
    );
    same_site
        && !post_segments.is_empty()
        && link_segments.len() == post_segments.len() + 1
        && link_segments.starts_with(&post_segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{ReaderPostContent, reader_script};
    use rstest::rstest;
    use serde_json::Value;

    const POST: &str = "https://example.com/2023/03/12/a-post/";

    fn opens_in_the_viewer(html: &str) -> bool {
        let mut document = Html::parse_fragment(html);
        let post_url = Url::parse(POST).unwrap();
        mark_gallery_image_links(&mut document, Some(&post_url));
        let link = Selector::parse("a").unwrap();
        let link = document.root_element().select(&link).next().unwrap();
        link.value().attr(VIEWER_LINK_ATTRIBUTE).is_some()
    }

    /// An image in a newer Gallery block.
    fn block(link: &str, img_attributes: &str) -> String {
        format!(
            r#"<figure class="wp-block-gallery has-nested-images"><figure class="wp-block-image"><a href="{link}"><img {img_attributes}></a></figure></figure>"#
        )
    }

    #[rstest]
    #[case::tiled_gallery(
        r#"<div class="tiled-gallery"><a href="https://example.com/2023/03/12/a-post/renamed/"><img src="https://example.com/a.jpg"></a></div>"#,
        true
    )]
    #[case::classic_gallery(
        r#"<div class="gallery"><dl><dt><a href="https://example.com/other/"><img src="https://example.com/a.jpg"></a></dt></dl></div>"#,
        true
    )]
    #[case::gallery_block_before_5_9(
        r#"<ul class="wp-block-gallery"><li class="blocks-gallery-item"><figure><a href="https://example.com/x/"><img src="https://example.com/a.jpg"></a></figure></li></ul>"#,
        true
    )]
    #[case::outside_a_gallery(
        r#"<figure class="wp-block-image"><a href="https://example.com/2023/03/12/a-post/a/"><img src="https://example.com/a.jpg"></a></figure>"#,
        false
    )]
    fn marks_links_by_gallery(#[case] html: &str, #[case] opens: bool) {
        assert_eq!(opens_in_the_viewer(html), opens);
    }

    #[rstest]
    #[case::carousel_permalink(
        "http://example.com/2023/03/12/a-post/renamed",
        r#"src="https://example.com/a.jpg" data-permalink="https://example.com/2023/03/12/a-post/renamed/""#,
        true
    )]
    #[case::carousel_permalink_isnt_the_link(
        "https://wordpress.com/theme/marl",
        r#"src="https://example.files.wordpress.com/2022/03/marl.jpg" data-permalink="https://example.com/2022/03/09/themes/marl/""#,
        false
    )]
    #[case::carousel_permalink_wins_over_the_name(
        "https://example.com/elsewhere/marl/",
        r#"src="https://example.com/marl.jpg" data-permalink="https://example.com/2022/03/09/themes/marl/""#,
        false
    )]
    #[case::attachment_id(
        "https://example.com/?attachment_id=7",
        r#"src="https://example.com/a.jpg" class="wp-image-7""#,
        true
    )]
    #[case::another_attachment_id(
        "https://example.com/?attachment_id=8",
        r#"src="https://example.com/a.jpg" class="wp-image-7""#,
        false
    )]
    #[case::named_after_the_file(
        "https://example.com/dsc_2150/",
        r#"src="https://example.com/wp-content/uploads/2017/04/DSC_2150-1024x683.jpg""#,
        true
    )]
    #[case::named_after_the_file_with_a_number(
        "https://example.com/img_9118-2/",
        r#"src="https://example.com/wp-content/uploads/IMG_9118-scaled.jpg""#,
        true
    )]
    #[case::named_after_the_file_with_its_extension(
        "https://example.com/20130225-172522-jpg/",
        r#"src="https://example.com/wp-content/uploads/20130225-172522.jpg?w=150""#,
        true
    )]
    #[case::named_after_an_edited_file(
        "https://example.com/img_2984/",
        r#"src="https://example.com/wp-content/uploads/img_2984-e1371913901440.jpg""#,
        true
    )]
    #[case::named_after_the_original(
        "https://example.com/0072-clouds-copy/",
        r#"src="https://cdn.example.com/resized.jpg" data-orig-file="https://example.com/0072-clouds-copy.jpg""#,
        true
    )]
    #[case::after_the_post(
        "https://example.com/2023/03/12/a-post/cerealmag_1/",
        r#"src="https://example.com/wp-content/uploads/cereal-1.jpg""#,
        true
    )]
    #[case::after_another_post(
        "https://example.com/2023/03/13/another-post/cerealmag_1/",
        r#"src="https://example.com/wp-content/uploads/cereal-1.jpg""#,
        false
    )]
    #[case::custom_link(
        "https://www.teachappy.co.uk/post/creating-healthy-habits",
        r#"src="https://example.com/wp-content/uploads/TH2.jpg""#,
        false
    )]
    #[case::custom_link_on_the_site(
        "https://example.com/shop/",
        r#"src="https://example.com/wp-content/uploads/a.jpg""#,
        false
    )]
    fn tells_attachment_pages_from_other_links_in_gallery_blocks(
        #[case] link: &str,
        #[case] img_attributes: &str,
        #[case] opens: bool,
    ) {
        assert_eq!(opens_in_the_viewer(&block(link, img_attributes)), opens);
    }

    /// How many of the post's linked images open in the image viewer, of how many.
    fn marked_links(content: &ReaderPostContent) -> (usize, usize) {
        let document = Html::parse_fragment(&content.html());
        let links: Vec<bool> = document
            .root_element()
            .select(&LINKED_IMAGES)
            .filter_map(|img| {
                img.ancestors()
                    .filter_map(ElementRef::wrap)
                    .find(|ancestor| ancestor.value().name() == "a")
            })
            .map(|link| link.value().attr(VIEWER_LINK_ATTRIBUTE).is_some())
            .collect();
        (links.iter().filter(|marked| **marked).count(), links.len())
    }

    /// Real posts from en.blog.wordpress.com, as the Reader API sends them:
    /// `/rest/v1.2/read/sites/3584907/posts/<id>`, keeping `URL` and `content`.
    #[rstest]
    // A Jetpack tiled gallery, its images linked to their attachment pages.
    #[case::tiled_gallery("12152", (27, 27))]
    // Gallery blocks whose images link to themes on WordPress.com, with Carousel's
    // `data-permalink`. The source has `"linkDestination":"custom"` on all 42.
    #[case::gallery_blocks_with_custom_links("45746", (0, 42))]
    fn marks_real_posts(#[case] id: &str, #[case] expected: (usize, usize)) {
        let path = format!(
            "{}/test-data/reader/gallery-links/{id}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let post: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let content = ReaderPostContent::new(
            post["content"].as_str().unwrap(),
            post["URL"].as_str().map(str::to_owned),
        );
        assert_eq!(marked_links(&content), expected);
    }

    #[rstest]
    // Without the post's address, a link can't follow it, but the other checks still work.
    #[case::tiled_gallery_without_the_post(
        r#"<div class="tiled-gallery"><a href="https://example.com/x/"><img src="https://example.com/a.jpg"></a></div>"#,
        None,
        true
    )]
    #[case::named_after_the_file_without_the_post(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><a href="https://example.com/dsc_2150/"><img src="https://example.com/DSC_2150.jpg"></a></figure></figure>"#,
        None,
        true
    )]
    #[case::after_the_post_without_the_post(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><a href="https://example.com/2023/03/12/a-post/cerealmag_1/"><img src="https://example.com/cereal-1.jpg"></a></figure></figure>"#,
        None,
        false
    )]
    // Relative links are resolved against the post first.
    #[case::relative_link_after_the_post(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><a href="cerealmag_1/"><img src="/wp-content/uploads/cereal-1.jpg"></a></figure></figure>"#,
        Some(POST),
        true
    )]
    #[case::relative_link_named_after_the_file(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><a href="/dsc_2150/"><img src="/wp-content/uploads/DSC_2150.jpg"></a></figure></figure>"#,
        Some(POST),
        true
    )]
    #[case::relative_custom_link(
        r#"<figure class="wp-block-gallery"><figure class="wp-block-image"><a href="/shop/"><img src="/wp-content/uploads/a.jpg"></a></figure></figure>"#,
        Some(POST),
        false
    )]
    fn marks_links_in_posts(#[case] html: &str, #[case] url: Option<&str>, #[case] opens: bool) {
        let content = ReaderPostContent::new(html, url.map(str::to_owned));
        assert_eq!(marked_links(&content), (usize::from(opens), 1));
    }

    #[test]
    fn photon_sizes_keep_the_mark() {
        let content = ReaderPostContent::new(
            r#"<div class="tiled-gallery"><a href="https://example.com/x/"><img src="https://example.com/a.jpg" width="1200"></a></div>"#,
            Some(POST.to_owned()),
        );
        assert_eq!(marked_links(&content.with_photon_sizes()), (1, 1));
    }

    /// `reader.js` reads the attribute as `dataset.readerImage`.
    #[test]
    fn the_script_opens_marked_links_in_the_viewer() {
        assert_eq!(VIEWER_LINK_ATTRIBUTE, "data-reader-image");
        assert!(reader_script().contains(r#""readerImage" in link.dataset"#));
    }
}
