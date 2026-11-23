//! What a sweep counts, and the report it writes.
//!
//! The report holds counts, note type names, the kind names the library knows, key paths and
//! kinds of error. It never holds a note's text, IDs, hashes, addresses or names, nor the name of
//! a block, range, medium or action the library doesn't know: those are counted, with how many
//! distinct names there were.

use crate::notification_checks::{NoteCheck, without_values};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use wp_api::wp_com::notifications::{
    NotificationActionKind, NotificationBlock, NotificationBlockKind, NotificationMedia,
    NotificationMediaKind, NotificationRange, NotificationRangeKind, NotificationType,
    UnreadableNotification, WpComNotification,
};

use crate::walk::UNDATED_BEFORE;

/// How often a kind of value arrived as `Other`, and how many distinct names it had.
#[derive(Debug, Default, Clone, Serialize)]
pub struct OtherCount {
    pub values: u64,
    pub distinct: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    /// How the walk ended: it reached the end, or why it stopped before.
    pub end: String,
    pub requests: u64,
    /// Steps of the walk: each asks for a page as whole notes and as IDs.
    pub pages: u64,
    /// Notes that came whole, each counted once.
    pub notes: u64,
    /// Notes the IDs listed that never came whole: the API left them out. There can be more:
    /// a page of whole notes reaches further back than its list of IDs, and the walk moves on
    /// from the whole notes.
    pub notes_listed_only: u64,
    /// Notes the library couldn't read, by note type and the kind of error.
    pub unreadable_notes: BTreeMap<String, u64>,
    /// Pages that didn't parse at all, by the kind of error.
    pub pages_that_did_not_parse: BTreeMap<String, u64>,
    /// Requests that failed, by HTTP status or the kind of failure.
    pub failed_requests: BTreeMap<String, u64>,
    pub note_types: BTreeMap<String, u64>,
    /// Notes whose type has no case of its own, which is expected for most types.
    pub note_types_without_a_case: OtherCount,
    pub read: u64,
    pub unread: u64,
    /// Notes with someone they're from in their note context.
    pub with_an_actor: u64,
    /// Notes at the very start of 1970, which is how the API sends a note with no date.
    pub undated: u64,
    /// Pages where a note with no date came before a dated note.
    pub pages_with_an_undated_note_before_a_dated_one: u64,
    /// Body blocks by kind. `(none)` is a block with no kind: text.
    pub block_kinds: BTreeMap<String, u64>,
    pub range_kinds: BTreeMap<String, u64>,
    pub media_kinds: BTreeMap<String, u64>,
    pub action_kinds: BTreeMap<String, u64>,
    /// Values that arrived as `Other`, by what they are.
    pub other_values: BTreeMap<&'static str, OtherCount>,
    /// Keys in a note's `meta.ids` and `meta.links` that the model has no field for.
    pub meta_keys_without_a_field: BTreeMap<String, u64>,
    pub texts: u64,
    pub runs: u64,
    /// Texts whose runs aren't the text, or whose runs panicked, by key path and what's wrong.
    pub text_problems: BTreeMap<String, u64>,
    pub layouts: BTreeMap<&'static str, u64>,
    pub layouts_by_note_type: BTreeMap<String, u64>,
    /// Notes whose layout panicked, by note type.
    pub layout_panics: BTreeMap<String, u64>,
    #[serde(skip)]
    distinct: BTreeMap<&'static str, HashSet<String>>,
}

impl Report {
    /// Whether everything that came could be read.
    pub fn all_read(&self) -> bool {
        self.unreadable_notes.is_empty()
            && self.pages_that_did_not_parse.is_empty()
            && self.failed_requests.is_empty()
            && self.text_problems.is_empty()
            && self.layout_panics.is_empty()
            && self.end == END_REACHED
    }

    pub fn unreadable(&mut self, notes: &[UnreadableNotification]) {
        for note in notes {
            let key = format!(
                "{}: {}",
                note.note_type.as_deref().unwrap_or("(no type)"),
                without_values(&note.error)
            );
            *self.unreadable_notes.entry(key).or_default() += 1;
        }
    }

    /// Counts a note the walk hasn't counted before.
    pub fn note(&mut self, note: &WpComNotification, check: &NoteCheck) {
        self.notes += 1;
        let note_type = note
            .note_type
            .as_ref()
            .map_or("(no type)", NotificationType::as_str)
            .to_string();
        *self.note_types.entry(note_type.clone()).or_default() += 1;
        if let Some(NotificationType::Other(name)) = &note.note_type {
            self.other("note type", name);
        }
        match note.read {
            Some(true) => self.read += 1,
            Some(false) => self.unread += 1,
            None => {}
        }
        if note
            .timestamp
            .is_some_and(|time| time.0.timestamp() < UNDATED_BEFORE)
        {
            self.undated += 1;
        }
        if let Some(meta) = &note.meta {
            self.with_an_actor += u64::from(meta.actor.is_some());
            for key in meta.ids.iter().flat_map(|ids| ids.other.keys()) {
                *self
                    .meta_keys_without_a_field
                    .entry(format!("meta.ids.{key}"))
                    .or_default() += 1;
            }
            for key in meta.links.iter().flat_map(|links| links.other.keys()) {
                *self
                    .meta_keys_without_a_field
                    .entry(format!("meta.links.{key}"))
                    .or_default() += 1;
            }
        }

        for text in note.subject.iter().chain(&note.header) {
            self.parts(&text.ranges, &text.media);
        }
        for block in &note.body {
            self.block(block);
        }
        if let Some(meta) = &note.meta {
            if let Some(summary) = &meta.summary {
                self.parts(&summary.ranges, &summary.media);
            }
            if let Some(content) = meta
                .parent_comment
                .as_ref()
                .and_then(|p| p.content.as_ref())
            {
                self.block(content);
            }
        }

        self.texts += check.texts;
        self.runs += check.runs;
        for (path, problem) in &check.text_problems {
            *self
                .text_problems
                .entry(format!("{path}: {problem}"))
                .or_default() += 1;
        }
        match check.layout {
            Some(layout) => {
                *self.layouts.entry(layout).or_default() += 1;
                *self
                    .layouts_by_note_type
                    .entry(format!("{note_type}: {layout}"))
                    .or_default() += 1;
            }
            None => *self.layout_panics.entry(note_type).or_default() += 1,
        }
    }

