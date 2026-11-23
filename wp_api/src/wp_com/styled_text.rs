//! Plain text with ranges that style it, link it or put something in it, turned into flat runs.
//!
//! WordPress.com sends some text, such as a notification's, as plain text plus ranges over it.
//! Ranges can nest, overlap and cover no text, and they count positions in UTF-16 units. This
//! module resolves them into [`StyledTextRun`]s that follow one another with no gaps or overlap,
//! so an app builds its attributed text by appending each run's text with the run's styles.

use crate::{
    comments::CommentId,
    posts::PostId,
    wp_com::{WpComSiteId, me::WpComUserId},
};
use std::collections::HashMap;

/// A piece of text with one set of styles, at most one link, and maybe something to show in its
/// place.
///
/// Runs come in the order of the text, and their `text` put together is the whole text. To build
/// attributed text, append each run's `text` with its styles and link. A run with an
/// `attachment` shows the attachment instead of its `text`, which is then a description of it,
/// such as an image's alternative text, and is often empty.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct StyledTextRun {
    /// The run's part of the text.
    pub text: String,
    /// Where the run starts in the whole text, in UTF-16 units. That's how a Kotlin `String` and
    /// an `NSString` count, so it can be used as an index in either.
    pub utf16_offset: u32,
    /// How long the run's text is, in UTF-16 units.
    pub utf16_length: u32,
    /// The run's styles, outermost first, each at most once.
    pub styles: Vec<TextRunStyle>,
    /// Where the run leads, when it's a link. A run inside two links leads where the inner one
    /// does.
    pub link: Option<TextRunLink>,
    /// Something to show instead of the run's text.
    pub attachment: Option<TextRunAttachment>,
}

/// How a run looks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum TextRunStyle {
    Bold,
    Italic,
    Strikethrough,
    Superscript,
    Subscript,
    /// Code, in a fixed-width font.
    Code,
    /// A block of text whose spaces and line breaks are kept, usually in a fixed-width font.
    Preformatted,
    /// Quoted text, such as the comment a reply is to.
    Quote,
    /// The title of a work, usually in italics.
    Citation,
    /// A heading, from 1 (the largest) to 6.
    Heading {
        level: u8,
    },
    /// An item in a list. `number` is its place in a numbered list, and `None` in a bulleted
    /// one. The runs of one item share the style: show the number or bullet before the first.
    /// A run in a list inside a list has one of these for each list, outermost first.
    ListItem {
        number: Option<u32>,
    },
    /// Text to highlight, such as the words that matched a search.
    Highlight,
    /// Text to show as a button. Its run usually has a link.
    Button,
}

/// Where a run leads.
///
/// The person, site, post and comment links have the thing's IDs, for an app to open it itself,
/// and its web address when the API sent one.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TextRunLink {
    /// A web address.
    Web { url: String },
    /// A person.
    User {
        user_id: Option<WpComUserId>,
        /// The person's site.
        site_id: Option<WpComSiteId>,
        url: Option<String>,
    },
    /// A site.
    Site {
        site_id: Option<WpComSiteId>,
        url: Option<String>,
    },
    /// A post or a page.
    Post {
        site_id: Option<WpComSiteId>,
        post_id: Option<PostId>,
        url: Option<String>,
    },
    /// A comment.
    Comment {
        site_id: Option<WpComSiteId>,
        post_id: Option<PostId>,
        comment_id: Option<CommentId>,
        url: Option<String>,
    },
}

/// Something a run shows instead of its text.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TextRunAttachment {
    /// An image. The run's text, when it has any, is the image's alternative text.
    Image {
        url: String,
        /// In pixels, when the API sent a number of pixels.
        width: Option<u32>,
        /// In pixels, when the API sent a number of pixels.
        height: Option<u32>,
        /// An emoji drawn as an image: show it at the size of the text around it.
        is_emoji: bool,
        /// An achievement's badge.
        is_badge: bool,
    },
    /// An icon: a character in the Noticons font.
    Icon { character: String },
    /// A line break.
    LineBreak,
    /// A line across the text, between two parts of it.
    Separator,
}

