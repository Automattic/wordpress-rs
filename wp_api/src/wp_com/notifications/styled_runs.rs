//! A note's text and ranges as [`StyledTextRun`]s.

use super::{
    NotificationBlock, NotificationMedia, NotificationMediaKind, NotificationRange,
    NotificationRangeKind, NotificationText, lenient::FromId,
};
use crate::{
    comments::CommentId,
    posts::PostId,
    wp_com::{
        WpComSiteId,
        me::WpComUserId,
        styled_text::{
            ListKind, StyledText, StyledTextRun, TextMark, TextRunAttachment, TextRunLink,
            TextRunStyle, styled_runs,
        },
    },
};

/// Turns a note's subject, header or context summary into runs to show, in the order of its
/// text.
///
/// The runs' text put together is the line's whole text. Ranges and media whose positions don't
/// fit the text don't fail: they're cut at the end of the text, and an image or icon placed past
/// the end goes at the end.
#[uniffi::export]
pub fn notification_text_styled_runs(text: &NotificationText) -> Vec<StyledTextRun> {
    notification_styled_text(&text.text, &text.ranges, &text.media).runs
}

/// Turns a body block's text into runs to show, in the order of its text.
///
/// The runs' text put together is the block's whole text. Ranges and media whose positions
/// don't fit the text don't fail: they're cut at the end of the text, and an image or icon
/// placed past the end goes at the end.
#[uniffi::export]
pub fn notification_block_styled_runs(block: &NotificationBlock) -> Vec<StyledTextRun> {
    notification_styled_text(&block.text, &block.ranges, &block.media).runs
}

pub(super) fn notification_styled_text(
    text: &str,
    ranges: &[NotificationRange],
    media: &[NotificationMedia],
) -> StyledText {
    let mut nothing_to_show = 0;
    // Ranges come before media, which matters where they tie.
    let mut marks: Vec<TextMark> = ranges
        .iter()
        .map(|range| range_mark(range, &mut nothing_to_show))
        .collect();
    marks.extend(
        media
            .iter()
            .map(|medium| media_mark(medium, &mut nothing_to_show)),
    );
    let mut styled = styled_runs(text, &marks);
    styled.adjustments.nothing_to_show += nothing_to_show;
    styled
}

