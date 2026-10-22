//! Runs every post in a corpus of real Reader API responses through [`ReaderPostContent`].
//!
//! The corpus is real people's posts, so it isn't in this repository, and this test is ignored
//! unless you point it at one:
//!
//! ```sh
//! READER_CORPUS=path/to/corpus cargo test -p wp_mobile --lib reader::corpus -- --ignored --nocapture
//! ```
//!
//! Set `READER_CORPUS_OUT` to a directory to also write each cleaned post there, for reading or for
//! diffing between changes. Set `READER_CORPUS_SUMMARY` to a file to write one line per post of
//! what `ReaderPostContent` found in it.
//!
//! Each post is a JSON file with the Reader API's fields in `fields`, and the features an
//! independent detector found in `features`. The test fails if the cleanup leaves something it
//! should have removed, and prints where `ReaderPostContent` disagrees with the detector.

use std::{
    collections::BTreeMap,
    env, fs,
    path::Path,
    time::{Duration, Instant},
};

use scraper::{ElementRef, Html, Node};
use serde_json::Value;

use super::images::image_key;
use super::{ReaderLink, ReaderPostAnchorKind, ReaderPostContent, reader_link};

pub(super) struct Post {
    pub(super) id: String,
    pub(super) url: String,
    pub(super) content: String,
    pub(super) excerpt: String,
    pub(super) featured_image: String,
    pub(super) features: Vec<String>,
}

impl Post {
    /// The post, the way an app would make it: with its URL, for relative links and images.
    pub(super) fn content(&self) -> std::sync::Arc<ReaderPostContent> {
        let url = (!self.url.is_empty()).then(|| self.url.clone());
        ReaderPostContent::new(&self.content, url)
    }
}

