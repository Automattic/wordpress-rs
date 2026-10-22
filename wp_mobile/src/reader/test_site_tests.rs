//! Checks what [`ReaderPostContent`] makes of our own test posts, as the Reader API sends them.
//!
//! Most of the posts are on https://jetpack.wpmt.co, and `test-data/reader/capture-test-site-posts.sh`
//! captures them into `test-data/reader/test-site`. The snapshot tests cover all of their output;
//! these tests name the behaviours each post is there for. The Reader API reads this site's posts
//! from its feed, so it removes their scripts, and writes in-page links with the post's address.
//! The Getty posts are on https://wpmt.co, a WordPress.com site, whose posts keep their scripts.

use std::{path::Path, sync::Arc};

use super::corpus_tests::{Post, post};
use super::{ReaderPostAnchor, ReaderPostAnchorKind, ReaderPostContent};

fn test_post(name: &str) -> (Post, Arc<ReaderPostContent>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("test-data/reader/test-site")
        .join(format!("{name}.json"));
    let post = post(&path);
    let content = post.content();
    (post, content)
}

/// Whether one of the post's scripts comes from any of `hosts`. A provider can serve its script from
/// more than one host, and the post's own script is used when the Reader API leaves it in.
fn loads_a_script_from(content: &ReaderPostContent, hosts: &[&str]) -> bool {
    content.scripts().iter().any(|script| {
        url::Url::parse(script)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .is_some_and(|host| hosts.contains(&host.as_str()))
    })
}

fn anchor(content: &ReaderPostContent, id: &str) -> ReaderPostAnchor {
    content
        .anchors()
        .into_iter()
        .find(|anchor| anchor.id == id)
        .unwrap_or_else(|| panic!("there's an anchor for #{id}"))
}

// Shortcodes

#[test]
fn hides_the_shortcodes_of_plugins_wordpress_com_cant_run() {
    let (_, content) = test_post("shortcodes-from-plugins");
    let text = content.plain_text();
    assert!(!text.contains('['), "a shortcode is left in:\n{text}");
    // What they enclose stays.
    for kept in [
        "WPBakery: text in a column.",
        "Divi: text in a section.",
        "Shortcodes Ultimate: a drop cap starts this sentence.",
        "The infobox’s text.",
        "Date: Oct 21 2026 19:00 UTC",
        "The reference.",
    ] {
        assert!(text.contains(kept), "{kept:?} isn't in:\n{text}");
    }
}

#[test]
fn hides_shortcodes_in_blocks() {
    let (_, content) = test_post("shortcodes-in-blocks");
    let text = content.plain_text();
    assert!(!text.contains('['), "a shortcode is left in:\n{text}");
    assert!(text.contains("A drop cap in a paragraph block."));
    assert!(text.contains("WPBakery in a Shortcode block."));
}

#[test]
fn hides_shortcodes_the_site_leaves_as_text() {
    let (_, content) = test_post("shortcodes-run-by-the-site-without-shortcode-embeds");
    let text = content.plain_text();
    assert!(!text.contains("[soundcloud"));
    assert!(!text.contains("[code"));
    assert!(text.contains("SELECT 1;"));
    // WordPress made a link of the tweet's URL with the `]` in it, as `%5D`, so the shortcode isn't
    // whole in the text, and stays.
    assert!(text.contains("[tweet https://twitter.com/jack/status/20%5D"));
}

#[test]
fn runs_the_embeds_jetpack_shortcodes_make() {
    let (_, content) = test_post("shortcodes-run-by-the-site");
    assert_eq!(
        content.scripts(),
        ["https://platform.twitter.com/widgets.js"]
    );
    let html = content.html();
    assert!(html.contains("https://w.soundcloud.com/player/"));
    // The Reader API leaves out the YouTube shortcode's wrapper, so the cleanup sizes the iframe.
    assert!(html.contains("aspect-ratio: 640 / 360;"));
    assert!(!content.plain_text().contains("[code"));
}