/// What a range does to the text it covers.
#[derive(Debug, Clone, Default)]
pub(crate) struct TextMark {
    /// From the start up to, but not including, the end, in UTF-16 units, as sent.
    pub indices: Option<(u32, u32)>,
    pub style: Option<TextRunStyle>,
    pub link: Option<TextRunLink>,
    pub attachment: Option<TextRunAttachment>,
    /// A list whose items are the [`Self::is_list_item`] marks inside it.
    pub list: Option<ListKind>,
    /// An item of the smallest list around it. Its style is [`TextRunStyle::ListItem`].
    pub is_list_item: bool,
    /// The range's ID, which only means something inside the same text.
    pub id: Option<String>,
    /// The [`Self::id`] of the range this one is inside.
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
    Numbered,
    Bulleted,
}

impl TextMark {
    fn styles_or_links(&self) -> bool {
        self.style.is_some() || self.link.is_some() || self.is_list_item
    }
}

/// How often each rule for a position that doesn't fit the text was used.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StyledTextAdjustments {
    /// Ranges with no position, which were left out.
    pub no_position: u64,
    /// Ranges that end before they start, which were made to cover no text at their start.
    pub reversed: u64,
    /// Ranges that start after the end of the text, which were moved to the end of the text.
    pub start_past_text: u64,
    /// Ranges that end after the end of the text, which were cut at the end of the text.
    pub end_past_text: u64,
    /// Ranges with a position between the two halves of a character that takes two UTF-16
    /// units, which were widened to the whole character.
    pub inside_character: u64,
    /// Attachments covering text that another attachment covers, which were left out.
    pub overlapping_attachment: u64,
    /// Ranges of a kind that shows something, but without what to show, such as an image with
    /// no address. Those were left out.
    pub nothing_to_show: u64,
}

#[cfg(test)]
impl StyledTextAdjustments {
    pub(crate) fn add(&mut self, other: &Self) {
        self.no_position += other.no_position;
        self.reversed += other.reversed;
        self.start_past_text += other.start_past_text;
        self.end_past_text += other.end_past_text;
        self.inside_character += other.inside_character;
        self.overlapping_attachment += other.overlapping_attachment;
        self.nothing_to_show += other.nothing_to_show;
    }
}

pub(crate) struct StyledText {
    pub runs: Vec<StyledTextRun>,
    pub adjustments: StyledTextAdjustments,
}

/// Where each character of a text starts, in UTF-16 units and in bytes.
struct Utf16Index {
    /// One entry per character, then one for the end of the text.
    starts: Vec<(u32, usize)>,
}

impl Utf16Index {
    fn new(text: &str) -> Self {
        let mut starts = Vec::with_capacity(text.len() + 1);
        let mut utf16 = 0u32;
        for (byte, character) in text.char_indices() {
            starts.push((utf16, byte));
            utf16 += character.len_utf16() as u32;
        }
        starts.push((utf16, text.len()));
        Self { starts }
    }

    fn len(&self) -> u32 {
        self.starts.last().map_or(0, |(utf16, _)| *utf16)
    }

    /// The start of the character that `position` is in.
    fn floor(&self, position: u32) -> u32 {
        match self
            .starts
            .binary_search_by_key(&position, |(utf16, _)| *utf16)
        {
            Ok(_) => position,
            Err(next) => self.starts[next - 1].0,
        }
    }

    /// The end of the character that `position` is in.
    fn ceil(&self, position: u32) -> u32 {
        match self
            .starts
            .binary_search_by_key(&position, |(utf16, _)| *utf16)
        {
            Ok(_) => position,
            Err(next) => self.starts[next].0,
        }
    }

    /// The byte offset of `position`, which is at the start of a character or the end of the
    /// text.
    fn byte(&self, position: u32) -> usize {
        let found = self
            .starts
            .binary_search_by_key(&position, |(utf16, _)| *utf16)
            .expect("positions are at the start of a character");
        self.starts[found].1
    }
}

/// A mark with a position that fits the text.
struct Placed {
    start: u32,
    end: u32,
    /// Where it was in the list of marks, which breaks ties.
    order: usize,
}

impl Placed {
    fn len(&self) -> u32 {
        self.end - self.start
    }
}

