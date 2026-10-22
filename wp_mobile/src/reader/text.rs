use scraper::{ElementRef, Html, Node};

/// Elements whose text isn't part of the post. The Reader hides forms, `<noscript>` holds markup as
/// text, and a script's or style's text is code. The cleanup removes most of these from a post, but
/// not from its excerpt.
const NOT_TEXT: &[&str] = &[
    "script", "style", "noscript", "template", "form", "button", "select", "option", "textarea",
    "input",
];

/// Classes of what the stylesheet hides: sharing buttons, related posts, ads, feed leftovers, search
/// boxes, dropdowns and contact forms. The Reader API sometimes sends an excerpt that's only these.
const HIDDEN_CLASSES: &[&str] = &[
    "feedflare",
    "sharedaddy",
    "jp-relatedposts",
    "jp-relatedposts-headline",
    "mc4wp-form",
    "wpcnt",
    "OUTBRAIN",
    "adsbygoogle",
    "wp-block-cover-image",
    "wp-block-search",
    "wp-block-search__label",
    "wp-block-search__inside-wrapper",
    "wp-block-archives-dropdown",
    "wp-block-categories-dropdown",
    "jetpack-contact-form-container",
];

/// Whether the Reader doesn't show an element's text.
fn is_hidden(element: &scraper::node::Element) -> bool {
    NOT_TEXT.contains(&element.name())
        || element
            .classes()
            .any(|class| HIDDEN_CLASSES.contains(&class))
}

/// Elements that start a new line of text.
const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "dd",
    "details",
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
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "td",
    "th",
    "tr",
    "ul",
];

/// The post's text, with a line break between blocks like paragraphs and list items.
pub(super) fn plain_text(root: ElementRef) -> String {
    text_without(root, |_| false)
}

/// The text WordPress makes an excerpt from: the post's text, without the blocks it leaves out of
/// excerpts, like images and their captions, embeds, and Jetpack's blocks.
pub(super) fn excerpt_source_text(root: ElementRef) -> String {
    text_without(root, |element| {
        element.name() == "figcaption"
            || element.classes().any(|class| {
                matches!(
                    class,
                    "wp-block-image"
                        | "wp-block-gallery"
                        | "wp-block-embed"
                        | "wp-block-video"
                        | "wp-block-audio"
                        | "wp-block-cover"
                        | "wp-block-buttons"
                        | "wp-block-file"
                        | "wp-caption"
                        | "jetpack-blogging-prompt"
                ) || class.starts_with("wp-block-jetpack-")
            })
    })
}

