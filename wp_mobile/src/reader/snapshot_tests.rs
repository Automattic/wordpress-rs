//! Runs test posts through [`ReaderPostContent`], and compares the results with snapshots.
//!
//! - `test-data/reader/posts` holds post content from the Theme Unit Test site, captured by
//!   `test-data/reader/capture-posts.sh`. Their snapshots are in `test-data/reader/snapshots`.
//! - `test-data/reader/corpus` holds anonymized real posts, made from a private corpus by
//!   `test-data/reader/make-corpus-fixtures.py`. Their snapshots are in
//!   `test-data/reader/snapshots/corpus`.
//! - `test-data/reader/test-site` holds our own test posts, captured by
//!   `test-data/reader/capture-test-site-posts.sh`. Their snapshots are in
//!   `test-data/reader/snapshots/test-site`.
//!
//! Each snapshot is the cleaned post, with what was found in it listed in a comment at the top, so
//! it opens in a browser.
//!
//! To accept a change, regenerate the snapshots and review the diff:
//!
//! ```sh
//! UPDATE_SNAPSHOTS=1 cargo test -p wp_mobile --lib reader::
//! ```

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::corpus_tests::{cleanup_problems, posts};
use super::{ReaderPostAnchor, ReaderPostAnchorKind, ReaderPostContent, ReaderPostImage};

const UPDATE_COMMAND: &str = "UPDATE_SNAPSHOTS=1 cargo test -p wp_mobile --lib reader::";

fn test_data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/reader")
}

