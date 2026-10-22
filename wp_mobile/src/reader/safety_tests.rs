//! Checks that nothing a post brings can run in the app's web view, or act on the page.
//!
//! Anyone who can publish a post the Reader shows controls its HTML, and the page runs `reader.js`
//! with a message handler into the app. So this checks what the cleanup produces, rather than what
//! it was given: it parses the cleaned post the way the web view will, and looks for anything that
//! runs a script, navigates somewhere other than a web page, or changes the page as a whole.
//!
//! [`safety_problems`] runs on every test post, on every case in `clean.rs`, and on the private
//! corpus. `test-data/reader/sanitizer` adds DOMPurify's test cases, which are written to get past
//! HTML sanitizers.

use std::{fs, path::Path};

use scraper::{ElementRef, Html, Selector};
use serde_json::Value;

use super::{
    ReaderBrand, ReaderPageOptions, ReaderPostContent, ReaderTheme, embeds, gists,
    reader_default_app_colors, reader_script,
};

/// A post's address, so the cases also go through resolving relative URLs.
const POST_URL: &str = "https://example.com/2026/10/post/";

/// Everything about `content` that would let the post run something in the app's web view.
pub(super) fn safety_problems(content: &ReaderPostContent) -> Vec<String> {
    let mut problems = Vec::new();
    let mut check = |what: &str, html: &str| {
        problems.extend(
            markup_problems(html)
                .into_iter()
                .map(|problem| format!("{what}: {problem}")),
        );
        // The page parses the post again. If it comes out differently, the checks above looked at
        // something other than what the web view will show.
        let reparsed = Html::parse_fragment(html).root_element().inner_html();
        if reparsed != html {
            problems.push(format!("{what}: parsing it again changes it"));
        }
    };
    check("the post", &content.html());
    check("the offline post", &content.offline_html());
    check(
        "the post with Photon sizes",
        &content.with_photon_sizes().html(),
    );

    for script in content.scripts() {
        let allowed = url::Url::parse(&script)
            .is_ok_and(|url| url.scheme() == "https" && embeds::is_allowed(&url));
        if !allowed {
            problems.push(format!("scripts: {script} isn't an allowed embed script"));
        }
    }
    problems.extend(page_problems(content));
    problems.extend(native_problems(content));
    problems
}

/// What in an HTML fragment could run a script or act on the page.
fn markup_problems(html: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let parsed = Html::parse_fragment(html);
    for element in parsed
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
    {
        let element = element.value();
        let tag = element.name();
        match tag {
            "script" | "style" | "noscript" | "base" | "link" => {
                problems.push(format!("a <{tag}>"));
            }
            // A `<meta itemprop>` is data about the post. Anything else can redirect the page.
            "meta"
                if element.attr("itemprop").is_none()
                    || element.attr("http-equiv").is_some()
                    || element.attr("charset").is_some() =>
            {
                problems.push("a <meta> that acts on the page".to_owned());
            }
            // SVG animations can set an attribute after the page has loaded, out of the cleanup's
            // reach.
            "animate" | "set" | "animateMotion" | "animateTransform"
                if element.attr("attributeName").is_some_and(|target| {
                    is_event_handler(target)
                        || is_url_attribute(target.trim_start_matches("xlink:"))
                }) =>
            {
                problems.push(format!("an SVG <{tag}> that changes a link or handler"));
            }
            // An iframe without a sandbox, or with one that allows it, can navigate the whole page
            // to a page of its own.
            "iframe"
                if element.attr("sandbox").is_none_or(|sandbox| {
                    sandbox.split_ascii_whitespace().any(|token| {
                        token
                            .to_ascii_lowercase()
                            .starts_with("allow-top-navigation")
                    })
                }) =>
            {
                problems.push("an <iframe> that can navigate the page".to_owned());
            }
            _ => {}
        }

        for (name, value) in element.attrs() {
            if is_event_handler(name) {
                problems.push(format!("an event handler, {name}, on <{tag}>"));
            } else if name == "srcdoc" {
                problems.push(format!("a srcdoc on <{tag}>"));
            } else if name == "style" && !is_cleanup_style(value) {
                problems.push(format!("an inline style on <{tag}>"));
            } else if name == "data-reader-gist" {
                // `reader.js` loads a script from it.
                let is_code_block =
                    tag == "figure" && element.classes().any(|class| class == "reader-gist");
                if !is_code_block || gists::gist_data(value).as_deref() != Some(value) {
                    problems.push(format!("a gist's data on <{tag}> that isn't a gist's"));
                }
            } else if is_url_attribute(name) {
                let urls: Vec<&str> = if name == "srcset" {
                    value
                        .split(',')
                        .filter_map(|candidate| candidate.split_whitespace().next())
                        .collect()
                } else {
                    vec![value]
                };
                for url in urls {
                    if let Some(scheme) = scheme(url)
                        && !is_allowed_scheme(tag, name, &scheme)
                    {
                        problems.push(format!("a {scheme}: URL in {name} on <{tag}>"));
                    }
                }
            }
        }
    }
    problems
}

