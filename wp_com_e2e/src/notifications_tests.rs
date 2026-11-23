//! The user's notifications, read only: nothing here marks a note read or seen.
//!
//! CI runs these with an account that may have few notes or none, while new ones can arrive
//! between two requests. So each trial checks only what the API promises of one answer, and a
//! trial that needs a note and finds none passes and says so. No trial prints anything from a
//! note but its type. No request sends a `locale`, which would change every note's hash.
//!
//! A request for whole notes gives them new hashes, so each trial holds
//! `AccountLocks::notification_hashes` for reading, and runs while the sync trials, which compare
//! hashes, don't.

use libtest_mimic::{Failed, Trial};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use wp_api::wp_com::notifications::{
    NotificationId, NotificationParams, NotificationType, NotificationsParams, NotificationsQuery,
    NotificationsResponse, UnreadableNotification, WpComNotification,
};

use crate::context::{TestContext, reading};
use crate::notification_checks::{check_note, without_values};

/// Notes per page: enough to see several types, few enough to stay quick.
const PAGE_SIZE: u32 = 20;

/// Fails when the page lists a note the library couldn't read, naming only its type and why.
fn no_unreadable(unreadable: &[UnreadableNotification]) -> Result<(), Failed> {
    match unreadable.first() {
        None => Ok(()),
        Some(note) => Err(format!(
            "{} note(s) couldn't be read. The first is of type `{}`: {}",
            unreadable.len(),
            note.note_type.as_deref().unwrap_or("(no type)"),
            without_values(&note.error)
        )
        .into()),
    }
}

fn note_type(note: &WpComNotification) -> &str {
    note.note_type
        .as_ref()
        .map_or("(no type)", NotificationType::as_str)
}

/// A note at the very start of 1970, which is how the API sends a note with no date.
fn undated(note: &WpComNotification) -> bool {
    note.timestamp
        .is_none_or(|timestamp| timestamp.0.timestamp() < 86_400)
}

async fn list(
    ctx: &TestContext,
    query: NotificationsQuery,
    include_body: bool,
    include_context: bool,
) -> Result<NotificationsResponse, Failed> {
    let page = ctx
        .client
        .notifications()
        .list(&NotificationsParams {
            query,
            include_body,
            include_context,
        })
        .await
        .map_err(|e| e.to_string())?
        .data;
    no_unreadable(&page.unreadable_notes)?;
    Ok(page)
}

fn first_page() -> NotificationsQuery {
    NotificationsQuery {
        number: Some(PAGE_SIZE),
        ..Default::default()
    }
}

/// Counts of note types, for a trial's output.
fn types(notes: &[WpComNotification]) -> BTreeMap<&str, usize> {
    let mut types = BTreeMap::new();
    for note in notes {
        *types.entry(note_type(note)).or_default() += 1;
    }
    types
}

