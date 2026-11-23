//! Checks of a fetched note, shared by the notification trials and `notifications_sweep`: every
//! text in the note becomes styled runs, and the note gets a layout. What they return never holds
//! any of the note's text, so it can be printed.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Once;
use wp_api::wp_com::notifications::{
    NotificationLayoutContent, NotificationLayoutFallback, WpComNotification,
    notification_block_styled_runs, notification_layout, notification_text_styled_runs,
};
use wp_api::wp_com::styled_text::StyledTextRun;

/// What turning a note's texts into styled runs, and building its layout, gave.
#[derive(Debug, Default)]
pub struct NoteCheck {
    /// Texts turned into styled runs.
    pub texts: u64,
    pub runs: u64,
    /// Texts whose runs aren't the text, or whose runs panicked, by key path and what's wrong.
    pub text_problems: Vec<(&'static str, &'static str)>,
    /// The layout's name. `None` when building it panicked.
    pub layout: Option<&'static str>,
}

impl NoteCheck {
    pub fn is_ok(&self) -> bool {
        self.text_problems.is_empty() && self.layout.is_some()
    }

    /// What's wrong, without anything from the note, or `None` when nothing is.
    pub fn problem(&self) -> Option<String> {
        if self.is_ok() {
            return None;
        }
        let mut problems: Vec<String> = self
            .text_problems
            .iter()
            .map(|(path, problem)| format!("{path}: {problem}"))
            .collect();
        if self.layout.is_none() {
            problems.push("building the layout panicked".to_string());
        }
        Some(problems.join("; "))
    }
}

/// Turns every text of a note into styled runs, and builds its layout, as an app showing it
/// would. A panic in either is caught and reported.
pub fn check_note(note: &WpComNotification) -> NoteCheck {
    let mut check = NoteCheck::default();
    for text in &note.subject {
        check_runs(&mut check, "subject[]", &text.text, || {
            notification_text_styled_runs(text)
        });
    }
    for text in &note.header {
        check_runs(&mut check, "header[]", &text.text, || {
            notification_text_styled_runs(text)
        });
    }
    for block in &note.body {
        check_runs(&mut check, "body[]", &block.text, || {
            notification_block_styled_runs(block)
        });
    }
    if let Some(meta) = &note.meta {
        if let Some(summary) = &meta.summary {
            check_runs(&mut check, "meta.summary", &summary.text, || {
                notification_text_styled_runs(summary)
            });
        }
        let content = meta
            .parent_comment
            .as_ref()
            .and_then(|parent| parent.content.as_ref());
        if let Some(content) = content {
            check_runs(
                &mut check,
                "meta.parent_comment.content",
                &content.text,
                || notification_block_styled_runs(content),
            );
        }
    }
    check.layout = catching_quietly(|| notification_layout(note))
        .ok()
        .map(|layout| layout_name(&layout.content, layout.fallback));
    check
}

fn check_runs(
    check: &mut NoteCheck,
    path: &'static str,
    text: &str,
    runs: impl FnOnce() -> Vec<StyledTextRun>,
) {
    check.texts += 1;
    match catching_quietly(runs) {
        Ok(runs) => {
            check.runs += runs.len() as u64;
            if let Some(problem) = runs_problem(text, &runs) {
                check.text_problems.push((path, problem));
            }
        }
        Err(_) => check
            .text_problems
            .push((path, "turning it into runs panicked")),
    }
}

thread_local! {
    /// Set while this thread runs the library on a note.
    static QUIET_PANICS: Cell<bool> = const { Cell::new(false) };
}

/// Runs `work` on a note and catches its panic. A panic's message can quote the note, so only
/// where it happened is printed. A panic anywhere else prints as it did before.
fn catching_quietly<T>(work: impl FnOnce() -> T) -> std::thread::Result<T> {
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let usual = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if QUIET_PANICS.get() {
                let place = info
                    .location()
                    .map_or("an unknown place".to_string(), |at| at.to_string());
                eprintln!("panicked at {place}");
            } else {
                usual(info);
            }
        }));
    });
    QUIET_PANICS.set(true);
    let result = catch_unwind(AssertUnwindSafe(work));
    QUIET_PANICS.set(false);
    result
}

/// What's wrong with a text's runs, if anything. The runs must cover the text in order, each as
/// long as its own text, and put together be the text.
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

/// The name of a layout, and for a notice, why the note has no layout of its own.
pub fn layout_name(
    content: &NotificationLayoutContent,
    fallback: Option<NotificationLayoutFallback>,
) -> &'static str {
    match (content, fallback) {
        (NotificationLayoutContent::Comment { .. }, _) => "comment",
        (NotificationLayoutContent::Reactions { .. }, _) => "reactions",
        (NotificationLayoutContent::Follow { .. }, _) => "follow",
        (NotificationLayoutContent::NewPost { .. }, _) => "new post",
        (NotificationLayoutContent::Notice { .. }, Some(NotificationLayoutFallback::NoContext)) => {
            "notice (no note context)"
        }
        (NotificationLayoutContent::Notice { .. }, _) => "notice (type has no layout of its own)",
    }
}

/// An error message with every quoted value and every number taken out, so it can be shown
/// without anything from the note: `invalid type: string "abc"` becomes
/// `invalid type: string "…"`, and each run of digits becomes `9`.
pub fn without_values(message: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_taken_out_of_an_error() {
        assert_eq!(
            without_values(
                "invalid type: string \"Ada wrote\", expected u64 at line 12 column 345"
            ),
            "invalid type: string \"…\", expected u9 at line 9 column 9"
        );
        assert_eq!(without_values("missing field `id`"), "missing field `…`");
    }

    #[test]
    fn a_panic_on_a_note_is_caught_and_later_panics_print_as_usual() {
        let caught = catching_quietly(|| -> u8 { panic!("a note's text") });
        assert!(caught.is_err());
        assert!(!QUIET_PANICS.get());
        assert_eq!(catching_quietly(|| 7).ok(), Some(7));
    }

    #[test]
    fn a_note_with_every_kind_of_text_is_checked() {
        let note: WpComNotification = serde_json::from_str(
            r#"{
                "id": 10000000001,
                "type": "comment",
                "subject": [{"text": "Ada replied", "ranges": [{"type": "user", "indices": [0, 3], "id": 5}]}],
                "header": [{"text": "A post"}],
                "body": [{"text": "Hello"}, {"text": ""}],
                "meta": {"summary": {"text": "Ada"}}
            }"#,
        )
        .unwrap();
        let check = check_note(&note);
        assert_eq!(check.texts, 5);
        assert!(check.is_ok(), "{:?}", check.problem());
        assert_eq!(check.layout, Some("notice (no note context)"));
    }
}