    fn block(&mut self, block: &NotificationBlock) {
        let kind = match &block.kind {
            None => "(none)",
            Some(NotificationBlockKind::Other(name)) => {
                self.other("block kind", name);
                "other"
            }
            Some(kind) => kind.as_str(),
        };
        *self.block_kinds.entry(kind.to_string()).or_default() += 1;
        for action in &block.actions {
            let kind = match &action.kind {
                NotificationActionKind::Other(name) => {
                    self.other("action kind", name);
                    "other"
                }
                kind => kind.as_str(),
            };
            *self.action_kinds.entry(kind.to_string()).or_default() += 1;
        }
        self.parts(&block.ranges, &block.media);
    }

    fn parts(&mut self, ranges: &[NotificationRange], media: &[NotificationMedia]) {
        for range in ranges {
            let kind = match &range.kind {
                None => "(none)",
                Some(NotificationRangeKind::Other(name)) => {
                    self.other("range kind", name);
                    "other"
                }
                Some(kind) => kind.as_str(),
            };
            *self.range_kinds.entry(kind.to_string()).or_default() += 1;
        }
        for medium in media {
            let kind = match &medium.kind {
                None => "(none)",
                Some(NotificationMediaKind::Other(name)) => {
                    self.other("media kind", name);
                    "other"
                }
                Some(kind) => kind.as_str(),
            };
            *self.media_kinds.entry(kind.to_string()).or_default() += 1;
        }
    }

    fn other(&mut self, what: &'static str, name: &str) {
        self.other_values.entry(what).or_default().values += 1;
        let distinct = self.distinct.entry(what).or_default();
        distinct.insert(name.to_string());
        let count = distinct.len();
        self.other_values.entry(what).or_default().distinct = count;
        if what == "note type" {
            self.note_types_without_a_case = self.other_values[what].clone();
        }
    }

    /// The report's lines for the terminal: counts only.
    pub fn summary(&self) -> String {
        format!(
            "{}: {} requests, {} pages, {} notes (at least {} more listed by ID only), {} unreadable, {} pages \
             that didn't parse, {} failed requests, {} text problems, {} layout panics",
            self.end,
            self.requests,
            self.pages,
            self.notes,
            self.notes_listed_only,
            self.unreadable_notes.values().sum::<u64>(),
            self.pages_that_did_not_parse.values().sum::<u64>(),
            self.failed_requests.values().sum::<u64>(),
            self.text_problems.values().sum::<u64>(),
            self.layout_panics.values().sum::<u64>(),
        )
    }
}

pub const END_REACHED: &str = "reached the end";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification_checks::check_note;

    #[test]
    fn a_name_the_library_does_not_know_is_counted_not_kept() {
        let note: WpComNotification = serde_json::from_str(
            r#"{
                "id": 10000000001,
                "type": "example_note",
                "read": 0,
                "subject": [{"text": "Hello", "ranges": [{"type": "example_range", "indices": [0, 5]}, {"type": "b", "indices": [0, 1]}]}],
                "body": [{"type": "example_block", "text": "Hi", "actions": {"example-action": true, "follow": false}}],
                "meta": {"ids": {"site": 1, "example_id": 2}}
            }"#,
        )
        .unwrap();
        let mut report = Report::default();
        report.note(&note, &check_note(&note));
        let json = serde_json::to_string(&report).unwrap();
        for unknown in ["example_range", "example_block", "example-action"] {
            assert!(!json.contains(unknown), "{unknown} is in {json}");
        }
        // Note type names and key paths may be in it.
        assert!(json.contains("example_note"));
        assert!(json.contains("meta.ids.example_id"));
        assert_eq!(report.other_values["range kind"].values, 1);
        assert_eq!(report.other_values["block kind"].distinct, 1);
        assert_eq!(report.range_kinds["b"], 1);
        assert_eq!(report.action_kinds["other"], 1);
        assert_eq!(report.note_types_without_a_case.values, 1);
        assert_eq!(report.unread, 1);
    }

    #[test]
    fn an_unreadable_note_is_kept_as_its_type_and_error_kind() {
        let mut report = Report::default();
        report.unreadable(&[UnreadableNotification {
            id: None,
            note_type: Some("like".to_string()),
            error: "invalid type: string \"Ada\", expected a sequence at line 1 column 40"
                .to_string(),
        }]);
        assert_eq!(
            report.unreadable_notes.keys().collect::<Vec<_>>(),
            ["like: invalid type: string \"…\", expected a sequence at line 9 column 9"]
        );
    }
}