/// Resolves `marks` over `text` into runs.
///
/// Positions that don't fit the text never fail: a missing one leaves the mark out, a reversed
/// one covers no text at its start, one past the end of the text is moved to the end, and one
/// between the two halves of a character is widened to the whole character.
pub(crate) fn styled_runs(text: &str, marks: &[TextMark]) -> StyledText {
    let index = Utf16Index::new(text);
    let length = index.len();
    let mut adjustments = StyledTextAdjustments::default();

    let mut placed = Vec::with_capacity(marks.len());
    for (order, mark) in marks.iter().enumerate() {
        let Some((mut start, mut end)) = mark.indices else {
            adjustments.no_position += 1;
            continue;
        };
        if start > end {
            adjustments.reversed += 1;
            end = start;
        }
        if start > length {
            adjustments.start_past_text += 1;
            start = length;
            end = length;
        } else if end > length {
            adjustments.end_past_text += 1;
            end = length;
        }
        let snapped_start = index.floor(start);
        let snapped_end = if end == start {
            snapped_start
        } else {
            index.ceil(end)
        };
        if (snapped_start, snapped_end) != (start, end) {
            adjustments.inside_character += 1;
        }
        placed.push(Placed {
            start: snapped_start,
            end: snapped_end,
            order,
        });
    }

    let list_styles = list_item_styles(marks, &placed);
    let style_of = |placed: &Placed| -> Option<TextRunStyle> {
        let mark = &marks[placed.order];
        if mark.is_list_item {
            list_styles.get(&placed.order).cloned()
        } else {
            mark.style.clone()
        }
    };

    // Attachments over text can't be split, so the first of any that overlap wins.
    let mut covering_attachments: Vec<&Placed> = placed
        .iter()
        .filter(|placed| marks[placed.order].attachment.is_some() && placed.len() > 0)
        .collect();
    covering_attachments
        .sort_by_key(|placed| (placed.start, u32::MAX - placed.len(), placed.order));
    let mut attachment_spans: Vec<&Placed> = Vec::new();
    for attachment in covering_attachments {
        match attachment_spans.last() {
            Some(last) if attachment.start < last.end => adjustments.overlapping_attachment += 1,
            _ => attachment_spans.push(attachment),
        }
    }
    let inside_attachment = |position: u32| {
        attachment_spans
            .iter()
            .find(|span| span.start < position && position < span.end)
    };

    let styling: Vec<&Placed> = placed
        .iter()
        .filter(|placed| placed.len() > 0 && marks[placed.order].styles_or_links())
        .collect();

    // Attachments that cover no text go before the text at their position.
    let mut points: Vec<(u32, &Placed)> = placed
        .iter()
        .filter(|placed| marks[placed.order].attachment.is_some() && placed.len() == 0)
        .map(|placed| {
            let position = inside_attachment(placed.start).map_or(placed.start, |span| span.start);
            (position, placed)
        })
        .collect();
    points.sort_by_key(|(position, placed)| (*position, placed.order));

    let mut boundaries = vec![0, length];
    for placed in styling
        .iter()
        .copied()
        .chain(attachment_spans.iter().copied())
    {
        boundaries.push(placed.start);
        boundaries.push(placed.end);
    }
    boundaries.retain(|position| inside_attachment(*position).is_none());
    boundaries.extend(points.iter().map(|(position, _)| *position));
    boundaries.sort_unstable();
    boundaries.dedup();

    let ids: HashMap<&str, usize> = marks
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(order, mark)| mark.id.as_deref().map(|id| (id, order)))
        .collect();

    let mut runs: Vec<StyledTextRun> = Vec::new();
    let mut points = points.into_iter().peekable();
    let mut push_points_at = |position: u32, runs: &mut Vec<StyledTextRun>| {
        while let Some((_, point)) = points.next_if(|(at, _)| *at <= position) {
            let mark = &marks[point.order];
            let covering: Vec<&Placed> = styling
                .iter()
                .copied()
                .filter(|placed| {
                    (placed.start < position && position < placed.end)
                        || ((placed.start == position || placed.end == position)
                            && is_inside(marks, &ids, point.order, placed.order))
                })
                .collect();
            let (styles, link) = styles_and_link(&covering, marks, &style_of);
            runs.push(StyledTextRun {
                text: String::new(),
                utf16_offset: position,
                utf16_length: 0,
                styles,
                link,
                attachment: mark.attachment.clone(),
            });
        }
    };

    for window in boundaries.windows(2) {
        let (start, end) = (window[0], window[1]);
        push_points_at(start, &mut runs);
        let attachment = attachment_spans
            .iter()
            .find(|span| span.start == start && span.end == end)
            .and_then(|span| marks[span.order].attachment.clone());
        let covering: Vec<&Placed> = styling
            .iter()
            .copied()
            .filter(|placed| placed.start <= start && end <= placed.end)
            .collect();
        let (styles, link) = styles_and_link(&covering, marks, &style_of);
        let text = &text[index.byte(start)..index.byte(end)];
        match runs.last_mut() {
            Some(last)
                if attachment.is_none()
                    && last.attachment.is_none()
                    && last.utf16_offset + last.utf16_length == start
                    && last.utf16_length > 0
                    && last.styles == styles
                    && last.link == link =>
            {
                last.text.push_str(text);
                last.utf16_length += end - start;
            }
            _ => runs.push(StyledTextRun {
                text: text.to_string(),
                utf16_offset: start,
                utf16_length: end - start,
                styles,
                link,
                attachment,
            }),
        }
    }
    push_points_at(length, &mut runs);

    StyledText { runs, adjustments }
}

