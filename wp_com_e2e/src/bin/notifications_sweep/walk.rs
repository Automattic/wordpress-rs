//! How the sweep pages back through an account's notes, and how it knows it has reached the end.
//!
//! Each step asks for one page twice, with the same `before` and `number`: as whole notes, and as
//! IDs only. Both answers are the newest notes at or before `before`, but the whole notes leave out
//! the notes the API can't show, which the IDs still list. So:
//!
//! - The next page starts at the oldest note's second on this page. `before` includes its second,
//!   so the notes sharing it come back, and none is skipped.
//! - A page that brings no note older than `before` doesn't move the walk on. If its list of IDs is
//!   shorter than asked for, it listed every note left, so the walk has reached the end. If it's
//!   full, the page is asked for again at the most a page can hold. If that's still full, more
//!   notes share one second, or more notes in a row can't be shown, than one page holds, and the
//!   walk stops and says so.
//!
//! Each step either moves `before` back, makes the page bigger once, or ends, so the walk can't
//! loop.

use std::collections::HashSet;
use wp_api::wp_com::notifications::NotificationId;

/// The most notes a page can hold.
pub const MAX_PAGE_SIZE: u32 = 200;

/// The time a note with no date comes with: within the first day of 1970.
pub const UNDATED_BEFORE: i64 = 86_400;

/// Where the walk is, and every note it has met.
#[derive(Debug)]
pub struct Walk {
    /// The `before` of the next page, in seconds since 1970. `None` for the newest page.
    pub before: Option<i64>,
    /// The `number` of the next page.
    pub number: u32,
    page_size: u32,
    /// Notes that came whole, or were listed as notes the library couldn't read.
    seen: HashSet<NotificationId>,
    /// Notes the IDs listed that didn't come whole.
    listed_only: HashSet<NotificationId>,
}

/// What a step found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Ask for the next page.
    Continue,
    /// Every note has been met.
    End,
    /// The walk can't move on: a full list of IDs, at the most a page can hold, brought nothing
    /// older than `before`.
    Stuck,
}

/// One page, as the walk needs it.
pub struct Page<'a> {
    /// The whole notes' IDs, and their times in seconds since 1970.
    pub notes: &'a [(NotificationId, Option<i64>)],
    /// Notes on the whole page the library couldn't read, where their ID could be read.
    pub unreadable: &'a [NotificationId],
    /// The IDs-only answer to the same page.
    pub listed: &'a [NotificationId],
}

impl Walk {
    pub fn new(page_size: u32) -> Self {
        let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
        Self {
            before: None,
            number: page_size,
            page_size,
            seen: HashSet::new(),
            listed_only: HashSet::new(),
        }
    }

    /// Takes in a page, and says what to do next. Returns how many of its notes are new to the
    /// walk too.
    pub fn take(&mut self, page: Page) -> (Step, usize) {
        let mut new = 0;
        for id in page.notes.iter().map(|(id, _)| id).chain(page.unreadable) {
            if self.seen.insert(*id) {
                new += 1;
            }
            self.listed_only.remove(id);
        }
        for id in page.listed {
            if !self.seen.contains(id) {
                self.listed_only.insert(*id);
            }
        }

        if let Some(oldest) = oldest(page.notes)
            && self.before.is_none_or(|before| oldest < before)
        {
            self.before = Some(oldest);
            self.number = self.page_size;
            return (Step::Continue, new);
        }
        if page.listed.len() < self.number as usize {
            return (Step::End, new);
        }
        if self.number < MAX_PAGE_SIZE {
            self.number = MAX_PAGE_SIZE;
            return (Step::Continue, new);
        }
        (Step::Stuck, new)
    }

    /// How many notes came whole or were listed as unreadable.
    #[cfg(test)]
    pub fn seen(&self) -> usize {
        self.seen.len()
    }

    /// How many notes were listed by ID but never came whole.
    pub fn listed_only(&self) -> usize {
        self.listed_only.len()
    }
}