#[test]
fn leaves_brackets_that_arent_shortcodes() {
    let (_, content) = test_post("brackets-that-arent-shortcodes");
    let text = content.plain_text();
    for prose in [
        "[sic]",
        "[Laughter]",
        "[emphasis mine]",
        "[1]",
        "[REDACTED]",
        "[Facebook group]",
        "[crossed out letters]",
        "[TL;DR]",
        "[MCN-Announce]",
        "[is being]",
        "[x = 5]",
        "[a, b]",
    ] {
        assert!(text.contains(prose), "{prose} isn't in:\n{text}");
    }

    // Shortcodes in code are left as they are.
    let html = content.html();
    assert!(html.contains(r#"<code>[gallery ids="1,2"]</code>"#));
    assert!(html.contains("<code>[[time]Oct 21 2026[/time]]</code>"));
    assert!(html.contains("[[vc_row]][[vc_column]]Text[[/vc_column]][[/vc_row]]"));

    // An escaped shortcode in prose, which WordPress shows as `[gallery ids="1,2"]`, looks just like
    // one the Reader API didn't run, so it's hidden too. Posts about shortcodes put them in code.
    assert!(
        html.contains(
            "Escaped shortcodes in prose, which WordPress shows as text:  and A caption."
        )
    );
}

#[test]
fn shows_core_shortcodes_the_feed_ran() {
    let (_, content) = test_post("shortcodes-left-for-wordpress-com");
    assert!(!content.plain_text().contains('['));
    // Three galleries of three, in WordPress's form for feeds: thumbnails, each linked.
    assert_eq!(content.images().len(), 9);
}

// Embeds

#[test]
fn loads_a_script_for_every_embed_that_needs_one() {
    // The Reader API removes scripts from posts it reads from a feed, so these come from the
    // cleanup's list of known embeds. If it stops removing them, the post's own are used instead.
    let (_, content) = test_post("embeds");
    for hosts in [
        &["www.instagram.com", "platform.instagram.com"][..],
        &["platform.twitter.com", "platform.x.com"],
        &["embed.reddit.com", "embed.redditmedia.com"],
        &["embed.bsky.app"],
    ] {
        assert!(
            loads_a_script_from(&content, hosts),
            "no script from {hosts:?}"
        );
    }

    let (_, content) = test_post("jetpack-shortcode-embeds");
    for hosts in [
        &["www.instagram.com", "platform.instagram.com"][..],
        &["connect.facebook.net"],
        &["assets.pinterest.com"],
    ] {
        assert!(
            loads_a_script_from(&content, hosts),
            "no script from {hosts:?}"
        );
    }
}

#[test]
fn lets_the_page_load_gists_from_github() {
    // The gist from the shortcode, and the one from a URL on its own line, whose HTML `wpautop`
    // broke on the site.
    let (_, content) = test_post("jetpack-shortcode-embeds");
    let data = "https://gist.github.com/octocat/6cad326836d38bd3a7ae.json?file=hello_world.rb";
    let html = content.html();
    assert_eq!(
        html.matches(&format!(
            r#"<figure class="reader-gist" data-reader-gist="{data}"><pre><code>class HelloWorld
   def initialize(name)
      @name = name.capitalize
"#
        ))
        .count(),
        2,
        "{html}"
    );
    assert!(
        !content
            .plain_text()
            .contains("hidden or bidirectional Unicode")
    );

    let page = |offline| {
        content.page_html(super::ReaderPageOptions {
            theme: super::ReaderTheme::Sepia,
            app_colors: super::reader_default_app_colors(super::ReaderBrand::Wordpress),
            language: None,
            offline,
            font: None,
            text_scale: 1.0,
            server_css: None,
            css: None,
        })
    };
    // The shared script loads the gists' data from GitHub.
    assert!(page(false).contains(" https://gist.github.com "));
    // Offline, the code blocks stay.
    assert!(!page(true).contains("gist.github.com "));
    assert!(!content.offline_html().contains("data-reader-gist"));
}

#[test]
fn keeps_what_the_script_needs_for_wordpress_post_embeds() {
    let (_, content) = test_post("embeds");
    let html = content.html();
    // The script hides the blockquote and sizes the iframe, matching them by their secret.
    assert!(html.contains(r#"<blockquote class="wp-embedded-content" data-secret=""#));
    assert!(html.contains(r#"<iframe class="wp-embedded-content" data-secret=""#));
}

#[test]
fn links_embeds_that_never_loaded() {
    let (_, content) = test_post("embeds");
    let html = content.html();
    assert!(html.contains(r#"<a href="https://www.threads.net/@zuck/post/CuP48CiS5sx""#));
    assert!(html.contains(r#"<a href="https://www.flickr.com/photos/nasahqphoto/53638095797/""#));
}

#[test]
fn sizes_iframes_unless_their_wrapper_does() {
    let (_, content) = test_post("jetpack-shortcode-embeds");
    let html = content.html();
    // Dailymotion's shortcode is a bare iframe.
    assert!(html.contains("aspect-ratio: 425 / 334;"));
    // Vimeo's is in a wrapper that keeps its shape.
    assert!(!html.contains("aspect-ratio: 600 / 338;"));
}

#[test]
fn builds_getty_embeds_from_their_inline_script() {
    for name in ["getty-block", "getty-code", "getty-url"] {
        let (_, content) = test_post(name);
        let html = content.html();
        assert!(
            html.contains(r#"src="https://embed.gettyimages.com/embed/1234567?et="#),
            "{name}: {html}"
        );
        assert!(html.contains("aspect-ratio: 394 / 594;"), "{name}: {html}");
        assert!(
            !loads_a_script_from(&content, &["embed-cdn.gettyimages.com"]),
            "{name}"
        );
    }
}

// Blocks

#[test]
fn finds_headings_and_the_footnotes_block() {
    let (_, content) = test_post("core-blocks");
    assert_eq!(
        anchor(&content, "heading-3").kind,
        ReaderPostAnchorKind::Heading { level: 3 }
    );
    let footnote = anchor(&content, "8d4a0f6e-6c47-4d8a-9a3e-0c5f1b2e7a10");
    assert_eq!(footnote.kind, ReaderPostAnchorKind::Footnote);
    assert_eq!(
        footnote.text.as_deref(),
        Some("The first footnote, with a link.")
    );
}

#[test]
fn makes_links_within_the_post_in_page_links() {
    for name in ["core-blocks", "languages-and-anchors"] {
        let (post, content) = test_post(name);
        let html = content.html();
        assert!(
            !html.contains(&format!(r#"href="{}#"#, post.url)),
            "{name} still links to itself by its address"
        );
    }
    let (_, content) = test_post("core-blocks");
    assert!(
        content
            .html()
            .contains(r##"<a href="#8d4a0f6e-6c47-4d8a-9a3e-0c5f1b2e7a10" id="##)
    );
}

#[test]
fn finds_the_images_in_galleries_slideshows_and_tiled_galleries() {
    let (_, content) = test_post("core-blocks");
    let galleries = content.galleries();
    assert_eq!(galleries.len(), 1);
    assert_eq!(galleries[0].images.len(), 3);

    let (_, content) = test_post("jetpack-blocks");
    let sizes: Vec<usize> = content
        .galleries()
        .iter()
        .map(|gallery| gallery.images.len())
        .collect();
    // The slideshow, then the tiled gallery.
    assert_eq!(sizes, [3, 2]);
}

#[test]
fn keeps_tiled_gallery_column_widths() {
    let (_, content) = test_post("jetpack-blocks");
    let html = content.html();
    assert_eq!(html.matches(r#"style="flex-grow: 50;""#).count(), 2);
}

// Images

#[test]
fn loads_every_image_over_https() {
    let (_, content) = test_post("images");
    for image in content.images() {
        assert!(
            image.url.starts_with("https://") || image.url.starts_with("data:"),
            "{} isn't on https",
            image.url
        );
    }
}

#[test]
fn loads_lazily_loaded_images() {
    let (_, content) = test_post("images");
    assert!(
        content
            .images()
            .iter()
            .any(|image| image.url.ends_with("/cmm2548-42.png"))
    );
}

#[test]
fn shows_emoji_as_text() {
    let (_, content) = test_post("images");
    let text = content.plain_text();
    // Feeds turn emoji, and smileys like `:)`, into images.
    for emoji in ["😀", "🎉", "👍🏽", "🇨🇦", "🙂", "😉"] {
        assert!(text.contains(emoji), "{emoji} isn't in the text");
    }
    assert!(!content.html().contains("wp-smiley"));
}

#[test]
fn offers_photon_sizes_of_heic_images() {
    let (_, content) = test_post("images");
    let resized = content.with_photon_sizes();
    let heic = resized
        .images()
        .into_iter()
        .find(|image| image.url.contains("reader-test-heic.heic"))
        .expect("the post has the original HEIC");
    assert!(!heic.sizes.is_empty());
}

#[test]
fn knows_whether_the_featured_image_is_in_the_post() {
    let (post, content) = test_post("images");
    assert!(content.includes_image(&post.featured_image));
    let (post, content) = test_post("languages-and-anchors");
    assert!(!content.includes_image(&post.featured_image));
}

// Text and anchors

#[test]
fn finds_anchors_in_the_post() {
    let (_, content) = test_post("languages-and-anchors");
    assert_eq!(
        anchor(&content, "café").kind,
        ReaderPostAnchorKind::Heading { level: 2 }
    );
    assert_eq!(
        anchor(&content, "old-style-anchor").kind,
        ReaderPostAnchorKind::Other
    );
    assert_eq!(
        anchor(&content, "fn-reader-2").text.as_deref(),
        Some("The second note, with a link.")
    );
    assert!(
        !content
            .anchors()
            .iter()
            .any(|anchor| anchor.id == "nowhere")
    );
}

#[test]
fn keeps_the_direction_of_right_to_left_text() {
    let (_, content) = test_post("languages-and-anchors");
    assert!(content.html().contains(r#"dir="rtl" lang="ar""#));
}

#[test]
fn knows_a_manual_excerpt_from_one_wordpress_made() {
    let (post, content) = test_post("languages-and-anchors");
    assert!(content.is_manual_excerpt(&post.excerpt));
    let (post, content) = test_post("core-blocks");
    assert!(!content.is_manual_excerpt(&post.excerpt));
}

// Cleanup

#[test]
fn cleans_up_what_the_post_says_it_should() {
    let (_, content) = test_post("cleanup");
    let html = content.html();
    // Only an embed's script would load, and the post has no embeds that need one: the X script
    // it includes is for nothing.
    assert!(
        content
            .scripts()
            .iter()
            .all(|script| script.starts_with("https://platform.twitter.com/"))
    );
    assert!(html.contains("aspect-ratio: 640 / 360;"));
    assert!(html.contains("aspect-ratio: 300 / 250;"));
    assert!(html.contains(r#"<iframe height="152""#));
    assert!(!html.contains("autoplay"));
    assert!(
        html.contains(
            r#"<video class="wp-block-cover__video-background intrinsic-ignore" controls"#
        )
    );
    assert!(html.contains(r#"<a href="https://example.com/an-embed-that-never-loaded""#));
}
