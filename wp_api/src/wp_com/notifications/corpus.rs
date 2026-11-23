//! Reads a folder of recorded notes and reports every note the model can't read, and every
//! value that lands in an `Other` case. It also turns every text in every note into styled runs,
//! checks that the runs put back together are the text, and counts how often each rule for a
//! position that doesn't fit its text was used. Last, it builds every note's layout and counts
//! the layouts by note type and the notices by why the note has no layout of its own.
//!
//! The notes are kept outside the repository. Each `.jsonl` file holds one note per line: the
//! note as the API sends it, or an object with the note under `note`. Name the folders, separated
//! as in `PATH`, and run:
//!
//! ```bash
//! NOTIFICATIONS_CORPUS=/path/to/notes cargo test -p wp_api --lib \
//!     wp_com::notifications::corpus -- --ignored --nocapture
//! ```
//!
//! Set `NOTIFICATIONS_CORPUS_REPORT` to a file to write the report there as well. The report
//! holds only counts, type names and key paths, never a value from a note.

use super::{
    layout::{NotificationLayoutContent, NotificationLayoutFallback, notification_layout},
    styled_runs::notification_styled_text,
    *,
};
use crate::wp_com::styled_text::{StyledTextAdjustments, StyledTextRun, TextRunAttachment};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

/// How many notes go in each page that's read as a whole.
const PAGE_SIZE: usize = 200;