/// The second the next page starts at: the oldest dated note's, or, on a page of notes with no
/// date, theirs. Never below 1, since a `before` of 0 could read as no `before` at all.
fn oldest(notes: &[(NotificationId, Option<i64>)]) -> Option<i64> {
    let times = || notes.iter().filter_map(|(_, time)| *time);
    let oldest = times()
        .filter(|time| *time >= UNDATED_BEFORE)
        .min()
        .or_else(|| times().min())?;
    Some(oldest.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(id: u64) -> NotificationId {
        NotificationId(10_000_000_000 + id)
    }

    fn notes(list: &[(u64, i64)]) -> Vec<(NotificationId, Option<i64>)> {
        list.iter().map(|(n, t)| (id(*n), Some(*t))).collect()
    }

    fn ids(list: &[u64]) -> Vec<NotificationId> {
        list.iter().map(|n| id(*n)).collect()
    }

    fn take(walk: &mut Walk, whole: &[(u64, i64)], listed: &[u64]) -> Step {
        walk.take(Page {
            notes: &notes(whole),
            unreadable: &[],
            listed: &ids(listed),
        })
        .0
    }

    #[test]
    fn the_next_page_starts_at_the_oldest_second_and_ends_on_a_short_list() {
        let mut walk = Walk::new(3);
        assert_eq!(
            take(&mut walk, &[(5, 500), (4, 400), (3, 300)], &[5, 4, 3]),
            Step::Continue
        );
        assert_eq!(walk.before, Some(300));
        // The oldest note comes back, and two older ones with it.
        assert_eq!(
            take(&mut walk, &[(3, 300), (2, 200), (1, 100)], &[3, 2, 1]),
            Step::Continue
        );
        assert_eq!(walk.before, Some(100));
        // Only the note at that second is left: the list of IDs is short, so it's the end.
        assert_eq!(take(&mut walk, &[(1, 100)], &[1]), Step::End);
        assert_eq!(walk.seen(), 5);
        assert_eq!(walk.listed_only(), 0);
    }

    #[test]
    fn a_short_page_of_whole_notes_is_not_the_end() {
        let mut walk = Walk::new(3);
        // The API left out note 4, which the IDs still list.
        assert_eq!(
            take(&mut walk, &[(5, 500), (3, 300)], &[5, 4, 3]),
            Step::Continue
        );
        assert_eq!(walk.before, Some(300));
        assert_eq!(
            take(&mut walk, &[(3, 300), (1, 100)], &[3, 2, 1]),
            Step::Continue
        );
        assert_eq!(take(&mut walk, &[(1, 100)], &[1]), Step::End);
        assert_eq!(walk.listed_only(), 2);
    }

    #[test]
    fn an_empty_account_ends_at_once() {
        let mut walk = Walk::new(3);
        assert_eq!(take(&mut walk, &[], &[]), Step::End);
    }

    #[test]
    fn notes_sharing_a_second_past_a_page_get_a_bigger_page_then_stop() {
        let mut walk = Walk::new(2);
        assert_eq!(
            take(&mut walk, &[(9, 900), (8, 800)], &[9, 8]),
            Step::Continue
        );
        // Every note on the page shares the second it started at, and the list is full.
        assert_eq!(
            take(&mut walk, &[(8, 800), (7, 800)], &[8, 7]),
            Step::Continue
        );
        assert_eq!((walk.before, walk.number), (Some(800), MAX_PAGE_SIZE));
        // The bigger page moves on, and the page size goes back.
        assert_eq!(
            take(&mut walk, &[(8, 800), (7, 800), (6, 700)], &[8, 7, 6]),
            Step::Continue
        );
        assert_eq!((walk.before, walk.number), (Some(700), 2));

        let mut walk = Walk::new(MAX_PAGE_SIZE);
        let full: Vec<u64> = (0..u64::from(MAX_PAGE_SIZE)).collect();
        let whole: Vec<(u64, i64)> = full.iter().map(|n| (*n, 800)).collect();
        assert_eq!(take(&mut walk, &whole, &full), Step::Continue);
        assert_eq!(take(&mut walk, &whole, &full), Step::Stuck);
    }

    #[test]
    fn a_page_of_notes_that_cannot_be_shown_gets_a_bigger_page() {
        let mut walk = Walk::new(2);
        assert_eq!(
            take(&mut walk, &[(9, 900), (8, 800)], &[9, 8]),
            Step::Continue
        );
        // Nothing older came whole, though the IDs list two more.
        assert_eq!(take(&mut walk, &[(8, 800)], &[8, 7]), Step::Continue);
        assert_eq!(walk.number, MAX_PAGE_SIZE);
        assert_eq!(
            take(&mut walk, &[(8, 800), (6, 600)], &[8, 7, 6]),
            Step::Continue
        );
        assert_eq!(walk.before, Some(600));
        assert_eq!(take(&mut walk, &[(6, 600)], &[6]), Step::End);
        assert_eq!(walk.listed_only(), 1);
    }

    #[test]
    fn notes_with_no_date_page_last_and_never_from_zero() {
        let mut walk = Walk::new(3);
        // An undated note on a page of dated ones doesn't move the walk to 1970.
        assert_eq!(
            take(&mut walk, &[(5, 500_000), (2, 0), (4, 400_000)], &[5, 2, 4]),
            Step::Continue
        );
        assert_eq!(walk.before, Some(400_000));
        // Then the dated notes have run out, so the page is asked for again, bigger, and its
        // short list of IDs ends the walk.
        assert_eq!(
            take(&mut walk, &[(4, 400_000), (2, 0), (1, 1)], &[4, 2, 1]),
            Step::Continue
        );
        assert_eq!((walk.before, walk.number), (Some(400_000), MAX_PAGE_SIZE));
        assert_eq!(
            take(&mut walk, &[(4, 400_000), (2, 0), (1, 1)], &[4, 2, 1]),
            Step::End
        );
        // A page of undated notes alone moves the walk to their second, at least 1.
        let mut walk = Walk::new(3);
        assert_eq!(take(&mut walk, &[(2, 0), (1, 0)], &[2, 1]), Step::Continue);
        assert_eq!(walk.before, Some(1));
        assert_eq!(take(&mut walk, &[(2, 0), (1, 0)], &[2, 1]), Step::End);
        let mut walk = Walk::new(2);
        assert_eq!(
            take(&mut walk, &[(3, 200_000), (2, 0)], &[3, 2]),
            Step::Continue
        );
        assert_eq!(take(&mut walk, &[(2, 0), (1, 0)], &[2, 1]), Step::Continue);
        assert_eq!(walk.before, Some(1));
        assert_eq!(take(&mut walk, &[(2, 0), (1, 0)], &[2, 1]), Step::Continue);
        assert_eq!(walk.number, MAX_PAGE_SIZE);
        assert_eq!(take(&mut walk, &[(2, 0), (1, 0)], &[2, 1]), Step::End);
    }

    #[test]
    fn a_note_listed_first_and_sent_whole_later_is_not_listed_only() {
        let mut walk = Walk::new(2);
        assert_eq!(take(&mut walk, &[(9, 900)], &[9, 8]), Step::Continue);
        assert_eq!(walk.listed_only(), 1);
        assert_eq!(
            take(&mut walk, &[(9, 900), (8, 900)], &[9, 8]),
            Step::Continue
        );
        assert_eq!(walk.listed_only(), 0);
    }
}