fn range_mark(range: &NotificationRange, nothing_to_show: &mut u64) -> TextMark {
    use NotificationRangeKind as Kind;

    let mut mark = TextMark {
        indices: range.indices.map(|indices| (indices.start, indices.end)),
        id: range.id.clone(),
        parent: range.parent.clone(),
        ..Default::default()
    };
    let web = || range.url.clone().map(|url| TextRunLink::Web { url });
    let Some(kind) = &range.kind else {
        mark.link = web();
        return mark;
    };
    match kind {
        Kind::Bold => mark.style = Some(TextRunStyle::Bold),
        Kind::Italic | Kind::Emphasis => mark.style = Some(TextRunStyle::Italic),
        Kind::Strikethrough | Kind::Deleted => mark.style = Some(TextRunStyle::Strikethrough),
        Kind::Superscript => mark.style = Some(TextRunStyle::Superscript),
        Kind::Subscript => mark.style = Some(TextRunStyle::Subscript),
        Kind::Code => mark.style = Some(TextRunStyle::Code),
        Kind::Preformatted => mark.style = Some(TextRunStyle::Preformatted),
        Kind::Blockquote => mark.style = Some(TextRunStyle::Quote),
        Kind::Citation => mark.style = Some(TextRunStyle::Citation),
        Kind::Heading1 => mark.style = Some(TextRunStyle::Heading { level: 1 }),
        Kind::Heading2 => mark.style = Some(TextRunStyle::Heading { level: 2 }),
        Kind::Heading3 => mark.style = Some(TextRunStyle::Heading { level: 3 }),
        Kind::Heading4 => mark.style = Some(TextRunStyle::Heading { level: 4 }),
        Kind::Heading5 => mark.style = Some(TextRunStyle::Heading { level: 5 }),
        Kind::Heading6 => mark.style = Some(TextRunStyle::Heading { level: 6 }),
        Kind::OrderedList => mark.list = Some(ListKind::Numbered),
        Kind::UnorderedList => mark.list = Some(ListKind::Bulleted),
        Kind::ListItem => mark.is_list_item = true,
        Kind::LineBreak => mark.attachment = Some(TextRunAttachment::LineBreak),
        Kind::HorizontalRule => mark.attachment = Some(TextRunAttachment::Separator),
        Kind::Noticon => match &range.value {
            Some(character) => {
                mark.attachment = Some(TextRunAttachment::Icon {
                    character: character.clone(),
                });
            }
            None => *nothing_to_show += 1,
        },
        Kind::Match => {
            mark.style = Some(TextRunStyle::Highlight);
            mark.link = web();
        }
        Kind::Button => {
            mark.style = Some(TextRunStyle::Button);
            mark.link = web();
        }
        Kind::User => {
            let user_id = id::<WpComUserId>(range);
            if user_id.is_some() || range.url.is_some() {
                mark.link = Some(TextRunLink::User {
                    user_id,
                    site_id: range.site_id,
                    url: range.url.clone(),
                });
            }
        }
        Kind::Site => {
            let site_id = id::<WpComSiteId>(range).or(range.site_id);
            if site_id.is_some() || range.url.is_some() {
                mark.link = Some(TextRunLink::Site {
                    site_id,
                    url: range.url.clone(),
                });
            }
        }
        Kind::Post | Kind::Page => {
            let post_id = id::<PostId>(range);
            if post_id.is_some() || range.url.is_some() {
                mark.link = Some(TextRunLink::Post {
                    site_id: range.site_id,
                    post_id,
                    url: range.url.clone(),
                });
            }
        }
        Kind::Comment => {
            let comment_id = id::<CommentId>(range);
            if comment_id.is_some() || range.url.is_some() {
                mark.link = Some(TextRunLink::Comment {
                    site_id: range.site_id,
                    post_id: range.post_id,
                    comment_id,
                    url: range.url.clone(),
                });
            }
        }
        Kind::Stat
        | Kind::Scan
        | Kind::Follow
        | Kind::Like
        | Kind::Media
        | Kind::Link
        | Kind::Anchor
        | Kind::Tracks
        | Kind::RewindDownloadReady
        | Kind::Other(_) => mark.link = web(),
    }
    mark
}

fn media_mark(medium: &NotificationMedia, nothing_to_show: &mut u64) -> TextMark {
    let mut mark = TextMark {
        indices: medium.indices.map(|indices| (indices.start, indices.end)),
        id: medium.id.clone(),
        parent: medium.parent.clone(),
        ..Default::default()
    };
    match &medium.url {
        Some(url) => {
            mark.attachment = Some(TextRunAttachment::Image {
                url: url.clone(),
                width: pixels(medium.width.as_deref()),
                height: pixels(medium.height.as_deref()),
                is_emoji: medium.emoji == Some(true),
                is_badge: medium.kind == Some(NotificationMediaKind::Badge),
            });
        }
        None => *nothing_to_show += 1,
    }
    mark
}

/// The ID in a person, site, post or comment range's `id`.
fn id<T: FromId>(range: &NotificationRange) -> Option<T> {
    range
        .id
        .as_deref()
        .and_then(|id| id.trim().parse::<u64>().ok())
        .filter(|id| *id != 0)
        .and_then(T::from_id)
}