#[derive(Default)]
struct Tally {
    lines: u64,
    unreadable_lines: u64,
    notes: u64,
    parsed: u64,
    /// Notes that failed, by type and the reason with its values taken out.
    failed: BTreeMap<(String, String), u64>,
    by_type: BTreeMap<String, (u64, u64)>,
    /// Values in an `Other` case, by what they name.
    others: BTreeMap<(&'static str, String), u64>,
    /// Elements in the recorded note that the model left out, by key path.
    left_out: BTreeMap<String, u64>,
    /// Values the model read as absent while the note had a value, by key path.
    read_as_absent: BTreeMap<&'static str, u64>,
    /// Ranges and media whose positions don't fit their text, by key path.
    out_of_bounds: BTreeMap<String, u64>,
    undated: u64,
    /// Texts turned into styled runs.
    styled_texts: u64,
    styled_runs: u64,
    /// Texts whose runs aren't the text, by key path and what's wrong.
    styled_failures: BTreeMap<(String, &'static str), u64>,
    /// How often each rule for a position that doesn't fit its text was used.
    adjustments: StyledTextAdjustments,
    /// Texts that needed any of those rules, by key path.
    adjusted_texts: BTreeMap<String, u64>,
    /// Runs with an attachment, by its kind.
    attachments: BTreeMap<&'static str, u64>,
    pages: u64,
    pages_failed: u64,
    pages_miscounted: u64,
    /// Layouts built, by note type and layout.
    layouts: BTreeMap<(String, &'static str), u64>,
    /// Notices, by why the note has no layout of its own.
    fallbacks: BTreeMap<&'static str, u64>,
    /// Notes whose layout couldn't be built, by note type.
    layout_failures: BTreeMap<String, u64>,
}

#[test]
#[ignore = "needs a folder of recorded notes; see the module comment"]
fn corpus() {
    let Some(folders) = std::env::var_os("NOTIFICATIONS_CORPUS") else {
        println!("NOTIFICATIONS_CORPUS isn't set, so there's nothing to read.");
        return;
    };
    let mut tally = Tally::default();
    for folder in std::env::split_paths(&folders) {
        if !folder.is_dir() {
            println!("{} isn't a folder, so it was skipped.", folder.display());
            continue;
        }
        for file in note_files(&folder) {
            read_file(&file, &mut tally);
        }
    }
    let report = report(&tally);
    println!("{report}");
    if let Some(path) = std::env::var_os("NOTIFICATIONS_CORPUS_REPORT") {
        fs::write(&path, &report).expect("the report can be written");
    }
}

fn note_files(folder: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(folder)
        .expect("the folder can be read")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect();
    files.sort();
    files
}

fn read_file(file: &Path, tally: &mut Tally) {
    let reader = BufReader::new(File::open(file).expect("the file can be opened"));
    let mut page = Vec::with_capacity(PAGE_SIZE);
    for line in reader.lines() {
        let line = line.expect("the file can be read");
        if line.trim().is_empty() {
            continue;
        }
        tally.lines += 1;
        let Ok(mut value) = serde_json::from_str::<Value>(&line) else {
            tally.unreadable_lines += 1;
            continue;
        };
        let note = match value.get_mut("note") {
            Some(note) if note.is_object() => note.take(),
            _ => value,
        };
        let json = note.to_string();
        read_note(&note, &json, tally);
        page.push(json);
        if page.len() == PAGE_SIZE {
            read_page(&mut page, tally);
        }
    }
    read_page(&mut page, tally);
}

/// Reads the notes as one page, as the list endpoint would, and checks that each comes back as
/// a note or an unreadable note.
fn read_page(page: &mut Vec<String>, tally: &mut Tally) {
    if page.is_empty() {
        return;
    }
    tally.pages += 1;
    let json = format!(r#"{{"last_seen_time":"0","notes":[{}]}}"#, page.join(","));
    match serde_json::from_str::<NotificationsResponse>(&json) {
        Ok(response) => {
            if response.notes.len() + response.unreadable_notes.len() != page.len() {
                tally.pages_miscounted += 1;
            }
        }
        Err(_) => tally.pages_failed += 1,
    }
    page.clear();
}

fn read_note(raw: &Value, json: &str, tally: &mut Tally) {
    tally.notes += 1;
    let note_type = raw
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("(no type)")
        .to_string();
    let counts = tally.by_type.entry(note_type.clone()).or_default();
    counts.0 += 1;
    match serde_json::from_str::<WpComNotification>(json) {
        Ok(note) => {
            counts.1 += 1;
            tally.parsed += 1;
            check_note(raw, &note, tally);
            check_layout(&note_type, &note, tally);
        }
        Err(error) => {
            *tally
                .failed
                .entry((note_type, without_values(&error.to_string())))
                .or_default() += 1;
        }
    }
}

fn check_layout(note_type: &str, note: &WpComNotification, tally: &mut Tally) {
    let Ok(layout) = std::panic::catch_unwind(|| notification_layout(note)) else {
        *tally
            .layout_failures
            .entry(note_type.to_string())
            .or_default() += 1;
        return;
    };
    let name = match (&layout.content, layout.fallback) {
        (NotificationLayoutContent::Comment { .. }, _) => "comment",
        (NotificationLayoutContent::Reactions { .. }, _) => "reactions",
        (NotificationLayoutContent::Follow { .. }, _) => "follow",
        (NotificationLayoutContent::NewPost { .. }, _) => "new post",
        (NotificationLayoutContent::Notice { .. }, Some(NotificationLayoutFallback::NoContext)) => {
            "notice (no note context)"
        }
        (NotificationLayoutContent::Notice { .. }, _) => "notice (type has no layout of its own)",
    };
    *tally
        .layouts
        .entry((note_type.to_string(), name))
        .or_default() += 1;
    if let Some(fallback) = layout.fallback {
        let reason = match fallback {
            NotificationLayoutFallback::NoContext => "no note context",
            NotificationLayoutFallback::NoLayoutForType => "type has no layout of its own",
        };
        *tally.fallbacks.entry(reason).or_default() += 1;
    }
}

/// An error message with every quoted value and number taken out.
fn without_values(message: &str) -> String {
    let mut out = String::new();
    let mut quote = None;
    for character in message.chars() {
        match quote {
            Some(end) if character == end => {
                out.push('…');
                out.push(character);
                quote = None;
            }
            Some(_) => {}
            None if character == '"' || character == '`' => {
                out.push(character);
                quote = Some(character);
            }
            None if character.is_ascii_digit() => {
                if !out.ends_with('9') {
                    out.push('9');
                }
            }
            None => out.push(character),
        }
    }
    out
}

fn objects(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, |list| {
        list.iter().filter(|element| element.is_object()).count()
    })
}

fn has_text(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.is_empty())
}

fn check_note(raw: &Value, note: &WpComNotification, tally: &mut Tally) {
    if note.note_type.is_none() && raw.get("type").is_some() {
        *tally.read_as_absent.entry("type").or_default() += 1;
    }
    if note.timestamp.is_none() && raw.get("timestamp").is_some() {
        *tally.read_as_absent.entry("timestamp").or_default() += 1;
    }
    if note
        .timestamp
        .is_some_and(|timestamp| timestamp.0.timestamp() < 86_400)
    {
        tally.undated += 1;
    }
    if note.read.is_none() && raw.get("read").is_some() {
        *tally.read_as_absent.entry("read").or_default() += 1;
    }
    if note.note_hash.is_none() && raw.get("note_hash").is_some() {
        *tally.read_as_absent.entry("note_hash").or_default() += 1;
    }
    if note.title.is_none() && has_text(raw.get("title")) {
        *tally.read_as_absent.entry("title").or_default() += 1;
    }
    if let Some(NotificationType::Other(name)) = &note.note_type {
        *tally.others.entry(("note type", name.clone())).or_default() += 1;
    }

    check_texts("subject", raw.get("subject"), &note.subject, tally);
    check_texts("header", raw.get("header"), &note.header, tally);
    check_blocks("body", raw.get("body"), &note.body, tally);

    let raw_meta = raw.get("meta");
    if raw_meta.is_some_and(Value::is_object) && note.meta.is_none() {
        *tally.left_out.entry("meta".to_string()).or_default() += 1;
    }
    if let (Some(raw_meta), Some(meta)) = (raw_meta, &note.meta) {
        if let (Some(raw_summary), Some(summary)) = (raw_meta.get("summary"), &meta.summary) {
            check_text("meta.summary", raw_summary, summary, tally);
        }
        let raw_content = raw_meta
            .get("parent_comment")
            .and_then(|parent| parent.get("content"));
        let content = meta
            .parent_comment
            .as_ref()
            .and_then(|parent| parent.content.as_ref());
        if let (Some(raw_content), Some(content)) = (raw_content, content) {
            check_block("meta.parent_comment.content", raw_content, content, tally);
        }
        for key in [
            "actor",
            "summary",
            "post",
            "site",
            "parent_comment",
            "ids",
            "links",
        ] {
            let present = match key {
                "actor" => meta.actor.is_some(),
                "summary" => meta.summary.is_some(),
                "post" => meta.post.is_some(),
                "site" => meta.site.is_some(),
                "parent_comment" => meta.parent_comment.is_some(),
                "ids" => meta.ids.is_some(),
                _ => meta.links.is_some(),
            };
            if raw_meta.get(key).is_some_and(Value::is_object) && !present {
                *tally.left_out.entry(format!("meta.{key}")).or_default() += 1;
            }
        }
    }
}

fn check_texts(path: &str, raw: Option<&Value>, texts: &[NotificationText], tally: &mut Tally) {
    if objects(raw) != texts.len() {
        *tally.left_out.entry(format!("{path}[]")).or_default() += 1;
        return;
    }
    let raw_objects = raw
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|element| element.is_object());
    for (raw, text) in raw_objects.zip(texts) {
        check_text(&format!("{path}[]"), raw, text, tally);
    }
}

fn check_blocks(path: &str, raw: Option<&Value>, blocks: &[NotificationBlock], tally: &mut Tally) {
    if objects(raw) != blocks.len() {
        *tally.left_out.entry(format!("{path}[]")).or_default() += 1;
        return;
    }
    let raw_objects = raw
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|element| element.is_object());
    for (raw, block) in raw_objects.zip(blocks) {
        check_block(&format!("{path}[]"), raw, block, tally);
    }
}