/// The styles of the marks over a run, outermost first, and the link of the innermost one with
/// a link.
fn styles_and_link(
    covering: &[&Placed],
    marks: &[TextMark],
    style_of: &impl Fn(&Placed) -> Option<TextRunStyle>,
) -> (Vec<TextRunStyle>, Option<TextRunLink>) {
    let mut outermost_first = covering.to_vec();
    outermost_first.sort_by_key(|placed| (u32::MAX - placed.len(), placed.order));
    let mut styles: Vec<TextRunStyle> = Vec::new();
    let mut link = None;
    for placed in outermost_first {
        if let Some(style) = style_of(placed)
            && !styles.contains(&style)
        {
            styles.push(style);
        }
        if let Some(inner) = &marks[placed.order].link {
            link = Some(inner.clone());
        }
    }
    (styles, link)
}

/// Whether the mark at `inner` names the one at `outer` as its parent, or a parent of its
/// parent, through their IDs.
fn is_inside(marks: &[TextMark], ids: &HashMap<&str, usize>, inner: usize, outer: usize) -> bool {
    let mut current = inner;
    // A chain of parents longer than the list of marks has gone round in a circle.
    for _ in 0..marks.len() {
        let Some(parent) = marks[current].parent.as_deref().and_then(|id| ids.get(id)) else {
            return false;
        };
        if *parent == outer {
            return true;
        }
        current = *parent;
    }
    false
}

