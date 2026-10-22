//! Turns each post in a corpus of real Reader API responses into its [`Structure`]: its elements,
//! the classes WordPress and its plugins give them, and the shape of its text and URLs, as Rust
//! types that can't hold anything that could identify the post, its site or its author. So the
//! structures can be kept anywhere.
//!
//! ```sh
//! READER_CORPUS=path/to/corpus READER_CORPUS_TYPED=path/to/output \
//!   cargo test --release -p wp_mobile --lib reader::structure_tests::corpus_structures \
//!   -- --ignored --nocapture
//! ```
//!
//! - `READER_CORPUS_TYPED` gets the structures themselves, as JSON lines and as postcard.
//! - `READER_CORPUS_LEAKS` gets the rare words left in structures, if any, with where they are. It
//!   holds the corpus's words, so keep it with the corpus.
//! - `READER_CORPUS_DEBUG` names one post, to write its cleaned HTML before and after, and print the
//!   structure's side of each link, image and line of text that differs.
//!
//! `corpus_patterns` picks the fewest, smallest fragments of the structures that make the library do
//! everything it does across the corpus. See [`structure::patterns`].
//!
//! ```sh
//! READER_CORPUS=path/to/corpus READER_PATTERNS=path/to/patterns.postcard \
//!   READER_PATTERNS_PREVIEW=path/to/preview.html \
//!   cargo test --release -p wp_mobile --lib reader::structure_tests::corpus_patterns \
//!   -- --ignored --nocapture
//! ```
//!
//! The tests check both halves of that: that no rare word, name or number from a post is left in
//! its structure, and that `ReaderPostContent` finds the same things in the structure as in the
//! post.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    sync::LazyLock,
};

use regex::Regex;
use scraper::{ElementRef, Html};
use url::Url;

use super::corpus_tests::{Post, posts, summarize};
use super::structure::media;
use super::structure::patterns::{self, Feature, Named, Patterns};
use super::structure::{self, Structure};
use super::{ReaderFixedTheme, ReaderPageColors, ReaderPageOptions};

// --- What's kept ---------------------------------------------------------------------------------

/// What the tool writes itself, beyond its vocabularies: the stand-ins' names, and the words its
/// types serialize as. Rare as they may be, they don't come from a post.
const TOOL_WORDS: &[&str] = &[
    "anywhere",
    "invalid",
    "example",
    "site",
    "post",
    "user",
    "custom",
    "element",
    "someone",
    "javascript",
    "void",
    "mailto",
    "base64",
    "https",
    "http",
    "wordpress",
    "files",
    "wpcomstaging",
    "uploads",
    "content",
    "2020",
    "100000",
    "schema",
    "bsky",
    "feed",
    "widgets",
];

static VOCABULARY: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut words: Vec<String> = structure::vocabulary_words()
        .into_iter()
        .map(str::to_owned)
        .collect();
    words.extend(structure::class_names());
    words.extend(structure::attribute_names().into_iter().map(str::to_owned));
    words.extend(TOOL_WORDS.iter().map(|word| (*word).to_owned()));
    tokens(&words.join(" "))
});

/// A size, like `1024`, `1000px`, `768w` or `1024x768`, which says nothing about a post.
static SIZE_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{1,5}(px|w|x|x\d{1,5}|em|rem|vw|vh)?$").expect("a valid regex")
});

// --- What the code reads -------------------------------------------------------------------------

/// The library's own sources, without their tests.
fn library_sources() -> Vec<&'static str> {
    [
        include_str!("embeds.rs"),
        include_str!("clean.rs"),
        include_str!("content.rs"),
        include_str!("images.rs"),
        include_str!("text.rs"),
        include_str!("links.rs"),
        include_str!("anchors.rs"),
        include_str!("shortcodes.rs"),
        include_str!("getty.rs"),
        include_str!("gists.rs"),
    ]
    .into_iter()
    .map(|source| source.split("#[cfg(test)]").next().unwrap_or(source))
    .collect()
}

static SELECTOR_ARGUMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r##"(?s)(?:selector\(|Selector::parse\(|selector:)\s*(?:r#"(.*?)"#|"((?:[^"\\]|\\.)*)")"##,
    )
    .expect("a valid regex")
});

static QUOTED_VALUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\\?"[^"\\]*\\?"|'[^']*'"#).expect("a valid regex"));