pub fn tests(ctx: Arc<TestContext>) -> Vec<Trial> {
    vec![
        Trial::test("notifications::list_whole_notes", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let page = list(&ctx, first_page(), true, true).await?;
                    if page.last_seen_time.is_none() {
                        return Err("The page has no last-seen time".into());
                    }
                    if page.notes.len() > PAGE_SIZE as usize {
                        return Err(
                            format!("{} notes on a page of {PAGE_SIZE}", page.notes.len()).into(),
                        );
                    }
                    let ids: HashSet<_> = page.notes.iter().map(|note| note.id).collect();
                    if ids.len() != page.notes.len() {
                        return Err("A note is on the page twice".into());
                    }
                    for note in &page.notes {
                        let missing = [
                            ("type", note.note_type.is_none()),
                            ("read", note.read.is_none()),
                            ("timestamp", note.timestamp.is_none()),
                            ("note_hash", note.note_hash.is_none()),
                        ];
                        if let Some((field, _)) = missing.iter().find(|(_, missing)| *missing) {
                            return Err(
                                format!("A `{}` note has no `{field}`", note_type(note)).into()
                            );
                        }
                    }
                    // Newest first. A note without a date can be anywhere.
                    let dated: Vec<_> = page
                        .notes
                        .iter()
                        .filter(|note| !undated(note))
                        .filter_map(|note| note.timestamp)
                        .collect();
                    if dated.windows(2).any(|pair| pair[0].0 < pair[1].0) {
                        return Err("The notes aren't newest first".into());
                    }
                    if !page.notes.is_empty() && page.notes.iter().all(|note| note.body.is_empty())
                    {
                        return Err("No note on the page has a body".into());
                    }
                    let with_context = page
                        .notes
                        .iter()
                        .filter(|note| note.meta.as_ref().is_some_and(|meta| meta.actor.is_some()))
                        .count();
                    println!(
                        "{} notes, {with_context} with someone they're from: {:?}",
                        page.notes.len(),
                        types(&page.notes)
                    );
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::list_without_body_or_context", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let page = list(&ctx, first_page(), false, false).await?;
                    for note in &page.notes {
                        if !note.header.is_empty() || !note.body.is_empty() {
                            return Err(format!(
                                "A `{}` note has a header or body that wasn't asked for",
                                note_type(note)
                            )
                            .into());
                        }
                        let meta = note.meta.as_ref();
                        let context = meta.is_some_and(|meta| {
                            meta.actor.is_some()
                                || meta.summary.is_some()
                                || meta.post.is_some()
                                || meta.site.is_some()
                                || meta.parent_comment.is_some()
                        });
                        if context {
                            return Err(format!(
                                "A `{}` note has note context that wasn't asked for",
                                note_type(note)
                            )
                            .into());
                        }
                    }
                    println!("{} notes", page.notes.len());
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::list_hashes", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let page = ctx
                        .client
                        .notifications()
                        .list_hashes(&first_page())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    no_unreadable(&page.unreadable_notes)?;
                    if page.notes.iter().any(|note| note.note_hash.is_none()) {
                        return Err("A note has no hash".into());
                    }
                    let ids: HashSet<_> = page.notes.iter().map(|note| note.id).collect();
                    if ids.len() != page.notes.len() {
                        return Err("A note is on the page twice".into());
                    }
                    println!("{} notes", page.notes.len());
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::list_ids", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let page = ctx
                        .client
                        .notifications()
                        .list_ids(&first_page())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    no_unreadable(&page.unreadable_notes)?;
                    if page.ids.len() > PAGE_SIZE as usize {
                        return Err(
                            format!("{} IDs on a page of {PAGE_SIZE}", page.ids.len()).into()
                        );
                    }
                    let ids: HashSet<_> = page.ids.iter().collect();
                    if ids.len() != page.ids.len() {
                        return Err("An ID is on the page twice".into());
                    }
                    println!("{} IDs", page.ids.len());
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::get_one", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    // Taken from whole notes: a list of IDs also has the notes the API leaves
                    // out of whole notes because it can't show them.
                    let newest = list(&ctx, first_page(), false, false).await?;
                    let Some(id) = newest.notes.first().map(|note| &note.id) else {
                        println!("The account has no notes, so there's none to fetch");
                        return Ok(());
                    };
                    let params = NotificationParams {
                        include_context: true,
                        ..Default::default()
                    };
                    let page = ctx
                        .client
                        .notifications()
                        .get(id, &params)
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    no_unreadable(&page.unreadable_notes)?;
                    match page.notes.as_slice() {
                        [note] if note.id == *id => {
                            println!("Fetched a `{}` note", note_type(note));
                            Ok(())
                        }
                        notes => Err(format!(
                            "Expected the note asked for alone, got {} notes",
                            notes.len()
                        )
                        .into()),
                    }
                })
            }
        }),
        Trial::test("notifications::get_one_that_does_not_exist", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    // Far beyond the IDs the API hands out.
                    let id = NotificationId(999_999_999_999_999);
                    let page = ctx
                        .client
                        .notifications()
                        .get(&id, &NotificationParams::default())
                        .await
                        .map_err(|e| e.to_string())?
                        .data;
                    if !page.notes.is_empty() || !page.unreadable_notes.is_empty() {
                        return Err("A note that doesn't exist came back".into());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::paging_with_before", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    const SMALL_PAGE: u32 = 5;
                    let query = NotificationsQuery {
                        number: Some(SMALL_PAGE),
                        ..Default::default()
                    };
                    let first = list(&ctx, query.clone(), false, false).await?;
                    let oldest = first
                        .notes
                        .iter()
                        .filter(|note| !undated(note))
                        .min_by_key(|note| note.timestamp.map(|time| time.0));
                    let Some(oldest) = oldest else {
                        println!("The account has no dated notes, so there's nothing to page");
                        return Ok(());
                    };
                    let before = oldest.timestamp.expect("a dated note has a time");
                    let next = list(
                        &ctx,
                        NotificationsQuery {
                            before: Some(before),
                            ..query
                        },
                        false,
                        false,
                    )
                    .await?;
                    if let Some(newer) = next.notes.iter().find(|note| {
                        !undated(note) && note.timestamp.is_some_and(|t| t.0 > before.0)
                    }) {
                        return Err(format!(
                            "A `{}` note newer than `before` came back",
                            note_type(newer)
                        )
                        .into());
                    }
                    // `before` includes its second, so the oldest note comes back, unless more
                    // notes share that second than fit on the page.
                    let all_in_that_second = next.notes.len() == SMALL_PAGE as usize
                        && next.notes.iter().all(|note| note.timestamp == Some(before));
                    if !all_in_that_second && !next.notes.iter().any(|note| note.id == oldest.id) {
                        return Err(
                            "The oldest note of the first page isn't on the next page".into()
                        );
                    }
                    let older = next
                        .notes
                        .iter()
                        .filter(|note| note.timestamp.is_some_and(|t| t.0 < before.0))
                        .count();
                    println!(
                        "The next page has {} notes, {older} of them older than the first page",
                        next.notes.len()
                    );
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::unread_and_read_filters", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    for unread in [true, false] {
                        let query = NotificationsQuery {
                            unread,
                            read: !unread,
                            ..first_page()
                        };
                        let page = list(&ctx, query, false, false).await?;
                        if let Some(note) =
                            page.notes.iter().find(|note| note.read != Some(!unread))
                        {
                            let filter = if unread { "unread" } else { "read" };
                            return Err(format!(
                                "The `{filter}` filter returned a `{}` note whose read state is \
                                 {:?}",
                                note_type(note),
                                note.read
                            )
                            .into());
                        }
                        let filter = if unread { "unread" } else { "read" };
                        println!("{} {filter} notes", page.notes.len());
                    }
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::type_filter", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    // A type the account has, or a common one when it has none.
                    let first = list(&ctx, first_page(), false, false).await?;
                    let wanted = first
                        .notes
                        .iter()
                        .find_map(|note| note.note_type.clone())
                        .unwrap_or(NotificationType::Like);
                    let query = NotificationsQuery {
                        types: vec![wanted.clone()],
                        ..first_page()
                    };
                    let page = list(&ctx, query, false, false).await?;
                    if let Some(note) = page
                        .notes
                        .iter()
                        .find(|note| note.note_type.as_ref() != Some(&wanted))
                    {
                        return Err(format!(
                            "Asked for `{}` notes, got a `{}` note",
                            wanted.as_str(),
                            note_type(note)
                        )
                        .into());
                    }
                    if first
                        .notes
                        .iter()
                        .any(|note| note.note_type.as_ref() == Some(&wanted))
                        && page.notes.is_empty()
                    {
                        return Err(format!("No `{}` notes came back", wanted.as_str()).into());
                    }
                    println!("{} `{}` notes", page.notes.len(), wanted.as_str());
                    Ok(())
                })
            }
        }),
        Trial::test("notifications::styled_runs_and_layouts", {
            let ctx = Arc::clone(&ctx);
            move || {
                let _hashes = reading(&ctx.locks.notification_hashes);
                ctx.runtime.block_on(async {
                    let query = NotificationsQuery {
                        number: Some(50),
                        ..Default::default()
                    };
                    let page = list(&ctx, query, true, true).await?;
                    if page.notes.is_empty() {
                        println!("The account has no notes, so there's nothing to show");
                        return Ok(());
                    }
                    let mut layouts: BTreeMap<&str, usize> = BTreeMap::new();
                    let (mut texts, mut runs) = (0, 0);
                    for note in &page.notes {
                        let check = check_note(note);
                        if let Some(problem) = check.problem() {
                            return Err(format!("A `{}` note: {problem}", note_type(note)).into());
                        }
                        texts += check.texts;
                        runs += check.runs;
                        if let Some(layout) = check.layout {
                            *layouts.entry(layout).or_default() += 1;
                        }
                    }
                    println!(
                        "{} notes, {texts} texts as {runs} runs, layouts {layouts:?}",
                        page.notes.len()
                    );
                    Ok(())
                })
            }
        }),
    ]
}