#[test]
fn posts_match_their_snapshots() {
    let mut paths: Vec<_> = fs::read_dir(test_data().join("posts"))
        .expect("test-data/reader/posts exists")
        .map(|entry| entry.expect("test-data/reader/posts is readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "html")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "test-data/reader/posts has no posts");

    let snapshots = paths.iter().map(|path| {
        let name = path
            .file_name()
            .expect("posts have file names")
            .to_string_lossy();
        let html = fs::read_to_string(path).expect("posts are UTF-8");
        (
            name.into_owned(),
            snapshot(&ReaderPostContent::new(&html, None), ""),
        )
    });
    compare(&test_data().join("snapshots"), snapshots);
}

#[test]
fn corpus_posts_match_their_snapshots() {
    json_posts_match_their_snapshots("corpus", "snapshots/corpus");
}

#[test]
fn test_site_posts_match_their_snapshots() {
    json_posts_match_their_snapshots("test-site", "snapshots/test-site");
}

#[test]
fn corpus_posts_are_clean() {
    posts_are_clean("corpus");
}

#[test]
fn test_site_posts_are_clean() {
    posts_are_clean("test-site");
}

/// Cleaning a post the cleanup has already cleaned changes nothing. When the Reader API fixes
/// something the cleanup works around, it sends what the cleanup would have made, so this keeps
/// the workarounds from breaking posts once it does.
#[test]
fn cleaning_a_cleaned_post_changes_nothing() {
    let mut posts: Vec<(String, String, Option<String>)> = Vec::new();
    for entry in fs::read_dir(test_data().join("posts")).expect("test-data/reader/posts exists") {
        let path = entry.expect("test-data/reader/posts is readable").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "html")
        {
            let html = fs::read_to_string(&path).expect("posts are UTF-8");
            posts.push((path.display().to_string(), html, None));
        }
    }
    for fixtures in ["corpus", "test-site"] {
        for post in super::corpus_tests::posts(&test_data().join(fixtures)) {
            let url = (!post.url.is_empty()).then(|| post.url.clone());
            posts.push((format!("{fixtures}/{}", post.id), post.content.clone(), url));
        }
    }

    let failures: Vec<String> = posts
        .into_iter()
        .filter_map(|(name, html, url)| {
            let once = ReaderPostContent::new(&html, url.clone());
            let twice = ReaderPostContent::new(&once.html(), url);
            let changed: Vec<&str> = [
                ("html", once.html() == twice.html()),
                ("images", once.images() == twice.images()),
                ("galleries", once.galleries() == twice.galleries()),
                ("anchors", once.anchors() == twice.anchors()),
                ("mentions", once.mentions() == twice.mentions()),
                ("text", once.plain_text() == twice.plain_text()),
            ]
            .into_iter()
            .filter(|(_, same)| !same)
            .map(|(what, _)| what)
            .collect();
            (!changed.is_empty()).then(|| format!("{name}: {}", changed.join(", ")))
        })
        .collect();
    assert!(
        failures.is_empty(),
        "cleaning again changed:\n{}",
        failures.join("\n")
    );
}

/// Compares each post in `test-data/reader/<fixtures>`, a Reader API post as JSON, with its
/// snapshot in `test-data/reader/<snapshots>`.
fn json_posts_match_their_snapshots(fixtures: &str, snapshots: &str) {
    let posts = posts(&test_data().join(fixtures));
    assert!(
        !posts.is_empty(),
        "test-data/reader/{fixtures} has no posts"
    );

    let snapshots_for_posts = posts.iter().map(|post| {
        let content = post.content();
        let mut verdicts = String::new();
        if !post.features.is_empty() {
            verdicts += &format!("Features:\n  {}\n", post.features.join(" "));
        }
        if !post.excerpt.is_empty() {
            let kind = if content.is_manual_excerpt(&post.excerpt) {
                "the author's"
            } else {
                "made from the post"
            };
            verdicts += &format!("Excerpt: {kind}\n");
        }
        if !post.featured_image.is_empty() {
            let shown = content.includes_image(&post.featured_image);
            verdicts += &format!("Featured image in the post: {shown}\n");
        }
        (format!("{}.html", post.id), snapshot(&content, &verdicts))
    });
    compare(&test_data().join(snapshots), snapshots_for_posts);
}

/// Checks the cleanup leaves nothing it promises to remove in any post in
/// `test-data/reader/<fixtures>`.
fn posts_are_clean(fixtures: &str) {
    let problems: Vec<String> = posts(&test_data().join(fixtures))
        .iter()
        .flat_map(|post| cleanup_problems(&post.id, &post.content()))
        .collect();
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Compares each snapshot with the one in `directory`, or replaces it when updating.
fn compare(directory: &Path, snapshots: impl Iterator<Item = (String, String)>) {
    let update = env::var_os("UPDATE_SNAPSHOTS").is_some();
    if update {
        fs::create_dir_all(directory).expect("the snapshot directory can be created");
    }

    let mut failures = Vec::new();
    for (name, snapshot) in snapshots {
        let path = directory.join(&name);
        if update {
            fs::write(&path, snapshot).expect("the snapshot directory is writable");
            continue;
        }

        let Ok(expected) = fs::read_to_string(&path) else {
            failures.push(format!("{name}: no snapshot"));
            continue;
        };
        if let Some((line, (expected, actual))) = expected
            .lines()
            .zip(snapshot.lines())
            .enumerate()
            .find(|(_, (expected, actual))| expected != actual)
        {
            failures.push(format!(
                "{name}, line {}:\n  expected: {expected}\n  actual:   {actual}",
                line + 1
            ));
        } else if expected != snapshot {
            failures.push(format!("{name}: the length differs"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} post(s) don't match their snapshots. If the change is expected, run `{UPDATE_COMMAND}` and review the diff.\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// The cleaned post, with what was found in it in a comment at the top. It's still HTML, so it
/// opens in a browser.
fn snapshot(content: &ReaderPostContent, verdicts: &str) -> String {
    let mut found = String::new();
    let mut list = |title: &str, items: Vec<String>| {
        if !items.is_empty() {
            found += &format!("{title}:\n");
            for item in items {
                found += &format!("  {item}\n");
            }
        }
    };

    list("Scripts", content.scripts());
    list("Images", content.images().iter().map(describe).collect());
    for (number, gallery) in content.galleries().iter().enumerate() {
        let images = gallery
            .images
            .iter()
            .map(|image| image.url.clone())
            .collect();
        list(&format!("Gallery {}", number + 1), images);
    }
    let mentions = content.mentions().into_iter();
    list(
        "Mentions",
        mentions
            .map(|mention| format!("@{} {}", mention.username, mention.url))
            .collect(),
    );
    list(
        "Anchors",
        content.anchors().iter().map(describe_anchor).collect(),
    );

    format!(
        "<!-- Generated by `{UPDATE_COMMAND}`.\n{verdicts}{found}-->\n{}\n",
        content.html()
    )
}

fn describe_anchor(anchor: &ReaderPostAnchor) -> String {
    let kind = match anchor.kind {
        ReaderPostAnchorKind::Heading { level } => format!("heading {level}"),
        ReaderPostAnchorKind::Footnote => "footnote".to_owned(),
        ReaderPostAnchorKind::Other => "linked to".to_owned(),
    };
    match &anchor.text {
        Some(text) => format!("#{} · {kind} · “{text}”", anchor.id),
        None => format!("#{} · {kind}", anchor.id),
    }
}

fn describe(image: &ReaderPostImage) -> String {
    let mut description = image.url.clone();
    if let (Some(width), Some(height)) = (image.width, image.height) {
        description += &format!(" · {width}×{height}");
    }
    if !image.sizes.is_empty() {
        description += &format!(" · {} sizes", image.sizes.len());
    }
    if image.original_url.is_some() {
        description += " · original";
    }
    if let Some(caption) = &image.caption {
        description += &format!(" · “{caption}”");
    }
    description
}