/// The classes the library and its stylesheet read: the stylesheet's selectors, `class ==`
/// comparisons, the `*_CLASSES` and `HIDDEN_BLOCKS` lists, and the code's selectors.
fn classes_the_code_reads() -> BTreeSet<String> {
    let class = Regex::new(r"\.([a-zA-Z_][\w-]*)").expect("a valid regex");
    let mut classes = BTreeSet::new();
    let css = Regex::new(r"(?s)/\*.*?\*/")
        .expect("a valid regex")
        .replace_all(include_str!("reader.css"), "")
        .into_owned();
    for selectors in Regex::new(r"([^{}]+)\{")
        .expect("a valid regex")
        .captures_iter(&css)
    {
        if !selectors[1].trim().starts_with('@') {
            let selectors = QUOTED_VALUE.replace_all(&selectors[1], "");
            classes.extend(
                class
                    .captures_iter(&selectors)
                    .map(|found| found[1].to_owned()),
            );
        }
    }
    let compared = Regex::new(r#"class == "([\w-]+)""#).expect("a valid regex");
    let list = Regex::new(r"(?s)const (?:\w+_CLASSES|HIDDEN_BLOCKS): &\[&str\] = &\[(.*?)\];")
        .expect("a valid regex");
    let quoted = Regex::new(r#""([\w-]+)""#).expect("a valid regex");
    for source in library_sources() {
        classes.extend(
            compared
                .captures_iter(source)
                .map(|found| found[1].to_owned()),
        );
        for found in list.captures_iter(source) {
            classes.extend(
                quoted
                    .captures_iter(&found[1])
                    .map(|found| found[1].to_owned()),
            );
        }
        for found in SELECTOR_ARGUMENT.captures_iter(source) {
            let text = found
                .get(1)
                .or(found.get(2))
                .map_or("", |text| text.as_str());
            // Not a value in an attribute selector, like `[href*="tumblr.com"]`.
            let text = QUOTED_VALUE.replace_all(text, "");
            classes.extend(class.captures_iter(&text).map(|found| found[1].to_owned()));
        }
    }
    classes
}

/// The attributes the library and `reader.js` read: the code's `attr` calls and selectors, its
/// lazy-loading lists, and the `data-` attributes `reader.js` reads.
fn attributes_the_code_reads() -> BTreeSet<String> {
    let read = Regex::new(r#"attr\("([\w:-]+)"\)"#).expect("a valid regex");
    let in_selector = Regex::new(r"\[([a-zA-Z_][\w-]*)").expect("a valid regex");
    let lazy = Regex::new(r"(?s)const LAZY_\w+: &\[&str\] = &\[(.*?)\];").expect("a valid regex");
    let quoted = Regex::new(r#""([\w-]+)""#).expect("a valid regex");
    let mut attributes = BTreeSet::new();
    for source in library_sources() {
        attributes.extend(read.captures_iter(source).map(|found| found[1].to_owned()));
        for found in SELECTOR_ARGUMENT.captures_iter(source) {
            let text = found
                .get(1)
                .or(found.get(2))
                .map_or("", |text| text.as_str());
            attributes.extend(
                in_selector
                    .captures_iter(text)
                    .map(|found| found[1].to_owned()),
            );
        }
        for found in lazy.captures_iter(source) {
            attributes.extend(
                quoted
                    .captures_iter(&found[1])
                    .map(|found| found[1].to_owned()),
            );
        }
    }
    let script = include_str!("reader.js");
    let data = Regex::new(r"\[(data-[\w-]+)").expect("a valid regex");
    let dataset = Regex::new(r"dataset\.([a-zA-Z]+)").expect("a valid regex");
    attributes.extend(data.captures_iter(script).map(|found| found[1].to_owned()));
    for found in dataset.captures_iter(script) {
        // `dataset.readerImage` is `data-reader-image`.
        let name: String = found[1]
            .chars()
            .flat_map(|c| {
                if c.is_ascii_uppercase() {
                    vec!['-', c.to_ascii_lowercase()]
                } else {
                    vec![c]
                }
            })
            .collect();
        attributes.insert(format!("data-{name}"));
    }
    // What `reader.js` writes itself, rather than reads from a post.
    attributes.remove("data-reader-image");
    attributes
}

/// The hosts the stylesheet's and the code's selectors name, like `iframe[src*="icloud.com"]`.
fn hosts_the_code_reads() -> BTreeSet<String> {
    let host = Regex::new(r#"\[(?:src|href)[*^$|~]?=\\?["']([^"'\\]+)"#).expect("a valid regex");
    let mut texts = vec![include_str!("reader.css").to_owned()];
    for source in library_sources() {
        for found in SELECTOR_ARGUMENT.captures_iter(source) {
            texts.push(
                found
                    .get(1)
                    .or(found.get(2))
                    .map_or("", |text| text.as_str())
                    .to_owned(),
            );
        }
    }
    texts
        .iter()
        .flat_map(|text| {
            host.captures_iter(text)
                .map(|found| found[1].to_owned())
                .collect::<Vec<_>>()
        })
        .map(|value| {
            let value = value
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            value
                .trim_start_matches("//")
                .split('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .filter(|host| host.contains('.'))
        .collect()
}

/// Every class and attribute the library, its stylesheet or `reader.js` reads, a structure keeps.
/// Otherwise a change to the library that reads a new one would go untested by the structures.
#[test]
fn the_vocabulary_has_what_the_code_reads() {
    let classes: Vec<String> = classes_the_code_reads()
        .into_iter()
        .filter(|class| structure::kept_class(class).as_deref() != Some(class.as_str()))
        .collect();
    assert!(
        classes.is_empty(),
        "add these to the vocabulary: {classes:?}"
    );
    let known: HashSet<&str> = structure::attribute_names().into_iter().collect();
    let attributes: Vec<String> = attributes_the_code_reads()
        .into_iter()
        .filter(|attribute| {
            !known.contains(attribute.to_ascii_lowercase().as_str())
                && !attribute.starts_with("data-wp-")
        })
        .collect();
    assert!(
        attributes.is_empty(),
        "add these to an attribute table: {attributes:?}"
    );
    let hosts: Vec<String> = hosts_the_code_reads()
        .into_iter()
        .filter(|host| !structure::is_provider_host(host))
        .collect();
    assert!(hosts.is_empty(), "add these to `KnownHost`: {hosts:?}");
    let words: Vec<String> = path_words_the_code_reads()
        .into_iter()
        .filter(|word| !structure::is_path_word(word))
        .collect();
    assert!(words.is_empty(), "add these to `PathWord`: {words:?}");
    let tags: Vec<String> = tags_the_code_reads()
        .into_iter()
        .filter(|tag| !structure::is_known_tag(tag))
        .collect();
    assert!(tags.is_empty(), "add these to `Tag`: {tags:?}");
}

/// The words `reader_link` matches in a link's path, like `["mentions", username]`.
fn path_words_the_code_reads() -> BTreeSet<String> {
    let source = include_str!("links.rs");
    let source = source.split("#[cfg(test)]").next().unwrap_or(source);
    // A slice pattern, which isn't a list's `&[…]`.
    let pattern = Regex::new(r#"(?:^|[^&])\[([^\[\]]*"[^\[\]]*)\]"#).expect("a valid regex");
    let quoted = Regex::new(r#""([\w-]+)""#).expect("a valid regex");
    pattern
        .captures_iter(source)
        .flat_map(|found| {
            quoted
                .captures_iter(&found[1])
                .map(|found| found[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The elements the library names: in `name() ==` comparisons, `matches!` and `match`es on an
/// element's name, and its lists of elements.
fn tags_the_code_reads() -> BTreeSet<String> {
    let quoted = Regex::new(r#""([\w:-]+)""#).expect("a valid regex");
    let compared = Regex::new(r#"name\(\) [!=]= "([\w:-]+)""#).expect("a valid regex");
    let matched = Regex::new(r"(?s)matches!\(\s*[\w.&*]*name\(\),(.*?)\)").expect("a valid regex");
    let lists = Regex::new(
        r"(?s)const (?:NOT_TEXT|BLOCKS|MEDIA|PHRASING|SIMPLIFIED): &\[&str\] = &\[(.*?)\];",
    )
    .expect("a valid regex");
    let match_start = Regex::new(r"match [\w.&*]*name\(\) \{").expect("a valid regex");
    let arm = Regex::new(r#"(?m)^\s*((?:"[\w:-]+"\s*\|\s*)*"[\w:-]+")\s*(?:=>|if\b)"#)
        .expect("a valid regex");
    let mut tags = BTreeSet::new();
    for source in library_sources() {
        tags.extend(
            compared
                .captures_iter(source)
                .map(|found| found[1].to_owned()),
        );
        for found in matched
            .captures_iter(source)
            .chain(lists.captures_iter(source))
        {
            tags.extend(
                quoted
                    .captures_iter(&found[1])
                    .map(|found| found[1].to_owned()),
            );
        }
        // The arms of a `match` on an element's name, up to its closing brace.
        for start in match_start.find_iter(source) {
            let mut depth = 0;
            let mut end = start.end();
            for (index, c) in source[start.end() - 1..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    end = start.end() - 1 + index;
                    break;
                }
            }
            for found in arm.captures_iter(&source[start.end()..end]) {
                tags.extend(
                    quoted
                        .captures_iter(&found[1])
                        .map(|found| found[1].to_owned()),
                );
            }
        }
    }
    tags
}

// --- The test posts ------------------------------------------------------------------------------

fn test_posts() -> Vec<(String, Post)> {
    let test_data = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/reader");
    ["corpus", "test-site"]
        .into_iter()
        .flat_map(|directory| {
            posts(&test_data.join(directory))
                .into_iter()
                .map(move |post| (format!("{directory}/{}", post.id), post))
        })
        .collect()
}

/// The test posts, which are anonymized already, keep what the library finds in them as
/// structures. So a change to the library that the structures miss shows up without the corpus.
///
/// "Sized iframes" counts `max-width:` anywhere in the HTML, including in code and in plugins'
/// settings, which structures don't keep, so it doesn't count here.
#[test]
fn test_posts_keep_what_the_library_finds() {
    let fields = |summary: String| -> Vec<String> {
        summary
            .split(" · ")
            .filter(|field| !field.starts_with("sized iframes"))
            .map(str::to_owned)
            .collect()
    };
    for (name, post) in test_posts() {
        let rebuilt = Structure::of(&post).post(&post.id);
        assert_eq!(
            fields(summarize(&rebuilt, &rebuilt.content())),
            fields(summarize(&post, &post.content())),
            "{name}"
        );
    }
}

/// A structure reads back as itself, from JSON and from postcard's binary format.
#[test]
fn structures_read_back_as_themselves() {
    for (name, post) in test_posts() {
        let structure = Structure::of(&post);
        let json = serde_json::to_string(&structure).expect("structures are JSON");
        let from_json: Structure = serde_json::from_str(&json).expect("structures read back");
        assert_eq!(from_json, structure, "{name} from JSON");
        let binary = postcard::to_stdvec(&structure).expect("structures are binary");
        let from_binary: Structure = postcard::from_bytes(&binary).expect("structures read back");
        assert_eq!(from_binary, structure, "{name} from postcard");
    }
}

/// A structure's checked strings are checked again when they're read, so a hand-edited one can't
/// smuggle a word in.
#[test]
fn rejects_words_where_the_types_have_none() {
    for json in [
        r#"{"url":"empty","featured_image":"empty","content":[{"text":[{"separator":"Jane"}]}],"excerpt":[]}"#,
        r#"{"url":"empty","featured_image":"empty","content":[{"element":{"tag":"janes-tag"}}],"excerpt":[]}"#,
        r#"{"url":"empty","featured_image":"empty","content":[{"element":{"tag":"p","attributes":[{"width":{"number":"Jane"}}]}}],"excerpt":[]}"#,
        r#"{"url":"empty","featured_image":"empty","content":[{"element":{"tag":"p","attributes":[{"class":[{"kept":"janes-class"}]}]}}],"excerpt":[]}"#,
    ] {
        assert!(
            serde_json::from_str::<Structure>(json).is_err(),
            "{json} read back"
        );
    }
}

#[test]
fn drops_what_identifies_a_post() {
    let post = Post {
        id: "1".to_owned(),
        url: "https://janedoe.wordpress.com/2024/05/my-trip-to-lisbon/".to_owned(),
        content: r#"
            <p class="has-janes-brand-color has-text-color janes-custom-class">Jane Doe went to
            Lisbon. Email <a href="mailto:jane@example.org">jane@example.org</a> or see
            <a href="https://janedoe.tumblr.com/post/123456789">her Tumblr</a>.</p>
            <figure class="wp-block-image size-large"><img src="https://janedoe.files.wordpress.com/2024/05/lisbon-beach-1024x768.jpg?w=1024"
              alt="Jane on the beach" data-id="98765432" class="wp-image-98765432"
              data-image-meta='{"camera":"Pixel 7"}'></figure>
            <p>Thanks <a class="mention" data-username="johnsmith" href="https://wordpress.com/mentions/johnsmith/">@johnsmith</a>!
            [jane_private_widget token="s3cret"] <time datetime="2024-05-12T09:30:00+01:00">May 12</time></p>
            <div id="jane-anchor" data-wpcom-embed-url="https://www.youtube.com/watch?v=Jane123"></div>
            <iframe src="https://janedoe.wordpress.com/embed/" target="wpcom-iframe-a186a0302e24eaf0f71c3fc479bfce7e"></iframe>
            <div class="is-janes-thing wp-janedoe-box"><img src="https://janedoe.files.wordpress.com/a.jpg"
              data-image-description="Jane at the beach" itemtype="http://schema.org/JanesType"
              lang="janedoe" data-theme="janes-dark" sizes="janedoe" data-wp-class--janes-thing="x"></div>
        "#
        .to_owned(),
        excerpt: "<p>Jane Doe went to Lisbon.</p>".to_owned(),
        featured_image: "https://janedoe.files.wordpress.com/2024/05/lisbon-beach.jpg".to_owned(),
        features: Vec::new(),
    };
    let structure = Structure::of(&post);
    let rebuilt = structure.post("1");
    let written = format!(
        "{} {} {} {} {}",
        serde_json::to_string(&structure).expect("structures are JSON"),
        rebuilt.url,
        rebuilt.featured_image,
        rebuilt.content,
        rebuilt.excerpt
    )
    .to_lowercase();
    let found: Vec<&str> = [
        "jane",
        "doe",
        "lisbon",
        "trip",
        "example.org",
        "tumblr.com/post/123",
        "beach",
        "pixel",
        "98765432",
        "johnsmith",
        "s3cret",
        "private_widget",
        "2024-05-12",
        "Jane123",
        "a186a0302e24eaf0f71c3fc479bfce7e",
        "brand",
        "custom-class",
    ]
    .into_iter()
    .filter(|identifying| written.contains(&identifying.to_lowercase()))
    .collect();
    assert!(found.is_empty(), "{found:?} left in {written}");
    // What the library uses is still there.
    for kept in [
        "wp-block-image",
        "size-large",
        "-1024x768.jpg",
        "w=1024",
        "www.youtube.com",
        // `user1` is the `example` in `jane@example.org`: a word after an `@` is a username.
        "/mentions/user2/",
        r#"data-username="user2""#,
    ] {
        assert!(
            rebuilt.content.contains(kept),
            "{kept} missing from {}",
            rebuilt.content
        );
    }
}

// --- The corpus ----------------------------------------------------------------------------------

/// What the library makes of a post, as facts like `script:www.instagram.com` or `gallery`, without
/// anything that names the post.
fn outcomes(post: &Post) -> BTreeSet<String> {
    let content = post.content();
    let html = content.html();
    let parsed = Html::parse_fragment(&html);
    let mut facts = BTreeSet::new();
    for script in content.scripts() {
        if let Some(host) = Url::parse(&script)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
        {
            facts.insert(format!("script:{host}"));
        }
    }
    for anchor in content.anchors() {
        facts.insert(format!("anchor:{:?}", anchor.kind));
    }
    let images = content.images();
    let mut fact = |condition: bool, name: &str| {
        if condition {
            facts.insert(name.to_owned());
        }
    };
    fact(!images.is_empty(), "images");
    fact(
        images.iter().any(|image| !image.sizes.is_empty()),
        "image with sizes",
    );
    fact(
        images.iter().any(|image| image.width.is_some()),
        "image with a width",
    );
    fact(
        images.iter().any(|image| image.caption.is_some()),
        "image with a caption",
    );
    fact(!content.galleries().is_empty(), "gallery");
    fact(!content.mentions().is_empty(), "mention");
    fact(!content.offline_image_urls().is_empty(), "offline image");
    fact(html.contains("<video"), "video");
    fact(html.contains("<audio"), "audio");
    fact(content.is_cut_off(), "cut off");
    fact(content.reading_time_minutes().is_none(), "no text");
    fact(
        content
            .plain_text()
            .chars()
            .any(|c| ('\u{0590}'..='\u{06FF}').contains(&c)),
        "right-to-left text",
    );
    if !post.excerpt.is_empty() {
        fact(content.is_manual_excerpt(&post.excerpt), "manual excerpt");
        fact(
            !content.is_manual_excerpt(&post.excerpt),
            "excerpt made from the post",
        );
    }
    if !post.featured_image.is_empty() {
        let shown = content.includes_image(&post.featured_image);
        fact(shown, "featured image in the post");
        fact(!shown, "featured image elsewhere");
    }
    for element in parsed
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
    {
        let name = element.value().name();
        if !matches!(
            name,
            "iframe" | "a" | "img" | "video" | "audio" | "blockquote"
        ) {
            continue;
        }
        // Where an iframe goes, by kind: a provider's host, or another site.
        if name == "iframe" {
            let host = element
                .value()
                .attr("src")
                .and_then(|src| Url::parse(src).ok())
                .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
                .unwrap_or_default();
            let kind = if structure::is_provider_host(&host) {
                host
            } else {
                "another site".to_owned()
            };
            facts.insert(format!("iframe:{kind}"));
            if element
                .value()
                .attr("style")
                .is_some_and(|style| style.contains("max-width"))
            {
                facts.insert("sized iframe".to_owned());
            }
        }
        for class in element.value().classes() {
            if class.starts_with("wp-block-") || class.starts_with("embed-") {
                facts.insert(format!("{name}.{class}"));
            }
        }
    }
    facts
}

/// Picks the patterns in a corpus, writes them, and checks that they make the library do everything
/// it does across the corpus.
#[test]
#[ignore = "needs a corpus of real posts: set READER_CORPUS and READER_PATTERNS"]
fn corpus_patterns() {
    let (Some(corpus), Some(out)) = (env::var_os("READER_CORPUS"), env::var_os("READER_PATTERNS"))
    else {
        panic!("Set READER_CORPUS to a directory of posts, and READER_PATTERNS to where to write");
    };
    let corpus = posts(Path::new(&corpus));
    let structures: Vec<Structure> = corpus.iter().map(Structure::of).collect();
    let named = Named {
        classes: classes_the_code_reads(),
        attributes: attributes_the_code_reads(),
    };
    let picked = patterns::pick(&structures, &named);
    for line in &picked.report {
        println!("{line}");
    }
    let binary = postcard::to_stdvec(&picked.patterns).expect("patterns are binary");
    fs::write(&out, &binary).expect("READER_PATTERNS is writable");
    println!(
        "{} KB of postcard in {}",
        binary.len() / 1024,
        Path::new(&out).display()
    );
    if let Some(preview) = env::var_os("READER_PATTERNS_PREVIEW") {
        fs::write(&preview, preview_html(&picked.patterns, &picked.covers))
            .expect("READER_PATTERNS_PREVIEW is writable");
    }
    check_coverage(&corpus, &picked.patterns);
}

/// Whether the patterns in `READER_PATTERNS` make the library do everything the corpus does.
#[test]
#[ignore = "needs a corpus of real posts and its patterns: set READER_CORPUS and READER_PATTERNS"]
fn patterns_cover_the_corpus() {
    let (Some(corpus), Some(patterns)) =
        (env::var_os("READER_CORPUS"), env::var_os("READER_PATTERNS"))
    else {
        panic!("Set READER_CORPUS to a directory of posts, and READER_PATTERNS to their patterns");
    };
    let patterns: Patterns =
        postcard::from_bytes(&fs::read(patterns).expect("readable")).expect("patterns read back");
    check_coverage(&posts(Path::new(&corpus)), &patterns);
}

/// The patterns as posts, which the library can run on.
fn pattern_posts(patterns: &Patterns) -> Vec<Post> {
    patterns
        .all()
        .enumerate()
        .map(|(index, structure)| structure.post(&format!("pattern-{index}")))
        .collect()
}

fn check_coverage(corpus: &[Post], patterns: &Patterns) {
    let mut in_corpus: BTreeMap<String, usize> = BTreeMap::new();
    for post in corpus {
        for fact in outcomes(post) {
            *in_corpus.entry(fact).or_default() += 1;
        }
    }
    let pattern_posts = pattern_posts(patterns);
    let in_patterns: BTreeSet<String> = pattern_posts.iter().flat_map(outcomes).collect();
    let mut problems = Vec::new();
    for post in &pattern_posts {
        problems.extend(super::corpus_tests::cleanup_problems(
            &post.id,
            &post.content(),
        ));
    }

    let missing: Vec<_> = in_corpus
        .iter()
        .filter(|(fact, _)| !in_patterns.contains(*fact))
        .collect();
    println!(
        "{} patterns make the library do {} of the {} things it does across the corpus",
        pattern_posts.len(),
        in_corpus.len() - missing.len(),
        in_corpus.len()
    );
    for (fact, posts) in &missing {
        println!("  missing: {fact} (in {posts} posts)");
    }
    let extra: Vec<_> = in_patterns
        .iter()
        .filter(|fact| !in_corpus.contains_key(*fact))
        .collect();
    if !extra.is_empty() {
        println!("Only in the patterns: {extra:?}");
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// A page with every pattern as the Reader shows it, labelled with what it was picked for. Each is
/// the page an app would show, in a frame of its own, with real images, videos and embeds in its
/// stand-ins' place: see [`media`]. A frame fits its post's body, which doesn't depend on the
/// frame's height, so it can't keep growing.
fn preview_html(patterns: &Patterns, covers: &[Vec<Feature>]) -> String {
    let options = || ReaderPageOptions {
        colors: ReaderPageColors::Fixed {
            theme: ReaderFixedTheme::Soft,
        },
        language: None,
        offline: false,
        font: None,
        text_scale: 1.0,
        server_css: None,
        // The apps give the post room at its edges.
        css: Some("body { padding: 16px; }".to_owned()),
    };
    let mut html = "<!doctype html><html><head><meta charset=\"utf-8\">\
        <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><style>\
        body { margin: 0; background: #ddd; } \
        .pattern { margin: 0 0 16px; background: #f2f2f2; } \
        .pattern-label { margin: 0; padding: 8px 16px; font: 11px ui-monospace, monospace; \
        color: #555; background: #fff; overflow-wrap: anywhere; } \
        iframe { display: block; width: 100%; height: 160px; border: 0; }</style>\
        <script>function fit(frame) { try { const body = frame.contentDocument.body; \
        const resize = () => { frame.style.height = Math.min(body.scrollHeight, 4000) + 'px'; }; \
        resize(); new ResizeObserver(resize).observe(body); } \
        catch (error) { frame.style.height = '600px'; } }</script></head><body>"
        .to_owned();
    let structures: Vec<&Structure> = patterns.all().collect();
    for (index, (structure, covers)) in structures.iter().zip(covers).enumerate() {
        let post = media::with_real_media(|| structure.post(&format!("pattern-{index}")));
        let page = post.content().page_html(options());
        let label: Vec<String> = covers.iter().map(ToString::to_string).collect();
        html += &format!(
            "<section class=\"pattern\"><p class=\"pattern-label\">{index}: {}</p>\
             <iframe loading=\"lazy\" onload=\"fit(this)\" srcdoc=\"{}\"></iframe></section>",
            label.join(" · ").replace('&', "&amp;").replace('<', "&lt;"),
            page.replace('&', "&amp;").replace('"', "&quot;")
        );
    }
    html + "</body></html>"
}

/// Every item in the preview's media library still loads.
#[test]
#[ignore = "needs the network"]
fn media_library_is_still_there() {
    let failures: Vec<String> = media::checks()
        .into_iter()
        .filter_map(|url| {
            let output = std::process::Command::new("curl")
                .args([
                    "-sL",
                    "-o",
                    "/dev/null",
                    "--max-time",
                    "30",
                    "-w",
                    "%{http_code}",
                ])
                .args([
                    "-A",
                    "Mozilla/5.0 (Macintosh) AppleWebKit/605.1.15 Safari/605.1.15",
                ])
                .arg(&url)
                .output()
                .expect("curl runs");
            let status = String::from_utf8_lossy(&output.stdout).into_owned();
            (status != "200").then(|| format!("{status} {url}"))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Words, names and numbers in a post: what it says, and its attributes' values.
fn tokens(html: &str) -> HashSet<String> {
    static SPLITTER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"[^\p{L}\p{N}]+").expect("a valid regex"));
    SPLITTER
        .split(&html.to_lowercase())
        .filter(|token| token.chars().count() >= 4)
        .map(str::to_owned)
        .collect()
}

/// The classes and attributes in a corpus that a structure doesn't keep, with the sites each is
/// on: a post's ID starts with its site's.
fn unknown_names(posts: &[Post]) -> BTreeMap<&'static str, BTreeMap<String, BTreeSet<String>>> {
    let mut unknown: BTreeMap<&'static str, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let known_attributes: HashSet<&str> = structure::attribute_names().into_iter().collect();
    for post in posts {
        let document = Html::parse_fragment(&post.content);
        for element in document
            .root_element()
            .descendants()
            .filter_map(ElementRef::wrap)
        {
            for class in element.value().classes() {
                if structure::kept_class(class).is_none() {
                    unknown
                        .entry("class")
                        .or_default()
                        .entry(class.to_owned())
                        .or_default()
                        .insert(site_of(post));
                }
            }
            for (name, _) in element.value().attrs() {
                let name = name.to_ascii_lowercase();
                if !known_attributes.contains(name.as_str()) && !name.starts_with("data-wp-") {
                    unknown
                        .entry("attribute")
                        .or_default()
                        .entry(name)
                        .or_default()
                        .insert(site_of(post));
                }
            }
        }
    }
    unknown
}

/// Each link in the cleaned post, with what `reader_link` makes of it.
fn links_and_kinds(post: &Post) -> Vec<(String, String)> {
    let html = Html::parse_fragment(&post.content().html());
    html.root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "a")
        .filter_map(|element| {
            let href = element.value().attr("href")?.to_owned();
            let kind = format!("{:?}", super::reader_link(&href, Some(post.url.clone())));
            let kind = kind
                .split([' ', '{', '('])
                .next()
                .unwrap_or_default()
                .to_owned();
            Some((kind, href))
        })
        .collect()
}

fn site_of(post: &Post) -> String {
    post.id.split('-').next().unwrap_or_default().to_owned()
}

#[test]
#[ignore = "needs a corpus of real posts: set READER_CORPUS"]
fn corpus_structures() {
    let Some(corpus) = env::var_os("READER_CORPUS") else {
        panic!("Set READER_CORPUS to a directory of posts");
    };
    let typed_out = env::var_os("READER_CORPUS_TYPED").map(std::path::PathBuf::from);
    if let Some(directory) = &typed_out {
        fs::create_dir_all(directory).expect("READER_CORPUS_TYPED can be created");
    }
    let mut posts = posts(Path::new(&corpus));
    assert!(!posts.is_empty(), "READER_CORPUS has no posts");
    // One post, with both cleaned versions written out, to see why they differ.
    let debug = env::var("READER_CORPUS_DEBUG").ok();
    if let Some(id) = &debug {
        posts.retain(|post| &post.id == id);
    }

    // How many posts each token is in. A word in only a few posts can identify one.
    let mut document_frequency: HashMap<String, usize> = HashMap::new();
    let post_tokens: Vec<HashSet<String>> = posts
        .iter()
        .map(|post| {
            let tokens = tokens(&format!(
                "{} {} {} {}",
                post.content, post.excerpt, post.url, post.featured_image
            ));
            for token in &tokens {
                *document_frequency.entry(token.clone()).or_default() += 1;
            }
            tokens
        })
        .collect();

    let mut leaks: BTreeMap<String, usize> = BTreeMap::new();
    let mut posts_with_leaks = 0;
    let mut leak_report: Vec<String> = Vec::new();
    let mut agreement: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut disagreements: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut unique = HashSet::new();
    let (mut original_bytes, mut json_bytes, mut binary_bytes) = (0, 0, 0);
    let mut all_json = Vec::new();
    let mut all_structures = Vec::new();

    for (post, tokens_in_post) in posts.iter().zip(&post_tokens) {
        let structure = Structure::of(post);
        let json = serde_json::to_string(&structure).expect("structures are JSON");
        let binary = postcard::to_stdvec(&structure).expect("structures are binary");
        original_bytes += post.content.len() + post.excerpt.len();
        json_bytes += json.len();
        binary_bytes += binary.len();
        let mut hasher = DefaultHasher::new();
        structure.hash(&mut hasher);
        unique.insert(hasher.finish());
        let rebuilt = structure.post(&post.id);

        // Rare tokens from the post that are still in its structure, as it renders.
        let written = format!(
            "{} {} {} {}",
            rebuilt.content, rebuilt.excerpt, rebuilt.url, rebuilt.featured_image
        )
        .to_lowercase();
        let leaked: Vec<String> = tokens(&written)
            .intersection(tokens_in_post)
            .filter(|token| document_frequency[*token] < 10)
            .filter(|token| !SIZE_TOKEN.is_match(token))
            .filter(|token| !VOCABULARY.contains(*token))
            .cloned()
            .collect();
        if !leaked.is_empty() {
            posts_with_leaks += 1;
        }
        // Where each one is, for a report kept with the corpus, since they're the corpus's words.
        for token in &leaked {
            let position = written.find(token.as_str()).unwrap_or_default();
            let mut start = position.saturating_sub(40);
            while !written.is_char_boundary(start) {
                start -= 1;
            }
            let mut end = (position + token.len() + 20).min(written.len());
            while !written.is_char_boundary(end) {
                end -= 1;
            }
            leak_report.push(format!("{}\t{token}\t{}", post.id, &written[start..end]));
        }
        for token in leaked {
            *leaks.entry(token).or_default() += 1;
        }

        // What the library finds in each, field by field.
        let before = summarize(post, &post.content());
        let after = summarize(&rebuilt, &rebuilt.content());
        if debug.is_some() {
            let dir = env::temp_dir();
            fs::write(dir.join("structure-original.html"), post.content().html())
                .expect("writable");
            fs::write(dir.join("structure-rebuilt.html"), rebuilt.content().html())
                .expect("writable");
            fs::write(dir.join("structure-input.html"), &rebuilt.content).expect("writable");
            fs::write(
                dir.join("structure.json"),
                serde_json::to_string_pretty(&structure).expect("structures are JSON"),
            )
            .expect("writable");
            println!("{before}\n{after}\nwrote {}", dir.display());
            // Only the structure's side, which has nothing of the post's.
            let original = links_and_kinds(post);
            for (index, (kind, href)) in links_and_kinds(&rebuilt).into_iter().enumerate() {
                let was = original
                    .get(index)
                    .map_or("none", |(kind, _)| kind.as_str());
                if was != kind {
                    println!("  link {index}: {was} → {kind}: {href}");
                }
            }
            let images: Vec<String> = rebuilt
                .content()
                .images()
                .into_iter()
                .map(|image| image.url)
                .collect();
            // The text's lines, by shape: letters as `a` and digits as `0`.
            let shape = |text: String| -> Vec<String> {
                text.lines()
                    .map(|line| {
                        line.chars()
                            .map(|c| {
                                if c.is_alphabetic() {
                                    'a'
                                } else if c.is_numeric() {
                                    '0'
                                } else {
                                    c
                                }
                            })
                            .take(60)
                            .collect()
                    })
                    .collect()
            };
            println!(
                "  text in the post: {:?}",
                shape(post.content().plain_text())
            );
            println!(
                "  text in the structure: {:?}",
                shape(rebuilt.content().plain_text())
            );
            println!(
                "  images: {} in the post, {} in the structure: {images:?}",
                post.content().images().len(),
                images.len()
            );
        }
        for (field_before, field_after) in before.split(" · ").zip(after.split(" · ")) {
            let name: String = field_before
                .split(|c: char| c.is_ascii_digit() || c == '[' || c == '(' || c == '{')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned();
            let entry = agreement.entry(name.clone()).or_default();
            entry.1 += 1;
            if field_before == field_after {
                entry.0 += 1;
            } else {
                disagreements
                    .entry(name)
                    .or_default()
                    .push(format!("{}: {field_before} → {field_after}", post.id));
            }
        }

        all_json.push(json);
        all_structures.push(structure);
    }

    if let Some(typed_out) = &typed_out {
        fs::write(typed_out.join("structures.jsonl"), all_json.join("\n"))
            .expect("READER_CORPUS_TYPED is writable");
        fs::write(
            typed_out.join("structures.postcard"),
            postcard::to_stdvec(&all_structures).expect("structures are binary"),
        )
        .expect("READER_CORPUS_TYPED is writable");
    }
    if let Some(path) = env::var_os("READER_CORPUS_LEAKS") {
        fs::write(path, leak_report.join("\n")).expect("READER_CORPUS_LEAKS is writable");
    }
    println!(
        "{} posts, {} different structures",
        posts.len(),
        unique.len()
    );
    println!(
        "{} KB of HTML became {} KB of JSON, or {} KB of postcard",
        original_bytes / 1024,
        json_bytes / 1024,
        binary_bytes / 1024
    );
    println!(
        "\nRare tokens left in structures: {} in {posts_with_leaks} posts",
        leaks.len()
    );
    for (token, count) in leaks.iter().take(40) {
        println!("  {token} ({count})");
    }
    println!("\nWhat the library finds, the same in the post and its structure:");
    for (name, (same, total)) in &agreement {
        println!("  {same:5} / {total}  {name}");
    }
    for (name, examples) in &disagreements {
        println!("\n{name}:");
        for example in examples.iter().take(3) {
            println!("  {example}");
        }
    }
    // What the vocabularies don't have, on three sites or more. On fewer, a name is more likely
    // to be the site's own than a plugin's.
    println!("\nOn 3+ sites, and not in the vocabularies:");
    for (kind, names) in unknown_names(&posts) {
        let mut common: Vec<(&String, usize)> = names
            .iter()
            .map(|(name, sites)| (name, sites.len()))
            .filter(|(_, sites)| *sites >= 3)
            .collect();
        common.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let rare = names.len() - common.len();
        let common: Vec<String> = common
            .iter()
            .map(|(name, sites)| format!("{name} ({sites})"))
            .collect();
        println!(
            "  {kind}: {} (and {rare} on fewer sites)",
            common.join(", ")
        );
    }
}