fn check_text(path: &str, raw: &Value, text: &NotificationText, tally: &mut Tally) {
    check_parts(path, raw, &text.text, &text.ranges, &text.media, tally);
}

fn check_block(path: &str, raw: &Value, block: &NotificationBlock, tally: &mut Tally) {
    check_parts(path, raw, &block.text, &block.ranges, &block.media, tally);
    if let Some(NotificationBlockKind::Other(name)) = &block.kind {
        *tally
            .others
            .entry(("block type", name.clone()))
            .or_default() += 1;
    }
    let raw_actions = raw
        .get("actions")
        .and_then(Value::as_object)
        .map_or(0, |actions| actions.len());
    if raw_actions != block.actions.len() {
        *tally.left_out.entry(format!("{path}.actions")).or_default() += 1;
    }
    for action in &block.actions {
        if let NotificationActionKind::Other(name) = &action.kind {
            *tally.others.entry(("action", name.clone())).or_default() += 1;
        }
        if action.is_on.is_none() {
            *tally.read_as_absent.entry("actions value").or_default() += 1;
        }
    }
    if raw.get("meta").is_some_and(Value::is_object) && block.meta.is_none() {
        *tally.left_out.entry(format!("{path}.meta")).or_default() += 1;
    }
}

fn check_parts(
    path: &str,
    raw: &Value,
    text: &str,
    ranges: &[NotificationRange],
    media: &[NotificationMedia],
    tally: &mut Tally,
) {
    if has_text(raw.get("text")) && text.is_empty() {
        *tally.read_as_absent.entry("text").or_default() += 1;
    }
    let length = text.encode_utf16().count() as u32;
    if objects(raw.get("ranges")) != ranges.len() {
        *tally
            .left_out
            .entry(format!("{path}.ranges[]"))
            .or_default() += 1;
    }
    if objects(raw.get("media")) != media.len() {
        *tally.left_out.entry(format!("{path}.media[]")).or_default() += 1;
    }
    for range in ranges {
        match &range.kind {
            Some(NotificationRangeKind::Other(name)) => {
                *tally
                    .others
                    .entry(("range type", name.clone()))
                    .or_default() += 1;
            }
            None => *tally.read_as_absent.entry("range type").or_default() += 1,
            Some(_) => {}
        }
        check_indices(&format!("{path}.ranges[]"), range.indices, length, tally);
    }
    check_styled_runs(path, text, ranges, media, tally);
    for medium in media {
        match &medium.kind {
            Some(NotificationMediaKind::Other(name)) => {
                *tally
                    .others
                    .entry(("media type", name.clone()))
                    .or_default() += 1;
            }
            None => *tally.read_as_absent.entry("media type").or_default() += 1,
            Some(_) => {}
        }
        check_indices(&format!("{path}.media[]"), medium.indices, length, tally);
    }
}