/// The text of `root`, without the text of elements `skip` matches.
fn text_without(root: ElementRef, skip: impl Fn(&scraper::node::Element) -> bool) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current_block = None;
    let mut new_line = true;

    for node in root.descendants() {
        match node.value() {
            Node::Element(element) if element.name() == "br" => new_line = true,
            Node::Text(text) => {
                let mut ancestors = node.ancestors().filter_map(ElementRef::wrap);
                if ancestors
                    .clone()
                    .any(|ancestor| is_hidden(ancestor.value()) || skip(ancestor.value()))
                {
                    continue;
                }
                let block = ancestors
                    .find(|ancestor| BLOCKS.contains(&ancestor.value().name()))
                    .map(|block| block.id());
                if block != current_block {
                    current_block = block;
                    new_line = true;
                }
                if new_line && !text.trim().is_empty() {
                    lines.push(String::new());
                    new_line = false;
                }
                if let Some(line) = lines.last_mut() {
                    line.push_str(text);
                }
            }
            _ => {}
        }
    }

    lines
        .iter()
        .map(|line| collapse_whitespace(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Elements whose text [`simplified_html`] keeps.
const SIMPLIFIED: &[&str] = &[
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "p",
    "blockquote",
    "li",
    "pre",
    "code",
    "figcaption",
    "td",
    "th",
    "img",
];

/// The post's headings, paragraphs and other text, one element to a line, with images' text
/// alternatives. The markup in between – wrappers, spans, links, styles – is gone.
pub(super) fn simplified_html(root: ElementRef) -> String {
    root.descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| SIMPLIFIED.contains(&element.value().name()))
        // An element inside another one is already in that one's text.
        .filter(|element| {
            !element
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|ancestor| SIMPLIFIED.contains(&ancestor.value().name()))
        })
        .filter_map(|element| match element.value().name() {
            "img" => element
                .value()
                .attr("alt")
                .map(str::trim)
                .filter(|alt| !alt.is_empty())
                .map(|alt| format!(r#"<img alt="{}">"#, alt.replace('"', "&quot;"))),
            name => {
                let text = collapse_whitespace(&plain_text(element));
                (!text.is_empty()).then(|| format!("<{name}>{text}</{name}>"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `excerpt` is one the author wrote, rather than one WordPress made by shortening the post.
pub(super) fn is_manual_excerpt(excerpt: &str, post_text: &str) -> bool {
    let excerpt = plain_text(Html::parse_fragment(excerpt).root_element());
    // A site's own shortcodes, which the Reader API can't run, are left in both.
    let excerpt = collapse_whitespace(&without_shortcodes(&excerpt));
    // WordPress ends the excerpts it makes with `[…]`, and some themes with `…`.
    let shortened = ["…", "[…]", "[...]"];
    if excerpt.is_empty() || shortened.iter().any(|end| excerpt.ends_with(end)) {
        return false;
    }
    // An excerpt WordPress made repeats the start of the post. Only its own start counts: its end
    // can differ, where WordPress cut a word or a sentence. 40 characters without spaces is about
    // the 50 with spaces the apps compared before.
    //
    // WordPress strips tags without leaving spaces between blocks, so spaces don't count, and it
    // leaves out shortcodes.
    let start: String = without_whitespace(&excerpt).chars().take(40).collect();
    !without_whitespace(&without_shortcodes(post_text)).starts_with(&start)
}

/// How long the post takes to read, in whole minutes, rounded up: 200 words a minute, the speed the
/// Android app has used. Chinese and Japanese don't put spaces between words, so their characters
/// count instead, at 500 a minute. Images don't count. `None` for a post without text.
pub(super) fn reading_time_minutes(post_text: &str) -> Option<u32> {
    const WORDS_PER_MINUTE: f64 = 200.0;
    const CHARACTERS_PER_MINUTE: f64 = 500.0;

    let mut words = 0_u32;
    let mut characters = 0_u32;
    for token in post_text.split_whitespace() {
        characters += token.chars().filter(|c| is_unspaced_script(*c)).count() as u32;
        if token
            .chars()
            .any(|c| c.is_alphanumeric() && !is_unspaced_script(c))
        {
            words += 1;
        }
    }
    if words == 0 && characters == 0 {
        return None;
    }
    let minutes =
        f64::from(words) / WORDS_PER_MINUTE + f64::from(characters) / CHARACTERS_PER_MINUTE;
    Some((minutes.ceil() as u32).max(1))
}

/// Chinese characters and Japanese kana, which aren't written with spaces between words. Korean is.
fn is_unspaced_script(c: char) -> bool {
    matches!(
        c,
        '\u{3040}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
}

/// Whether the post is only the start of the real one, the way some sites' feeds send it: it ends
/// with WordPress's "Continue reading" link (`a.more-link`), or it's as short as the summary
/// WordPress makes for a feed and ends with the `[…]` WordPress ends it with.
///
/// Across 284,159 real posts, the link only ends teasers. `[…]` also ends long posts that quote
/// part of an article, and the excerpts in a Latest Posts block, so those don't count. Nor does a
/// post that just ends with `…`: posts end that way too.
pub(super) fn is_cut_off(root: ElementRef) -> bool {
    // WordPress's feed summaries are 55 words, unless a site changes that.
    const SUMMARY_WORDS: usize = 150;

    let text = plain_text(root);
    let text = text.trim_end();
    let own_text = text_without(root, |element| {
        element
            .classes()
            .any(|class| class == "wp-block-latest-posts")
    });
    let own_text = own_text.trim_end();
    if ["[…]", "[...]"].iter().any(|end| own_text.ends_with(end))
        && own_text.split_whitespace().count() <= SUMMARY_WORDS
    {
        return true;
    }

    let Some(more_link) = root
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| {
            element.value().name() == "a"
                && element.value().classes().any(|class| class == "more-link")
        })
        .last()
    else {
        return false;
    };
    // Nothing that reads as text comes after the link.
    let link_text = collapse_whitespace(&more_link.text().collect::<String>());
    !link_text.is_empty()
        && text
            .trim_end_matches(|c: char| !c.is_alphanumeric())
            .ends_with(link_text.trim_end_matches(|c: char| !c.is_alphanumeric()))
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Text without WordPress shortcodes, like `[caption id="1"]A photo[/caption]`. A shortcode that
/// closes takes its content with it.
fn without_shortcodes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        result.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let name_end = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        let name = &after[..name_end];
        // Shortcodes have lowercase names, unlike `[Bracketed text]`.
        let is_shortcode = name.starts_with(|c: char| c.is_ascii_lowercase())
            && !name.chars().any(|c| c.is_ascii_uppercase());
        let Some(close) = after.find(']').filter(|_| is_shortcode) else {
            result.push('[');
            rest = after;
            continue;
        };
        rest = &after[close + 1..];
        if let Some(end) = rest.find(&format!("[/{name}]")) {
            rest = &rest[end + name.len() + 3..];
        }
    }
    result.push_str(rest);
    result
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use crate::reader::ReaderPostContent;
    use rstest::rstest;

    #[rstest]
    #[case::paragraphs("<p>One</p><p>Two <em>and</em> three</p>", "One\nTwo and three")]
    #[case::line_breaks("<p>One<br>Two</p>", "One\nTwo")]
    #[case::text_after_a_block("<div>Before<p>Inside</p>After</div>", "Before\nInside\nAfter")]
    #[case::lists("<ul><li>One</li><li>Two</li></ul>", "One\nTwo")]
    #[case::whitespace("<p>  Lots   of\n space  </p>", "Lots of space")]
    #[case::characters("<p>Don&#8217;t &amp; won&rsquo;t</p>", "Don’t & won’t")]
    #[case::form_controls(
        "<p>Archives</p><select><option>May 2009</option></select>",
        "Archives"
    )]
    #[case::noscript("<p>Text</p><noscript><img src=\"a.jpg\"></noscript>", "Text")]
    #[case::form(
        "<p>Text</p><form><p>Enter your email address to subscribe.</p><label>Email Address:</label><input type=\"email\"></form>",
        "Text"
    )]
    #[case::sharing_buttons(
        r#"<p>Text</p><div class="sharedaddy sd-sharing-enabled"><h3 class="sd-title">Share this:</h3><span class="sharing-screen-reader-text">Click to share on Facebook</span></div>"#,
        "Text"
    )]
    // As the Reader API sends it for a Jetpack site, without its form and text box.
    #[case::search_block_without_its_form(
        r#"<p>Text</p><label class="wp-block-search__label" for="wp-block-search__input-4">Search</label><div class="wp-block-search__inside-wrapper"><button aria-label="Search" class="wp-block-search__button wp-element-button" type="submit">Search</button></div>"#,
        "Text"
    )]
    fn plain_text(#[case] html: &str, #[case] expected: &str) {
        assert_eq!(ReaderPostContent::new(html, None).plain_text(), expected);
    }

    #[test]
    fn simplified_html() {
        let content = ReaderPostContent::new(
            r#"<div class="wrapper"><h2 style="color: blue">Title</h2></div>
            <p>Content with <span>formatting</span>.</p>
            <figure><img src="a.jpg" alt="A &quot;photo&quot;"><figcaption>Caption</figcaption></figure>
            <p><img src="b.jpg" alt="Inside a paragraph"></p>
            <ul><li>One</li><li><p>Two</p></li></ul>
            <img src="c.jpg">"#,
            None,
        );
        assert_eq!(
            content.simplified_html(),
            [
                "<h2>Title</h2>",
                "<p>Content with formatting.</p>",
                r#"<img alt="A &quot;photo&quot;">"#,
                "<figcaption>Caption</figcaption>",
                "<li>One</li>",
                "<li>Two</li>",
            ]
            .join("\n")
        );
    }

    const POST: &str =
        "<p>The first paragraph of the post, which goes on for a while.</p><p>The second.</p>";

    #[rstest]
    #[case::written("A summary the author wrote.", true)]
    #[case::html("<p>A summary the author <em>wrote</em>.</p>", true)]
    #[case::shortened_post(
        "The first paragraph of the post, which goes on for a while. The [&hellip;]",
        false
    )]
    #[case::shortened_differently(
        "The first paragraph of the post, which goes on for a lot longer",
        false
    )]
    #[case::ellipsis("Something else entirely…", false)]
    #[case::wordpress_ending("Something else entirely [&hellip;]", false)]
    #[case::blocks_run_together(
        "The first paragraph of the post, which goes on for a while.The second.",
        false
    )]
    #[case::empty("  ", false)]
    // Excerpts the Reader API sends with only what plugins add after a post.
    #[case::only_likes_and_related_posts(
        r#"<div class='sharedaddy sd-block sd-like jetpack-likes-widget-wrapper'><h3 class='sd-title'>Like this:</h3><div class='likes-widget-placeholder'><span class='button'><span>Like</span></span> <span class="loading">Loading...</span></div></div>
        <div id='jp-relatedposts' class='jp-relatedposts'><h3 class="jp-relatedposts-headline"><em>Related</em></h3></div>"#,
        false
    )]
    #[case::only_ads_and_a_signup_form(
        r#"<div class='code-block code-block-1'><br><div class="OUTBRAIN" data-widget-id="TF_1"></div>
        <script async src="//pagead2.googlesyndication.com/pagead/js/adsbygoogle.js"></script><ins class="adsbygoogle"></ins>
        <script>(adsbygoogle = window.adsbygoogle || []).push({});</script></div>
        [jetpack-related-posts]
        <div class="form mc4wp-form"><form method="post"><p>Subscribe to our blog.</p><input type="submit" value="Subscribe"></form></div>"#,
        false
    )]
    #[case::written_with_sharing_buttons(
        r#"<p>A summary the author wrote.</p><div class="sharedaddy"><h3>Share this:</h3></div>"#,
        true
    )]
    fn knows_a_manual_excerpt(#[case] excerpt: &str, #[case] manual: bool) {
        assert_eq!(
            ReaderPostContent::new(POST, None).is_manual_excerpt(excerpt),
            manual
        );
    }

    #[test]
    fn finds_an_excerpt_after_a_writing_prompt() {
        let content = ReaderPostContent::new(
            r#"<div class="jetpack-blogging-prompt">Daily writing prompt What do you listen to while you work? View all responses</div>
            <p>I generally listen to a continuous stream of requests for past manuals.</p>"#,
            None,
        );
        assert!(!content.is_manual_excerpt(
            "I generally listen to a continuous stream of requests for past manuals."
        ));
    }

    #[test]
    fn leaves_shortcodes_out_of_the_comparison() {
        let content = ReaderPostContent::new(
            r#"<p>[caption id="attachment_1" align="aligncenter"]<img src="a.jpg"> A photo[/caption] Check out these photos from the show last Saturday!</p>"#,
            None,
        );
        assert!(!content.is_manual_excerpt("Check out these photos from the show last Saturday!"));
    }

    #[rstest]
    #[case::closing("a [caption id=\"1\"]b[/caption] c", "a  c")]
    #[case::self_closing("a [gallery ids=\"1,2\"] c", "a  c")]
    #[case::not_a_shortcode("see [1] and [ok", "see [1] and [ok")]
    #[case::bracketed_text("Cooking Dash [Game Download]", "Cooking Dash [Game Download]")]
    fn removes_shortcodes(#[case] text: &str, #[case] expected: &str) {
        assert_eq!(super::without_shortcodes(text), expected);
    }

    #[rstest]
    #[case::short("<p>A few words.</p>", Some(1))]
    #[case::two_hundred_words(&format!("<p>{}</p>", "word ".repeat(200)), Some(1))]
    #[case::rounds_up(&format!("<p>{}</p>", "word ".repeat(201)), Some(2))]
    #[case::images_dont_count(
        r#"<p>Two words</p><img src="a.jpg" alt="A long description of the image">"#,
        Some(1)
    )]
    #[case::japanese(&format!("<p>{}</p>", "日本語の文章です。".repeat(100)), Some(2))]
    #[case::no_text(r#"<img src="a.jpg">"#, None)]
    #[case::only_punctuation("<p>— …</p>", None)]
    fn knows_how_long_a_post_takes_to_read(#[case] html: &str, #[case] minutes: Option<u32>) {
        assert_eq!(
            ReaderPostContent::new(html, None).reading_time_minutes(),
            minutes
        );
    }

    #[rstest]
    #[case::continue_reading_link(
        r#"<p>The start of the post … <a href="https://example.com/post/" class="more-link">Continue reading<span class="screen-reader-text"> "The post"</span></a></p>"#,
        true
    )]
    #[case::wordpress_ending("<p>The start of the post [&hellip;]</p>", true)]
    #[case::three_dots_ending("<p>The start of the post [...]</p>", true)]
    #[case::more_link_then_more_post(
        r#"<p><a href="https://example.com/post/#more-1" class="more-link">Continue reading</a></p><p>The rest of the post.</p>"#,
        false
    )]
    #[case::another_read_more_link(
        r#"<p>Buy it now. <a href="https://example.com/product" class="read-more-link">Read more…</a></p>"#,
        false
    )]
    #[case::long_post_quoting_an_article(
        &format!("<p>{}</p><p>The bad news is that it ends here. […]</p>", "word ".repeat(200)),
        false
    )]
    #[case::latest_posts_block(
        r#"<p>The whole post.</p><ul class="wp-block-latest-posts"><li><a href="https://example.com/a/">Another post</a><div class="wp-block-latest-posts__post-excerpt">Its start [&hellip;]</div></li></ul>"#,
        false
    )]
    #[case::ellipsis("<p>And then he said…</p>", false)]
    #[case::whole_post("<p>The whole post.</p>", false)]
    fn knows_a_post_is_cut_off(#[case] html: &str, #[case] cut_off: bool) {
        assert_eq!(ReaderPostContent::new(html, None).is_cut_off(), cut_off);
    }

    /// The text leaves out what the page doesn't show, so the two lists have to agree.
    #[test]
    fn the_stylesheet_hides_every_hidden_class() {
        let stylesheet = include_str!("reader.css");
        for class in super::HIDDEN_CLASSES {
            assert!(
                stylesheet.contains(&format!(".{class},"))
                    || stylesheet.contains(&format!(".{class} {{")),
                "reader.css doesn't hide .{class}"
            );
        }
    }
}