/// The [`TextRunStyle::ListItem`] of each list item, by its place in the list of marks. An item
/// belongs to the smallest list around it, and is numbered among that list's items when the list
/// is numbered.
fn list_item_styles(marks: &[TextMark], placed: &[Placed]) -> HashMap<usize, TextRunStyle> {
    let lists: Vec<&Placed> = placed
        .iter()
        .filter(|placed| marks[placed.order].list.is_some())
        .collect();
    let mut items: Vec<&Placed> = placed
        .iter()
        .filter(|placed| marks[placed.order].is_list_item)
        .collect();
    items.sort_by_key(|placed| (placed.start, placed.order));

    let mut counts: HashMap<usize, u32> = HashMap::new();
    let mut styles = HashMap::new();
    for item in items {
        let list = lists
            .iter()
            .filter(|list| {
                list.start <= item.start && item.end <= list.end && list.order != item.order
            })
            .min_by_key(|list| (list.len(), usize::MAX - list.order));
        let number = match list.and_then(|list| marks[list.order].list.map(|kind| (list, kind))) {
            Some((list, ListKind::Numbered)) => {
                let count = counts.entry(list.order).or_default();
                *count += 1;
                Some(*count)
            }
            _ => None,
        };
        styles.insert(item.order, TextRunStyle::ListItem { number });
    }
    styles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style(start: u32, end: u32, style: TextRunStyle) -> TextMark {
        TextMark {
            indices: Some((start, end)),
            style: Some(style),
            ..Default::default()
        }
    }

    fn web(start: u32, end: u32, url: &str) -> TextMark {
        TextMark {
            indices: Some((start, end)),
            link: Some(TextRunLink::Web {
                url: url.to_string(),
            }),
            ..Default::default()
        }
    }

    fn attachment(start: u32, end: u32, attachment: TextRunAttachment) -> TextMark {
        TextMark {
            indices: Some((start, end)),
            attachment: Some(attachment),
            ..Default::default()
        }
    }

    fn icon(character: &str) -> TextRunAttachment {
        TextRunAttachment::Icon {
            character: character.to_string(),
        }
    }

    /// Each run as its text and styles.
    fn texts_and_styles(text: &str, marks: &[TextMark]) -> Vec<(String, Vec<TextRunStyle>)> {
        let styled = styled_runs(text, marks);
        check_runs(text, &styled.runs);
        styled
            .runs
            .into_iter()
            .map(|run| (run.text, run.styles))
            .collect()
    }

    /// The runs follow one another, and their text is the whole text.
    fn check_runs(text: &str, runs: &[StyledTextRun]) {
        let mut offset = 0;
        for run in runs {
            assert_eq!(run.utf16_offset, offset, "Runs leave no gap: {runs:?}");
            assert_eq!(run.utf16_length as usize, run.text.encode_utf16().count());
            offset += run.utf16_length;
        }
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            text
        );
    }

    fn owned(runs: &[(&str, &[TextRunStyle])]) -> Vec<(String, Vec<TextRunStyle>)> {
        runs.iter()
            .map(|(text, styles)| (text.to_string(), styles.to_vec()))
            .collect()
    }

    use TextRunStyle::{Bold, Code, Italic};

    #[test]
    fn test_text_without_ranges_is_one_run() {
        assert_eq!(
            texts_and_styles("Hello there", &[]),
            owned(&[("Hello there", &[])])
        );
    }

    #[test]
    fn test_empty_text_has_no_runs() {
        assert!(styled_runs("", &[]).runs.is_empty());
    }

    #[test]
    fn test_nested_ranges_are_flattened_outermost_first() {
        // "a bold and italic end": bold over "bold and italic", italic over "italic".
        let text = "a bold and italic end";
        let marks = [style(2, 17, Bold), style(11, 17, Italic)];
        assert_eq!(
            texts_and_styles(text, &marks),
            owned(&[
                ("a ", &[]),
                ("bold and ", &[Bold]),
                ("italic", &[Bold, Italic]),
                (" end", &[]),
            ])
        );
    }

    #[test]
    fn test_overlap_that_is_not_nesting_styles_both_parts() {
        let marks = [style(0, 6, Bold), style(3, 9, Italic)];
        assert_eq!(
            texts_and_styles("abcdefghij", &marks),
            owned(&[
                ("abc", &[Bold]),
                ("def", &[Bold, Italic]),
                ("ghi", &[Italic]),
                ("j", &[]),
            ])
        );
    }

    #[test]
    fn test_the_same_style_twice_shows_once() {
        let marks = [style(0, 4, Bold), style(1, 3, Bold)];
        assert_eq!(
            texts_and_styles("abcd", &marks),
            owned(&[("abcd", &[Bold])])
        );
    }

    #[test]
    fn test_empty_ranges_without_attachments_change_nothing() {
        let marks = [style(2, 2, Bold), web(0, 0, "https://example.com")];
        assert_eq!(texts_and_styles("abcd", &marks), owned(&[("abcd", &[])]));
    }

    #[test]
    fn test_a_range_with_no_position_is_left_out() {
        let marks = [TextMark {
            style: Some(Bold),
            ..Default::default()
        }];
        let styled = styled_runs("abcd", &marks);
        assert_eq!(styled.runs.len(), 1);
        assert!(styled.runs[0].styles.is_empty());
        assert_eq!(styled.adjustments.no_position, 1);
    }

    #[test]
    fn test_a_range_past_the_end_is_cut_at_the_end() {
        let styled = styled_runs("abcd", &[style(2, 9, Bold)]);
        check_runs("abcd", &styled.runs);
        assert_eq!(styled.runs[1].text, "cd");
        assert_eq!(styled.runs[1].styles, [Bold]);
        assert_eq!(styled.adjustments.end_past_text, 1);
    }

    #[test]
    fn test_a_range_that_starts_past_the_end_covers_nothing() {
        let marks = [style(7, 9, Bold), attachment(6, 6, icon("\u{f300}"))];
        let styled = styled_runs("abcd", &marks);
        check_runs("abcd", &styled.runs);
        assert_eq!(styled.adjustments.start_past_text, 2);
        let [text, icon_run] = &styled.runs[..] else {
            panic!(
                "Expected the text, then the icon at its end: {:?}",
                styled.runs
            );
        };
        assert!(text.styles.is_empty());
        assert_eq!(icon_run.utf16_offset, 4);
        assert_eq!(icon_run.attachment, Some(icon("\u{f300}")));
    }

    #[test]
    fn test_a_reversed_range_covers_nothing_at_its_start() {
        let marks = [
            style(3, 1, Bold),
            attachment(3, 1, TextRunAttachment::LineBreak),
        ];
        let styled = styled_runs("abcd", &marks);
        check_runs("abcd", &styled.runs);
        assert_eq!(styled.adjustments.reversed, 2);
        let texts: Vec<_> = styled.runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(texts, ["abc", "", "d"]);
        assert_eq!(
            styled.runs[1].attachment,
            Some(TextRunAttachment::LineBreak)
        );
        assert!(styled.runs.iter().all(|run| run.styles.is_empty()));
    }

    #[test]
    fn test_positions_count_utf16_units() {
        // "😀" and "𝄞" each take two UTF-16 units, "é" one.
        let text = "😀 café 𝄞 end";
        let marks = [style(3, 7, Bold), style(8, 10, Italic)];
        let styled = styled_runs(text, &marks);
        check_runs(text, &styled.runs);
        let runs: Vec<_> = styled
            .runs
            .iter()
            .map(|run| (run.text.as_str(), run.utf16_offset, run.utf16_length))
            .collect();
        assert_eq!(
            runs,
            [
                ("😀 ", 0, 3),
                ("café", 3, 4),
                (" ", 7, 1),
                ("𝄞", 8, 2),
                (" end", 10, 4)
            ]
        );
        assert_eq!(styled.adjustments, StyledTextAdjustments::default());
    }

    #[test]
    fn test_a_position_inside_a_character_widens_to_the_whole_character() {
        // Both ends of the range fall between the halves of an emoji.
        let text = "a😀b😀c";
        let styled = styled_runs(text, &[style(2, 5, Bold)]);
        check_runs(text, &styled.runs);
        let runs: Vec<_> = styled
            .runs
            .iter()
            .map(|run| (run.text.as_str(), run.styles.clone()))
            .collect();
        assert_eq!(runs, [("a", vec![]), ("😀b😀", vec![Bold]), ("c", vec![])]);
        assert_eq!(styled.adjustments.inside_character, 1);
    }

    #[test]
    fn test_an_empty_range_inside_a_character_moves_to_its_start() {
        let text = "a😀b";
        let styled = styled_runs(text, &[attachment(2, 2, TextRunAttachment::LineBreak)]);
        check_runs(text, &styled.runs);
        let texts: Vec<_> = styled.runs.iter().map(|run| run.text.as_str()).collect();
        assert_eq!(texts, ["a", "", "😀b"]);
        assert_eq!(styled.adjustments.inside_character, 1);
    }

    #[test]
    fn test_an_attachment_covering_text_is_one_run() {
        // An image replaces its alternative text, and a style inside it doesn't split it.
        let image = TextRunAttachment::Image {
            url: "https://example.com/a.png".to_string(),
            width: Some(10),
            height: None,
            is_emoji: false,
            is_badge: false,
        };
        let marks = [
            style(0, 9, Code),
            attachment(2, 6, image.clone()),
            style(3, 4, Bold),
        ];
        let styled = styled_runs("a [alt] b", &marks);
        check_runs("a [alt] b", &styled.runs);
        let runs: Vec<_> = styled
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
            runs,
            [
                ("a ", vec![Code], false),
                ("[alt", vec![Code], true),
                ("] b", vec![Code], false),
            ]
        );
        assert_eq!(styled.runs[1].attachment, Some(image));
    }

    #[test]
    fn test_overlapping_attachments_keep_the_first() {
        let marks = [
            attachment(0, 3, icon("1")),
            attachment(2, 5, icon("2")),
            attachment(3, 4, icon("3")),
        ];
        let styled = styled_runs("abcdef", &marks);
        check_runs("abcdef", &styled.runs);
        let attachments: Vec<_> = styled
            .runs
            .iter()
            .map(|run| (run.text.as_str(), run.attachment.clone()))
            .collect();
        assert_eq!(
            attachments,
            [
                ("abc", Some(icon("1"))),
                ("d", Some(icon("3"))),
                ("ef", None)
            ]
        );
        assert_eq!(styled.adjustments.overlapping_attachment, 1);
    }

    #[test]
    fn test_empty_attachments_go_before_the_text_at_their_position() {
        let marks = [
            style(0, 4, Bold),
            attachment(0, 0, icon("1")),
            attachment(2, 2, icon("2")),
            attachment(4, 4, icon("3")),
            attachment(2, 2, TextRunAttachment::Separator),
        ];
        let styled = styled_runs("abcd", &marks);
        check_runs("abcd", &styled.runs);
        let runs: Vec<_> = styled
            .runs
            .iter()
            .map(|run| {
                (
                    run.text.as_str(),
                    run.styles.clone(),
                    run.attachment.clone(),
                )
            })
            .collect();
        assert_eq!(
            runs,
            [
                ("", vec![], Some(icon("1"))),
                ("ab", vec![Bold], None),
                ("", vec![Bold], Some(icon("2"))),
                ("", vec![Bold], Some(TextRunAttachment::Separator)),
                ("cd", vec![Bold], None),
                ("", vec![], Some(icon("3"))),
            ],
            "At the edge of a range, an empty attachment is outside it"
        );
    }

    #[test]
    fn test_an_empty_attachment_whose_parent_is_a_range_is_inside_it() {
        let link = TextMark {
            id: Some("1".to_string()),
            ..web(0, 4, "https://example.com")
        };
        let image = TextMark {
            parent: Some("1".to_string()),
            ..attachment(0, 0, icon("x"))
        };
        let styled = styled_runs("abcd", &[link, image]);
        check_runs("abcd", &styled.runs);
        assert_eq!(styled.runs[0].attachment, Some(icon("x")));
        assert_eq!(
            styled.runs[0].link,
            Some(TextRunLink::Web {
                url: "https://example.com".to_string()
            })
        );
    }

    #[test]
    fn test_parents_that_go_round_in_a_circle_end() {
        let a = TextMark {
            id: Some("1".to_string()),
            parent: Some("2".to_string()),
            ..attachment(0, 0, icon("a"))
        };
        let b = TextMark {
            id: Some("2".to_string()),
            parent: Some("1".to_string()),
            ..attachment(0, 0, icon("b"))
        };
        let styled = styled_runs("x", &[style(0, 1, Bold), a, b]);
        check_runs("x", &styled.runs);
        assert!(styled.runs[0].styles.is_empty());
    }

    #[test]
    fn test_attachments_in_empty_text() {
        let styled = styled_runs(
            "",
            &[attachment(0, 0, icon("x")), attachment(0, 3, icon("y"))],
        );
        let attachments: Vec<_> = styled
            .runs
            .iter()
            .map(|run| run.attachment.clone())
            .collect();
        assert_eq!(attachments, [Some(icon("x")), Some(icon("y"))]);
        assert_eq!(styled.adjustments.end_past_text, 1);
    }

    #[test]
    fn test_the_inner_link_wins() {
        let marks = [
            web(0, 6, "https://example.com/outer"),
            web(2, 4, "https://example.com/inner"),
        ];
        let styled = styled_runs("abcdef", &marks);
        check_runs("abcdef", &styled.runs);
        let links: Vec<_> = styled
            .runs
            .iter()
            .map(|run| match &run.link {
                Some(TextRunLink::Web { url }) => url.trim_start_matches("https://example.com/"),
                _ => "none",
            })
            .collect();
        assert_eq!(links, ["outer", "inner", "outer"]);
    }

    #[test]
    fn test_list_items_are_numbered_in_numbered_lists() {
        // A numbered list of two items, the second holding a bulleted list of one.
        let text = "one\ntwo\nsub\n";
        let item = |start, end| TextMark {
            indices: Some((start, end)),
            is_list_item: true,
            ..Default::default()
        };
        let list = |start, end, kind| TextMark {
            indices: Some((start, end)),
            list: Some(kind),
            ..Default::default()
        };
        let marks = [
            list(0, 12, ListKind::Numbered),
            item(0, 4),
            item(4, 12),
            list(8, 12, ListKind::Bulleted),
            item(8, 12),
        ];
        assert_eq!(
            texts_and_styles(text, &marks),
            owned(&[
                ("one\n", &[TextRunStyle::ListItem { number: Some(1) }]),
                ("two\n", &[TextRunStyle::ListItem { number: Some(2) }]),
                (
                    "sub\n",
                    &[
                        TextRunStyle::ListItem { number: Some(2) },
                        TextRunStyle::ListItem { number: None }
                    ]
                ),
            ])
        );
    }

    #[test]
    fn test_a_list_item_outside_a_list_has_a_bullet() {
        let marks = [TextMark {
            indices: Some((0, 3)),
            is_list_item: true,
            ..Default::default()
        }];
        assert_eq!(
            texts_and_styles("one", &marks),
            owned(&[("one", &[TextRunStyle::ListItem { number: None }])])
        );
    }
}