/// The posts in a directory of JSON files, each with the Reader API's fields in `fields`, and the
/// features a detector found in `features`.
pub(super) fn posts(corpus: &Path) -> Vec<Post> {
    let mut paths: Vec<_> = fs::read_dir(corpus)
        .expect("READER_CORPUS is a directory")
        .map(|entry| entry.expect("the corpus is readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    paths.sort();

    paths.iter().map(|path| post(path)).collect()
}

/// One post, from a JSON file like the ones [`posts`] reads.
pub(super) fn post(path: &Path) -> Post {
    let json: Value = serde_json::from_str(&fs::read_to_string(path).expect("posts are UTF-8"))
        .expect("posts are JSON");
    let field = |name: &str| json["fields"][name].as_str().unwrap_or_default().to_owned();
    Post {
        id: path.file_stem().unwrap().to_string_lossy().into_owned(),
        url: field("URL"),
        content: field("content"),
        excerpt: field("excerpt"),
        featured_image: field("featured_image"),
        features: json["features"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|feature| feature.as_str().map(str::to_owned))
            .collect(),
    }
}

/// What the cleanup left that it promises to remove, including anything
/// [`safety_problems`](super::safety_tests::safety_problems) finds.
///
/// Markup inside raw text, like an iframe's fallback content, is text, so this looks at the parsed
/// elements rather than the HTML's text.
pub(super) fn cleanup_problems(id: &str, content: &ReaderPostContent) -> Vec<String> {
    let mut problems: Vec<String> = super::safety_tests::safety_problems(content)
        .into_iter()
        .map(|problem| format!("{id}: {problem}"))
        .collect();
    let parsed = Html::parse_fragment(&content.html());
    if parsed
        .root_element()
        .descendants()
        .any(|node| matches!(node.value(), Node::Comment(_)))
    {
        problems.push(format!("{id}: a comment is left"));
    }

    let offline = Html::parse_fragment(&content.offline_html());
    let has_srcset = offline
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .any(|element| element.value().name() == "img" && element.value().attr("srcset").is_some());
    if has_srcset {
        problems.push(format!("{id}: the offline HTML has an image with a srcset"));
    }
    problems
}

/// What `ReaderPostContent` found in a post, without anything that names the post, so the same post
/// before and after anonymizing summarizes the same way.
pub(super) fn summarize(post: &Post, content: &ReaderPostContent) -> String {
    let script_hosts: Vec<String> = content
        .scripts()
        .iter()
        .filter_map(|script| Some(url::Url::parse(script).ok()?.host_str()?.to_owned()))
        .collect();
    let anchors: Vec<String> = content
        .anchors()
        .iter()
        .map(|anchor| format!("{:?}", anchor.kind))
        .collect();
    let images = content.images();
    format!(
        "images {} (sizes {}, widths {}, captions {}) · galleries {:?} · mentions {} · anchors {:?} \
         · scripts {:?} · offline images {} · text {} lines · manual excerpt {} · featured image \
         shown {} · sized iframes {} · videos {} · links {} · link kinds {:?} · cut off {} \
         · reading time {:?}",
        images.len(),
        images.iter().map(|image| image.sizes.len()).sum::<usize>(),
        images.iter().filter(|image| image.width.is_some()).count(),
        images
            .iter()
            .filter(|image| image.caption.is_some())
            .count(),
        content
            .galleries()
            .iter()
            .map(|gallery| gallery.images.len())
            .collect::<Vec<_>>(),
        content.mentions().len(),
        anchors,
        script_hosts,
        content.offline_image_urls().len(),
        content.plain_text().lines().count(),
        content.is_manual_excerpt(&post.excerpt),
        !post.featured_image.is_empty() && content.includes_image(&post.featured_image),
        content.html().matches("max-width:").count(),
        content.html().matches("<video").count(),
        content.html().matches("<a ").count(),
        link_kinds(post, content),
        content.is_cut_off(),
        content.reading_time_minutes(),
    )
}

/// How many of the post's links go to each kind of place, by what [`reader_link`] makes of them.
fn link_kinds(post: &Post, content: &ReaderPostContent) -> BTreeMap<String, usize> {
    let html = Html::parse_fragment(&content.html());
    let post_url = (!post.url.is_empty()).then(|| post.url.clone());
    let mut kinds = BTreeMap::new();
    for link in html
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "a")
    {
        let Some(href) = link.value().attr("href") else {
            continue;
        };
        let kind = format!("{:?}", reader_link(href, post_url.clone()));
        let kind = kind
            .split([' ', '{', '('])
            .next()
            .unwrap_or_default()
            .to_owned();
        *kinds.entry(kind).or_default() += 1;
    }
    kinds
}

/// How often `ReaderPostContent` agrees with the detector about one feature.
#[derive(Default)]
struct Agreement {
    both: usize,
    only_detector: Vec<String>,
    only_us: Vec<String>,
}

impl Agreement {
    fn record(&mut self, id: &str, detector: bool, us: bool) {
        match (detector, us) {
            (true, true) => self.both += 1,
            (true, false) => self.only_detector.push(id.to_owned()),
            (false, true) => self.only_us.push(id.to_owned()),
            (false, false) => {}
        }
    }
}

#[test]
#[ignore = "needs a corpus of real posts: set READER_CORPUS"]
fn corpus() {
    let Some(corpus) = env::var_os("READER_CORPUS") else {
        panic!("Set READER_CORPUS to a directory of posts");
    };
    let posts = posts(Path::new(&corpus));
    assert!(!posts.is_empty(), "READER_CORPUS has no posts");
    let out = env::var_os("READER_CORPUS_OUT").map(std::path::PathBuf::from);
    let mut summary = String::new();
    if let Some(out) = &out {
        fs::create_dir_all(out).expect("READER_CORPUS_OUT can be created");
    }

    let mut problems: Vec<String> = Vec::new();
    let mut agreements: BTreeMap<&str, Agreement> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut count = |key: String| *counts.entry(key).or_default() += 1;
    let mut slowest: Vec<(Duration, String, usize)> = Vec::new();

    for post in &posts {
        let started = Instant::now();
        let content = post.content();
        let elapsed = started.elapsed();
        slowest.push((elapsed, post.id.clone(), post.content.len()));

        let html = content.html();
        let has = |feature: &str| post.features.iter().any(|f| f == feature);
        if let Some(out) = &out {
            fs::write(out.join(format!("{}.html", post.id)), &html)
                .expect("READER_CORPUS_OUT is writable");
        }

        problems.extend(cleanup_problems(&post.id, &content));
        summary += &format!("{}\t{}\n", post.id, summarize(post, &content));

        // Where we and the detector should agree.
        agreements.entry("mentions").or_default().record(
            &post.id,
            has("mention"),
            !content.mentions().is_empty(),
        );
        let anchors = content.anchors();
        agreements.entry("footnotes").or_default().record(
            &post.id,
            has("footnotes-block") || has("footnotes-markdown"),
            anchors
                .iter()
                .any(|anchor| anchor.kind == ReaderPostAnchorKind::Footnote),
        );
        agreements.entry("headings with an id").or_default().record(
            &post.id,
            has("heading-id"),
            anchors
                .iter()
                .any(|anchor| matches!(anchor.kind, ReaderPostAnchorKind::Heading { .. })),
        );
        agreements.entry("galleries").or_default().record(
            &post.id,
            [
                "gallery-block-nested",
                "gallery-block-legacy",
                "tiled-gallery-block",
                "tiled-gallery-legacy",
                "gallery-shortcode",
            ]
            .iter()
            .any(|feature| has(feature)),
            !content.galleries().is_empty(),
        );
        if has("excerpt-authored") || has("excerpt-generated") {
            agreements.entry("manual excerpts").or_default().record(
                &post.id,
                has("excerpt-authored"),
                content.is_manual_excerpt(&post.excerpt),
            );
        }
        if !post.featured_image.is_empty() {
            agreements
                .entry("featured image in the post")
                .or_default()
                .record(
                    &post.id,
                    has("featured-image-is-first-image"),
                    content.includes_image(&post.featured_image),
                );
        }

        // What's in the cleaned posts.
        for script in content.scripts() {
            let host = url::Url::parse(&script)
                .ok()
                .and_then(|url| url.host_str().map(str::to_owned))
                .unwrap_or(script);
            count(format!("script host: {host}"));
        }
        let images = content.images();
        count(format!("posts with images: {}", !images.is_empty()));
        for image in &images {
            count("images".to_owned());
            if !image.url.starts_with("https://") {
                count(format!(
                    "images not on https: {}",
                    image.url.split(':').next().unwrap_or_default()
                ));
            }
            if image.sizes.is_empty() {
                count("images without a srcset".to_owned());
            }
            if image.width.is_none() {
                count("images without a known width".to_owned());
            }
        }
        let with_photon = content.with_photon_sizes();
        count(format!(
            "images given Photon sizes: {}",
            with_photon
                .images()
                .iter()
                .zip(&images)
                .filter(|(after, before)| after.sizes.len() > before.sizes.len())
                .count()
                > 0
        ));
        if html.contains("max-width:") {
            count("posts with an iframe we sized".to_owned());
        }
        if html.contains(r#"class="wp-block-cover__video-background""#) && !html.contains("<video")
        {
            count("cover videos without a <video>".to_owned());
        }
        if html.contains("This post is password protected.") {
            count("password-protected posts".to_owned());
        }

        // What the app would do with each link, and the image viewer with each gallery image.
        let parsed = Html::parse_fragment(&html);
        for href in parsed
            .root_element()
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|element| element.value().name() == "a")
            .filter_map(|link| link.value().attr("href"))
        {
            let link = reader_link(href, (!post.url.is_empty()).then(|| post.url.clone()));
            let kind = format!("{link:?}");
            let kind = kind.split([' ', '{']).next().unwrap_or_default();
            count(format!("links: {kind}"));
            if let ReaderLink::Image { url } = &link
                && content.gallery_position(url).is_some()
            {
                count("links: Image, in a gallery".to_owned());
            }
        }
        for gallery in content.galleries() {
            for image in &gallery.images {
                let found = content
                    .gallery_position(&image.url)
                    .is_some_and(|position| {
                        image_key(&position.gallery.images[position.index as usize].url)
                            == image_key(&image.url)
                    });
                if !found {
                    problems.push(format!(
                        "{}: no gallery position for {}",
                        post.id, image.url
                    ));
                }
            }
        }
        if content.is_cut_off() {
            count("posts cut off".to_owned());
        }
        count(format!(
            "reading time: {}",
            match content.reading_time_minutes() {
                None => "none".to_owned(),
                Some(minutes @ 0..=5) => format!("{minutes} min"),
                Some(6..=15) => "6–15 min".to_owned(),
                Some(_) => "over 15 min".to_owned(),
            }
        ));
        count(format!(
            "preload URLs at 390pt 3x: {}",
            match content.preload_urls(390, 3.0).len() {
                0 => "0",
                1..=5 => "1–5",
                _ => "over 5",
            }
        ));
    }

    if let Some(path) = env::var_os("READER_CORPUS_SUMMARY") {
        fs::write(path, &summary).expect("READER_CORPUS_SUMMARY is writable");
    }
    println!("{} posts", posts.len());

    slowest.sort_by_key(|(elapsed, _, _)| std::cmp::Reverse(*elapsed));
    let total: Duration = slowest.iter().map(|(elapsed, _, _)| *elapsed).sum();
    // The time that this share of posts took longer than, from the slowest first.
    let slower_than = |share: f64| slowest[(slowest.len() as f64 * share) as usize].0;
    println!(
        "\nParsing took {total:?} in all: {:?} a post on average, {:?} for the median, and {:?} \
         at the 95th percentile. Slowest:",
        total / slowest.len() as u32,
        slower_than(0.5),
        slower_than(0.05),
    );
    for (elapsed, id, length) in slowest.iter().take(5) {
        println!("  {elapsed:?}  {id} ({} KB)", length / 1024);
    }

    println!("\nAgreement with the detector (both · only the detector · only us):");
    for (feature, agreement) in &agreements {
        println!(
            "  {feature}: {} · {} · {}",
            agreement.both,
            agreement.only_detector.len(),
            agreement.only_us.len()
        );
        for (label, ids) in [
            ("only the detector", &agreement.only_detector),
            ("only us", &agreement.only_us),
        ] {
            if !ids.is_empty() {
                let examples: Vec<_> = ids.iter().take(12).map(String::as_str).collect();
                println!("      {label}: {}", examples.join(", "));
            }
        }
    }

    println!("\nCounts:");
    for (key, value) in &counts {
        println!("  {key}: {value}");
    }

    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