/// The page around the post runs only the shared script and the allowed embed scripts, its policy
/// lets the shared script load the gists' data, and the post stays in the page's body.
fn page_problems(content: &ReaderPostContent) -> Vec<String> {
    let mut problems = Vec::new();
    for offline in [false, true] {
        let page = content.page_html(ReaderPageOptions {
            theme: ReaderTheme::Sepia,
            app_colors: reader_default_app_colors(ReaderBrand::Wordpress),
            language: None,
            offline,
            font: None,
            text_scale: 1.0,
            server_css: None,
            css: None,
        });
        let document = Html::parse_document(&page);
        let scripts = content.scripts();
        // An offline page loads no embed scripts, so its policy doesn't allow them.
        let loaded: Vec<String> = if offline {
            Vec::new()
        } else {
            scripts
                .iter()
                .cloned()
                .chain(gist_data(&document))
                .collect()
        };
        problems.extend(
            policy_problems(&document, &loaded)
                .into_iter()
                .map(|problem| format!("the page (offline: {offline}): {problem}")),
        );
        for script in document.select(&Selector::parse("script").expect("the selector is valid")) {
            let element = script.value();
            let is_ours = match element.attr("src") {
                Some(src) => !offline && scripts.iter().any(|script| script == src),
                None => script.text().collect::<String>() == reader_script(),
            };
            if !is_ours {
                problems.push(format!(
                    "the page (offline: {offline}): a script that isn't the shared one or an embed's"
                ));
            }
        }
        let post = if offline {
            content.offline_html()
        } else {
            content.html()
        };
        let body = document
            .select(&Selector::parse("body").expect("the selector is valid"))
            .next()
            .map(|body| body.inner_html());
        if !body.is_some_and(|body| body.starts_with(&format!("\n{post}\n"))) {
            problems.push(format!(
                "the page (offline: {offline}): the post isn't all in the body"
            ));
        }
    }
    problems
}

/// The addresses of the gists' data that `reader.js` loads, from their code blocks.
fn gist_data(document: &Html) -> Vec<String> {
    document
        .select(&Selector::parse("[data-reader-gist]").expect("the selector is valid"))
        .filter_map(|element| element.value().attr("data-reader-gist"))
        .map(str::to_owned)
        .collect()
}

/// The page's Content Security Policy comes before anything it covers, doesn't let inline scripts
/// or `eval` run, and lets each of the page's scripts load.
fn policy_problems(document: &Html, scripts: &[String]) -> Vec<String> {
    let selector = Selector::parse("head > *").expect("the selector is valid");
    let mut head = document.select(&selector);
    let first_with_content = head.find(|element| element.value().attr("charset").is_none());
    let Some(meta) = first_with_content.filter(|element| {
        element
            .value()
            .attr("http-equiv")
            .is_some_and(|name| name.eq_ignore_ascii_case("content-security-policy"))
    }) else {
        return vec!["the Content Security Policy isn't first in the <head>".to_owned()];
    };
    let policy = meta.value().attr("content").unwrap_or_default();
    let Some(script_src) = policy
        .split(';')
        .map(str::trim)
        .find_map(|directive| directive.strip_prefix("script-src "))
    else {
        return vec!["the policy has no script-src".to_owned()];
    };
    let sources: Vec<&str> = script_src.split_whitespace().collect();
    let mut problems = Vec::new();
    for source in &sources {
        if matches!(
            *source,
            "'unsafe-inline'" | "'unsafe-eval'" | "*" | "https:" | "http:"
        ) || source.contains('*')
        {
            problems.push(format!("the policy lets {source} scripts run"));
        }
    }
    for script in scripts {
        let origin = url::Url::parse(script)
            .ok()
            .and_then(|url| Some(format!("https://{}", url.host_str()?)));
        if !origin.is_some_and(|origin| sources.contains(&origin.as_str())) {
            problems.push(format!("the policy blocks {script}"));
        }
    }
    problems
}