fn check_styled_runs(
    path: &str,
    text: &str,
    ranges: &[NotificationRange],
    media: &[NotificationMedia],
    tally: &mut Tally,
) {
    tally.styled_texts += 1;
    let styled = std::panic::catch_unwind(|| notification_styled_text(text, ranges, media));
    let Ok(styled) = styled else {
        *tally
            .styled_failures
            .entry((path.to_string(), "panicked"))
            .or_default() += 1;
        return;
    };
    tally.styled_runs += styled.runs.len() as u64;
    if let Some(problem) = runs_problem(text, &styled.runs) {
        *tally
            .styled_failures
            .entry((path.to_string(), problem))
            .or_default() += 1;
    }
    if styled.adjustments != StyledTextAdjustments::default() {
        *tally.adjusted_texts.entry(path.to_string()).or_default() += 1;
    }
    tally.adjustments.add(&styled.adjustments);
    for run in &styled.runs {
        let kind = match &run.attachment {
            None => continue,
            Some(TextRunAttachment::Image { is_badge: true, .. }) => "badge",
            Some(TextRunAttachment::Image { is_emoji: true, .. }) => "emoji image",
            Some(TextRunAttachment::Image { .. }) => "image",
            Some(TextRunAttachment::Icon { .. }) => "icon",
            Some(TextRunAttachment::LineBreak) => "line break",
            Some(TextRunAttachment::Separator) => "separator",
        };
        *tally.attachments.entry(kind).or_default() += 1;
    }
}

/// What's wrong with a text's runs, if anything.
fn runs_problem(text: &str, runs: &[StyledTextRun]) -> Option<&'static str> {
    let mut offset = 0u32;
    for run in runs {
        if run.utf16_offset != offset {
            return Some("a run doesn't start where the one before it ends");
        }
        if run.utf16_length as usize != run.text.encode_utf16().count() {
            return Some("a run's length isn't the length of its text");
        }
        if run.utf16_length == 0 && run.attachment.is_none() {
            return Some("a run has no text and no attachment");
        }
        offset += run.utf16_length;
    }
    let joined: String = runs.iter().map(|run| run.text.as_str()).collect();
    (joined != text).then_some("the runs put together aren't the text")
}

fn check_indices(path: &str, indices: Option<NotificationIndices>, length: u32, tally: &mut Tally) {
    match indices {
        None => *tally.read_as_absent.entry("indices").or_default() += 1,
        Some(NotificationIndices { start, end }) if start > end || end > length => {
            *tally.out_of_bounds.entry(path.to_string()).or_default() += 1;
        }
        Some(_) => {}
    }
}