/// A number of pixels, such as `"256"`. Anything else, such as `"100%"`, is `None`.
fn pixels(size: Option<&str>) -> Option<u32> {
    size.and_then(|size| size.trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn block(value: serde_json::Value) -> NotificationBlock {
        serde_json::from_value(value).expect("the block parses")
    }

    fn runs(value: serde_json::Value) -> Vec<StyledTextRun> {
        let block = block(value);
        let runs = notification_block_styled_runs(&block);
        let text: String = runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(text, block.text, "The runs' text is the block's text");
        runs
    }

    fn link(range: serde_json::Value) -> Option<TextRunLink> {
        let mut range = range;
        range["indices"] = json!([0, 4]);
        let runs = runs(json!({ "text": "abcd", "ranges": [range] }));
        assert_eq!(runs.len(), 1);
        runs[0].link.clone()
    }

    #[test]
    fn test_a_subject_with_a_noticon_and_a_person() {
        let subject: NotificationText = serde_json::from_value(json!({
            "text": "Sam Example liked your post",
            "ranges": [
                { "type": "noticon", "indices": [0, 0], "value": "\u{f408}" },
                {
                    "type": "user", "indices": [0, 11], "id": 701, "site_id": 801,
                    "url": "https://sam.example.org"
                },
                { "type": "post", "indices": [23, 27], "id": "601", "site_id": "802" }
            ]
        }))
        .unwrap();
        let runs = notification_text_styled_runs(&subject);
        let summary: Vec<_> = runs
            .iter()
            .map(|run| {
                (
                    run.text.as_str(),
                    run.link.is_some(),
                    run.attachment.clone(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "",
                    false,
                    Some(TextRunAttachment::Icon {
                        character: "\u{f408}".to_string()
                    })
                ),
                ("Sam Example", true, None),
                (" liked your ", false, None),
                ("post", true, None),
            ]
        );
        assert_eq!(
            runs[1].link,
            Some(TextRunLink::User {
                user_id: Some(WpComUserId(701)),
                site_id: Some(WpComSiteId(801)),
                url: Some("https://sam.example.org".to_string()),
            })
        );
        assert_eq!(
            runs[3].link,
            Some(TextRunLink::Post {
                site_id: Some(WpComSiteId(802)),
                post_id: Some(PostId(601)),
                url: None,
            })
        );
    }

    #[test]
    fn test_a_noticon_without_a_character_is_left_out() {
        let text = notification_styled_text(
            "abc",
            &block(json!({ "text": "abc", "ranges": [{ "type": "noticon", "indices": [0, 0] }] }))
                .ranges,
            &[],
        );
        assert_eq!(text.runs.len(), 1);
        assert_eq!(text.adjustments.nothing_to_show, 1);
    }

    #[test]
    fn test_a_comment_with_nested_markup() {
        // A paragraph holding bold text with a link inside it, then a quote, then a list.
        let text = "Nice post! See this.\nQuoted\nfirst\nsecond\n";
        let runs = runs(json!({
            "text": text,
            "type": "comment",
            "ranges": [
                { "type": "p", "indices": [0, 21], "id": 1 },
                { "type": "b", "indices": [0, 10], "id": 2, "parent": 1 },
                { "type": "a", "indices": [15, 19], "id": 3, "parent": 1, "url": "https://example.com/this" },
                { "type": "em", "indices": [15, 19], "id": 4, "parent": 3 },
                { "type": "blockquote", "indices": [21, 28], "id": 5 },
                { "type": "ol", "indices": [28, 41], "id": 6 },
                { "type": "li", "indices": [28, 34], "id": 7, "parent": 6 },
                { "type": "li", "indices": [34, 41], "id": 8, "parent": 6 }
            ]
        }));
        let summary: Vec<_> = runs
            .iter()
            .map(|run| (run.text.as_str(), run.styles.clone(), run.link.is_some()))
            .collect();
        assert_eq!(
            summary,
            [
                ("Nice post!", vec![TextRunStyle::Bold], false),
                (" See ", vec![], false),
                ("this", vec![TextRunStyle::Italic], true),
                (".\n", vec![], false),
                ("Quoted\n", vec![TextRunStyle::Quote], false),
                (
                    "first\n",
                    vec![TextRunStyle::ListItem { number: Some(1) }],
                    false
                ),
                (
                    "second\n",
                    vec![TextRunStyle::ListItem { number: Some(2) }],
                    false
                ),
            ]
        );
    }

    #[test]
    fn test_html_names_with_a_style() {
        let style = |kind: &str| {
            let runs =
                runs(json!({ "text": "ab", "ranges": [{ "type": kind, "indices": [0, 2] }] }));
            runs[0].styles.clone()
        };
        assert_eq!(style("b"), [TextRunStyle::Bold]);
        assert_eq!(style("i"), [TextRunStyle::Italic]);
        assert_eq!(style("em"), [TextRunStyle::Italic]);
        assert_eq!(style("s"), [TextRunStyle::Strikethrough]);
        assert_eq!(style("del"), [TextRunStyle::Strikethrough]);
        assert_eq!(style("sup"), [TextRunStyle::Superscript]);
        assert_eq!(style("sub"), [TextRunStyle::Subscript]);
        assert_eq!(style("code"), [TextRunStyle::Code]);
        assert_eq!(style("pre"), [TextRunStyle::Preformatted]);
        assert_eq!(style("cite"), [TextRunStyle::Citation]);
        assert_eq!(style("h1"), [TextRunStyle::Heading { level: 1 }]);
        assert_eq!(style("h6"), [TextRunStyle::Heading { level: 6 }]);
        assert_eq!(style("li"), [TextRunStyle::ListItem { number: None }]);
        assert_eq!(style("match"), [TextRunStyle::Highlight]);
        for structure in ["p", "div", "span", "figure", "figcaption", "ul", "ol"] {
            assert_eq!(style(structure), [], "{structure} only gives structure");
        }
    }

    #[test]
    fn test_line_breaks_and_separators_cover_no_text() {
        let runs = runs(json!({
            "text": "onetwo",
            "ranges": [
                { "type": "br", "indices": [3, 3] },
                { "type": "hr", "indices": [6, 6] }
            ]
        }));
        let summary: Vec<_> = runs
            .iter()
            .map(|run| (run.text.as_str(), run.attachment.clone()))
            .collect();
        assert_eq!(
            summary,
            [
                ("one", None),
                ("", Some(TextRunAttachment::LineBreak)),
                ("two", None),
                ("", Some(TextRunAttachment::Separator)),
            ]
        );
    }

    #[test]
    fn test_media_placement() {
        let runs = runs(json!({
            "text": "Badge earned [photo]",
            "media": [
                {
                    "type": "badge", "indices": [0, 0], "url": "https://example.com/badge.png",
                    "width": "256", "height": "100%"
                },
                { "type": "image", "indices": [13, 20], "url": "https://example.com/photo.jpg" },
                { "type": "image", "indices": [5, 5], "url": "https://example.com/e.png", "emoji": true },
                { "type": "image", "indices": [1, 1] }
            ]
        }));
        let summary: Vec<_> = runs
            .iter()
            .map(|run| (run.text.as_str(), run.attachment.clone()))
            .collect();
        let image = |url: &str, width, height, is_emoji, is_badge| {
            Some(TextRunAttachment::Image {
                url: format!("https://example.com/{url}"),
                width,
                height,
                is_emoji,
                is_badge,
            })
        };
        assert_eq!(
            summary,
            [
                ("", image("badge.png", Some(256), None, false, true)),
                ("Badge", None),
                ("", image("e.png", None, None, true, false)),
                (" earned ", None),
                ("[photo]", image("photo.jpg", None, None, false, false)),
            ],
            "An image without an address is left out"
        );
    }

    #[test]
    fn test_out_of_range_positions_do_not_fail() {
        let block = block(json!({
            "text": "short",
            "ranges": [
                { "type": "b", "indices": [2, 40] },
                { "type": "a", "indices": [30, 40], "url": "https://example.com" },
                { "type": "i", "indices": [4, 1] }
            ],
            "media": [{ "type": "image", "indices": [9, 9], "url": "https://example.com/a.png" }]
        }));
        let text = notification_styled_text(&block.text, &block.ranges, &block.media);
        let summary: Vec<_> = text
            .runs
            .iter()
            .map(|run| {
                (
                    run.text.as_str(),
                    run.styles.clone(),
                    run.attachment.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("sh", vec![], false),
                ("ort", vec![TextRunStyle::Bold], false),
                ("", vec![], true),
            ]
        );
        assert_eq!(text.adjustments.end_past_text, 1);
        assert_eq!(text.adjustments.start_past_text, 2);
        assert_eq!(text.adjustments.reversed, 1);
    }

    #[test]
    fn test_person_links() {
        assert_eq!(
            link(json!({ "type": "user", "id": "701" })),
            Some(TextRunLink::User {
                user_id: Some(WpComUserId(701)),
                site_id: None,
                url: None
            })
        );
        assert_eq!(
            link(json!({ "type": "user", "url": "https://sam.example.org" })),
            Some(TextRunLink::User {
                user_id: None,
                site_id: None,
                url: Some("https://sam.example.org".to_string())
            })
        );
        assert_eq!(link(json!({ "type": "user" })), None);
    }

    #[test]
    fn test_site_links() {
        assert_eq!(
            link(json!({ "type": "site", "id": 801, "url": "https://example.com" })),
            Some(TextRunLink::Site {
                site_id: Some(WpComSiteId(801)),
                url: Some("https://example.com".to_string())
            })
        );
        assert_eq!(
            link(json!({ "type": "site", "site_id": 802 })),
            Some(TextRunLink::Site {
                site_id: Some(WpComSiteId(802)),
                url: None
            }),
            "Without an ID of its own, the site ID"
        );
    }

    #[test]
    fn test_post_and_page_links() {
        let post = TextRunLink::Post {
            site_id: Some(WpComSiteId(801)),
            post_id: Some(PostId(601)),
            url: Some("https://example.com/hello".to_string()),
        };
        for kind in ["post", "page"] {
            assert_eq!(
                link(json!({
                    "type": kind, "id": 601, "site_id": 801, "url": "https://example.com/hello"
                })),
                Some(post.clone())
            );
        }
    }

    #[test]
    fn test_comment_links() {
        assert_eq!(
            link(json!({
                "type": "comment", "id": 501, "post_id": 601, "site_id": 801,
                "url": "https://example.com/hello#comment-501"
            })),
            Some(TextRunLink::Comment {
                site_id: Some(WpComSiteId(801)),
                post_id: Some(PostId(601)),
                comment_id: Some(CommentId(501)),
                url: Some("https://example.com/hello#comment-501".to_string()),
            })
        );
    }

    #[test]
    fn test_web_links() {
        let web = Some(TextRunLink::Web {
            url: "https://example.com".to_string(),
        });
        for kind in [
            "a",
            "link",
            "stat",
            "scan",
            "follow",
            "like",
            "tracks",
            "rewind_download_ready",
            "example_link",
        ] {
            assert_eq!(
                link(json!({ "type": kind, "url": "https://example.com" })),
                web,
                "{kind}"
            );
        }
        assert_eq!(
            link(json!({ "url": "https://example.com" })),
            web,
            "No type"
        );
        assert_eq!(link(json!({ "type": "a" })), None, "No address");
        assert_eq!(
            link(json!({ "type": "b", "url": "https://example.com" })),
            None,
            "A style isn't a link"
        );
    }

    #[test]
    fn test_a_button_is_styled_and_linked() {
        let runs = runs(json!({
            "text": "Open",
            "ranges": [{ "type": "button", "indices": [0, 4], "url": "https://example.com" }]
        }));
        assert_eq!(runs[0].styles, [TextRunStyle::Button]);
        assert_eq!(
            runs[0].link,
            Some(TextRunLink::Web {
                url: "https://example.com".to_string()
            })
        );
    }
}