/// The URLs the app gets for its own views, like the image viewer, go to web pages or images.
fn native_problems(content: &ReaderPostContent) -> Vec<String> {
    let mut problems = Vec::new();
    let mut check = |what: &str, url: &str, allowed: &[&str]| {
        if let Some(scheme) = scheme(url)
            && !allowed.contains(&scheme.as_str())
        {
            problems.push(format!("{what}: a {scheme}: URL"));
        }
    };
    let images = content.images().into_iter().chain(
        content
            .galleries()
            .into_iter()
            .flat_map(|gallery| gallery.images),
    );
    for image in images {
        for url in image.urls() {
            check("an image", url, &["http", "https", "data"]);
        }
        if let Some(link) = &image.link {
            check("an image's link", link, LINK_SCHEMES);
        }
    }
    for mention in content.mentions() {
        check("a mention", &mention.url, &["http", "https"]);
    }
    problems
}

/// The schemes a link can go to: web pages, email addresses and phone numbers.
const LINK_SCHEMES: &[&str] = &["http", "https", "mailto", "tel", "sms"];

/// Whether a URL with `scheme` is safe in the `attribute` of a `tag`.
fn is_allowed_scheme(tag: &str, attribute: &str, scheme: &str) -> bool {
    match (tag, attribute) {
        ("a" | "area", "href") => LINK_SCHEMES.contains(&scheme),
        // Images and media can't run scripts, even SVG ones, and posts have real `data:` images.
        ("img" | "source", "src" | "srcset")
        | ("audio" | "video" | "track", "src")
        | ("video", "poster")
        | ("image", "href") => {
            matches!(scheme, "http" | "https" | "data")
        }
        _ => matches!(scheme, "http" | "https"),
    }
}

/// Attributes a browser loads or navigates to. `xlink:href` comes out of the parser as `href`.
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