fn report(tally: &Tally) -> String {
    let mut out = String::new();
    let mut line = |text: String| {
        out.push_str(&text);
        out.push('\n');
    };
    line(format!("Lines read: {}", tally.lines));
    line(format!(
        "Lines that aren't JSON: {}",
        tally.unreadable_lines
    ));
    line(format!("Notes read: {}", tally.notes));
    line(format!("Notes parsed: {}", tally.parsed));
    line(format!("Notes that failed: {}", tally.notes - tally.parsed));
    line(format!(
        "Notes at the start of 1970 (no date): {}",
        tally.undated
    ));
    line(format!(
        "Pages of {PAGE_SIZE} read whole: {}, failed: {}, with a note missing from both lists: {}",
        tally.pages, tally.pages_failed, tally.pages_miscounted
    ));
    line(String::new());
    line("Failures by note type and reason:".to_string());
    if tally.failed.is_empty() {
        line("  none".to_string());
    }
    for ((note_type, reason), count) in &tally.failed {
        line(format!("  {count:>8}  {note_type}: {reason}"));
    }
    line(String::new());
    line("Values in an Other case:".to_string());
    let mut others: Vec<_> = tally.others.iter().collect();
    others.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for ((what, name), count) in others {
        line(format!("  {count:>8}  {what}: {name}"));
    }
    for (title, entries) in [
        ("Elements the model left out, by key path:", &tally.left_out),
        (
            "Ranges and media whose positions don't fit their text, by key path:",
            &tally.out_of_bounds,
        ),
    ] {
        line(String::new());
        line(title.to_string());
        if entries.is_empty() {
            line("  none".to_string());
        }
        for (path, count) in entries {
            line(format!("  {count:>8}  {path}"));
        }
    }
    line(String::new());
    line(format!(
        "Texts turned into styled runs: {}, runs: {}",
        tally.styled_texts, tally.styled_runs
    ));
    line("Texts whose runs aren't the text, by key path:".to_string());
    if tally.styled_failures.is_empty() {
        line("  none".to_string());
    }
    for ((path, problem), count) in &tally.styled_failures {
        line(format!("  {count:>8}  {path}: {problem}"));
    }
    let adjustments = &tally.adjustments;
    line("Positions that didn't fit their text, by the rule used:".to_string());
    for (rule, count) in [
        ("no position: left out", adjustments.no_position),
        (
            "ends before it starts: covers no text at its start",
            adjustments.reversed,
        ),
        (
            "starts past the end: moved to the end",
            adjustments.start_past_text,
        ),
        (
            "ends past the end: cut at the end",
            adjustments.end_past_text,
        ),
        (
            "inside a character: widened to the whole character",
            adjustments.inside_character,
        ),
        (
            "attachment over another attachment: left out",
            adjustments.overlapping_attachment,
        ),
        (
            "icon or image with nothing to show: left out",
            adjustments.nothing_to_show,
        ),
    ] {
        line(format!("  {count:>8}  {rule}"));
    }
    line("Texts that needed any of those rules, by key path:".to_string());
    for (path, count) in &tally.adjusted_texts {
        line(format!("  {count:>8}  {path}"));
    }
    line("Runs with an attachment, by kind:".to_string());
    for (kind, count) in &tally.attachments {
        line(format!("  {count:>8}  {kind}"));
    }
    line(String::new());
    line("Values read as absent although the note had one:".to_string());
    if tally.read_as_absent.is_empty() {
        line("  none".to_string());
    }
    for (path, count) in &tally.read_as_absent {
        line(format!("  {count:>8}  {path}"));
    }
    line(String::new());
    let built: u64 = tally.layouts.values().sum();
    let failed: u64 = tally.layout_failures.values().sum();
    line(format!("Layouts built: {built}, failed: {failed}"));
    line("Notes whose layout couldn't be built, by note type:".to_string());
    if tally.layout_failures.is_empty() {
        line("  none".to_string());
    }
    for (note_type, count) in &tally.layout_failures {
        line(format!("  {count:>8}  {note_type}"));
    }
    line("Layouts by kind:".to_string());
    let mut by_kind: BTreeMap<&str, u64> = BTreeMap::new();
    for ((_, kind), count) in &tally.layouts {
        *by_kind.entry(kind).or_default() += count;
    }
    for (kind, count) in by_kind {
        line(format!("  {count:>8}  {kind}"));
    }
    line("Notices by why the note has no layout of its own:".to_string());
    for (reason, count) in &tally.fallbacks {
        line(format!("  {count:>8}  {reason}"));
    }
    line("Layouts by note type:".to_string());
    for ((note_type, kind), count) in &tally.layouts {
        line(format!("  {count:>8}  {note_type}: {kind}"));
    }
    line(String::new());
    line("Notes read and parsed, by note type:".to_string());
    for (note_type, (read, parsed)) in &tally.by_type {
        line(format!("  {read:>8} {parsed:>8}  {note_type}"));
    }
    out
}