/// A URL's scheme, lowercased, the way a browser reads it, or `None` for a relative URL. Browsers
/// ignore leading and trailing spaces and control characters, and tabs and line breaks anywhere.
fn scheme(url: &str) -> Option<String> {
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

/// Whether an inline style is one the cleanup writes: iframe sizing, or a tiled gallery column's
/// width. Those are sizes and nothing else.
fn is_cleanup_style(style: &str) -> bool {
    style
        .split(';')
        .map(str::trim)
        .filter(|declaration| !declaration.is_empty())
        .all(|declaration| {
            let Some((property, value)) = declaration.split_once(':') else {
                return false;
            };
            let is_size = value.split_whitespace().all(|part| {
                part == "auto"
                    || part == "/"
                    || part
                        .trim_end_matches(['%'])
                        .trim_end_matches("px")
                        .parse::<f64>()
                        .is_ok_and(f64::is_finite)
            });
            matches!(
                property.trim(),
                "width" | "max-width" | "height" | "aspect-ratio" | "flex-grow"
            ) && is_size
        })
}

/// DOMPurify's test cases: payloads written to get past HTML sanitizers, from
/// `test-data/reader/sanitizer/dompurify.json`.
#[test]
fn dompurify_cases_are_safe() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-data/reader/sanitizer/dompurify.json");
    let json: Value = serde_json::from_str(&fs::read_to_string(&path).expect("the cases exist"))
        .expect("the cases are JSON");
    let cases = json["cases"].as_array().expect("the cases are a list");
    assert!(!cases.is_empty());

    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let payload = case["payload"].as_str().expect("each case has a payload");
        for url in [None, Some(POST_URL.to_owned())] {
            let with_url = url.is_some();
            let content = ReaderPostContent::new(payload, url);
            for problem in safety_problems(&content) {
                failures.push(format!(
                    "case {index} ({}), with a post URL: {with_url}: {problem}",
                    case["title"].as_str().unwrap_or_default()
                ));
            }
            let again = ReaderPostContent::new(&content.html(), Some(POST_URL.to_owned()));
            for problem in safety_problems(&again) {
                failures.push(format!("case {index}, cleaned twice: {problem}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

mod checker_tests {
    use super::*;
    use rstest::rstest;

    // The checker itself has to notice each kind of problem, or the tests that use it prove nothing.
    #[rstest]
    #[case::script("<script>x()</script>")]
    #[case::svg_script("<svg><script>x()</script></svg>")]
    #[case::style("<style>p {}</style>")]
    #[case::base(r#"<base href="https://example.org/">"#)]
    #[case::refresh(r#"<meta http-equiv="refresh" content="0">"#)]
    #[case::refresh_with_itemprop(r#"<meta itemprop="a" http-equiv="refresh" content="0">"#)]
    #[case::event_handler(r#"<p onclick="x()">Hi</p>"#)]
    #[case::script_link(r#"<a href="javascript:x()">Hi</a>"#)]
    #[case::script_link_with_a_tab("<a href=\"java\tscript:x()\">Hi</a>")]
    #[case::script_form_action(r#"<form action="javascript:x()"></form>"#)]
    #[case::data_iframe(r#"<iframe sandbox="" src="data:text/html,Hi"></iframe>"#)]
    #[case::srcdoc(r#"<iframe sandbox="" srcdoc="Hi"></iframe>"#)]
    #[case::iframe_without_a_sandbox(r#"<iframe src="https://example.com/"></iframe>"#)]
    #[case::iframe_that_can_navigate_the_page(
        r#"<iframe sandbox="allow-scripts Allow-Top-Navigation-By-User-Activation" src="https://example.com/"></iframe>"#
    )]
    #[case::data_link(r#"<a href="data:text/html,Hi">Hi</a>"#)]
    #[case::svg_animation(r#"<svg><a><set attributeName="href" to="javascript:x()"/></a></svg>"#)]
    #[case::style_with_a_url(r#"<p style="background: url(a.png)">Hi</p>"#)]
    #[case::gist_data_from_another_host(
        r#"<figure class="reader-gist" data-reader-gist="https://example.com/a.json"></figure>"#
    )]
    #[case::gist_data_off_a_code_block(
        r#"<div data-reader-gist="https://gist.github.com/octocat/6cad326836d38bd3a7ae.json"></div>"#
    )]
    fn notices_unsafe_markup(#[case] html: &str) {
        assert!(!markup_problems(html).is_empty(), "{html}");
    }

    #[rstest]
    #[case::text("<p>Hi</p>")]
    #[case::links(r##"<a href="https://example.com/">A</a><a href="/b">B</a><a href="#c">C</a><a href="mailto:a@example.com">D</a>"##)]
    #[case::data_image(r#"<img src="data:image/png;base64,iVBORw0KGgo=">"#)]
    #[case::iframe_sizing(
        r#"<iframe sandbox="" src="https://example.com/" style="width: 100%; max-width: 640px; height: auto; aspect-ratio: 640 / 360;"></iframe>"#
    )]
    #[case::sandboxed_iframe(
        r#"<iframe sandbox="allow-scripts allow-same-origin allow-popups" src="https://example.com/"></iframe>"#
    )]
    #[case::tile_width(r#"<div style="flex-grow: 62.5;"></div>"#)]
    #[case::microdata(r#"<meta itemprop="width" content="300">"#)]
    #[case::gist_code_block(
        r#"<figure class="reader-gist" data-reader-gist="https://gist.github.com/octocat/6cad326836d38bd3a7ae.json?file=a.rb"><pre><code>x</code></pre></figure>"#
    )]
    #[case::url_with_a_colon_later(r#"<a href="/a:b">Hi</a>"#)]
    fn accepts_safe_markup(#[case] html: &str) {
        assert_eq!(markup_problems(html), Vec::<String>::new());
    }
}
