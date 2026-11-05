//! The state's rules, one example each. `scenario_tests.rs` checks them through the service.

use super::*;
use ReaderSavedStatus::{NotSaved, Saved, Unknown};
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedItemId;

fn post(id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(id),
    }
}

fn feed(id: u64) -> ReaderSavedKey {
    ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(id),
    }
}

fn url(url: &str) -> ReaderSavedKey {
    ReaderSavedKey::Url {
        url: url.to_string(),
    }
}

fn refresh(state: &mut ReaderSavedState, at: Instant, keys: &[ReaderSavedKey], total: u64) {
    let ticket = state.begin_refresh(at);
    state.apply_refresh(ticket, keys.iter().map(|key| (key.clone(), None)), total);
}

fn loaded(at: Instant, keys: &[ReaderSavedKey]) -> ReaderSavedState {
    let mut state = ReaderSavedState::default();
    refresh(&mut state, at, keys, keys.len() as u64);
    state
}

/// Starts a tap with one write.
fn tap(state: &mut ReaderSavedState, key: &ReaderSavedKey, saved: bool) -> WriteId {
    let tap = state.begin_tap();
    state.begin_write(tap, key, saved, true)
}

fn saved(state: &mut ReaderSavedState, key: &ReaderSavedKey, stored: &ReaderSavedKey, at: Instant) {
    let write = tap(state, key, true);
    state.complete_write(write, stored, None, at);
}

fn unsaved(state: &mut ReaderSavedState, key: &ReaderSavedKey, at: Instant) {
    let write = tap(state, key, false);
    state.complete_write(write, key, None, at);
}

const LAG: Duration = MAX_READ_LAG;
const S: Duration = Duration::from_secs(1);

// Writes and reads

#[test]
fn a_post_is_unknown_before_the_first_refresh() {
    let state = ReaderSavedState::default();
    assert_eq!(state.status(&[post(1)]), Unknown);
    assert_eq!(state.saved_count(), None);
}

#[test]
fn a_write_shows_straight_away_and_while_in_flight() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let write = tap(&mut state, &post(1), true);
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));

    // A read that started while the write was in flight can't undo it.
    refresh(&mut state, now + LAG * 10, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved);
    state.complete_write(write, &post(1), None, now + LAG * 10);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn a_refused_write_is_undone() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    let write = tap(&mut state, &post(1), false);
    state.fail_write(write);
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));
}

/// Cancelled after its request went out: the server may have applied it, so it stands until a read
/// that includes it.
#[test]
fn a_write_cancelled_after_it_was_sent_stands_until_a_read_says() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let write = tap(&mut state, &post(1), true);
    state.abandon_write(write, now);
    assert_eq!(state.status(&[post(1)]), Saved);

    refresh(&mut state, now + LAG - S, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved, "this read may predate it");

    refresh(&mut state, now + LAG, &[], 0);
    assert_eq!(
        state.status(&[post(1)]),
        NotSaved,
        "it never reached the server"
    );
}

#[test]
fn time_passing_changes_nothing_on_its_own() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    // No read has included the save, so it still decides, however long ago it was.
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.recent_saves(), vec![post(1)]);
}

#[test]
fn a_read_that_started_inside_the_window_cannot_undo_a_write_however_late_it_lands() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    refresh(&mut state, now + LAG - S, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_read_that_started_after_the_window_decides() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    // Unsaved elsewhere since. A read this late includes that.
    refresh(&mut state, now + LAG, &[], 0);
    assert_eq!(state.status(&[post(1)]), NotSaved);
    assert!(state.recent_saves().is_empty());
}

#[test]
fn an_older_read_landing_after_a_newer_one_is_ignored() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let older = state.begin_refresh(now + S);
    let newer = state.begin_refresh(now + S * 2);
    state.apply_refresh(newer, [(post(1), None)], 1);
    state.apply_refresh(older, [], 0);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn reads_prune_the_writes_they_include() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    for id in 1..=50 {
        saved(&mut state, &post(id), &post(id), now);
    }
    refresh(&mut state, now + LAG, &[], 0);
    assert!(state.writes.is_empty());
}

// Order

/// A write a tap adds once it learns another key comes before every later tap's.
#[test]
fn a_write_added_to_a_tap_comes_before_later_taps() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let unsave = state.begin_tap();
    state.begin_write(unsave, &feed(4), false, true);
    tap(&mut state, &post(3), true);
    state.begin_write(unsave, &post(3), false, true);

    assert_eq!(state.status(&[post(3)]), Saved, "the later tap was save");
}

// Keys

#[test]
fn a_post_is_saved_if_any_of_its_keys_is() {
    let now = Instant::now();
    let state = loaded(now, &[feed(2)]);
    assert_eq!(state.status(&[post(1), feed(2)]), Saved);
}

#[test]
fn unsave_sends_every_key_and_the_keys_they_are_stored_as() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    saved(&mut state, &feed(20), &post(2), now);
    assert_eq!(state.keys_to_unsave(&[feed(20)]), vec![feed(20), post(2)]);
    assert_eq!(
        state.keys_to_unsave(&[post(1), feed(2)]),
        vec![post(1), feed(2)]
    );
}

#[test]
fn a_partly_failed_unsave_leaves_the_post_saved_under_the_key_that_failed() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1), feed(2)]);
    let unsave = state.begin_tap();
    let by_post = state.begin_write(unsave, &post(1), false, true);
    let by_feed = state.begin_write(unsave, &feed(2), false, true);
    state.complete_write(by_feed, &feed(2), None, now);
    state.fail_write(by_post);

    assert_eq!(state.status(&[post(1), feed(2)]), Saved);
    assert_eq!(state.status(&[feed(2)]), NotSaved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_post_saved_under_another_key_reads_saved_under_the_key_it_was_saved_by() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &feed(7), &post(1), now);
    assert_eq!(state.status(&[feed(7)]), Saved);
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));
    assert_eq!(state.recent_saves(), vec![post(1)]);

    refresh(&mut state, now + LAG, &[post(1)], 1);
    assert_eq!(state.status(&[feed(7)]), Saved);
}

#[test]
fn a_key_the_server_stops_storing_as_another_is_looked_up_as_itself_again() {
    let now = Instant::now();
    let page = url("https://example.com/p3");
    let mut state = loaded(now, &[]);
    saved(&mut state, &page, &post(3), now);
    unsaved(&mut state, &post(3), now);

    // Post 3 is deleted, and the URL is saved as itself.
    let later = now + LAG * 5;
    refresh(&mut state, later, &[], 0);
    saved(&mut state, &page, &page, later);
    refresh(&mut state, later + LAG, std::slice::from_ref(&page), 1);

    assert_eq!(state.status(&[post(3)]), NotSaved);
    assert_eq!(state.status(std::slice::from_ref(&page)), Saved);
    assert_eq!(
        state.keys_to_unsave(std::slice::from_ref(&page)),
        vec![page]
    );
}

/// Deleting a URL that isn't saved echoes the URL, as the live API does.
#[test]
fn an_unsave_that_echoes_its_key_does_not_forget_where_the_post_is_stored() {
    let now = Instant::now();
    let page = url("https://example.com/p3");
    let mut state = loaded(now, &[]);
    saved(&mut state, &page, &post(3), now);
    unsaved(&mut state, &page, now);
    assert_eq!(state.post_keys(std::slice::from_ref(&page)), vec![post(3)]);
}

#[test]
fn a_refresh_teaches_the_url_of_a_post_saved_by_its_ids() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);
    state.apply_refresh(
        ticket,
        [(post(3), Some("https://example.com/p3/".to_string()))],
        1,
    );
    let page = url("https://example.com/p3?utm_source=x");
    assert_eq!(state.status(std::slice::from_ref(&page)), Saved);
    assert_eq!(
        state.keys_to_unsave(std::slice::from_ref(&page)),
        vec![page, post(3)]
    );
}

/// A refresh says a URL is a feed item's, and a write that the feed item is stored as a post.
#[test]
fn what_a_key_is_stored_as_follows_a_chain() {
    let now = Instant::now();
    let page = url("https://example.com/item");
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);
    state.apply_refresh(
        ticket,
        [(feed(4), Some("https://example.com/item".to_string()))],
        1,
    );
    saved(&mut state, &feed(4), &post(3), now);
    assert_eq!(state.post_keys(std::slice::from_ref(&page)), vec![post(3)]);
}

#[test]
fn known_saved_under_counts_only_answered_writes_and_the_keys() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    tap(&mut state, &post(1), false);
    assert!(
        state.known_saved_under(&post(1)),
        "the unsave isn't answered"
    );
    assert!(!state.known_saved_under(&feed(2)));
    saved(&mut state, &feed(2), &feed(2), now);
    assert!(state.known_saved_under(&feed(2)));
}

// The saved list

#[test]
fn a_listed_post_counts_as_saved_before_the_keys_catch_up() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    assert!(state.note_listed_post(&[post(1)], now + S));
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_listed_jetpack_post_already_saved_under_its_feed_item_counts_once() {
    let now = Instant::now();
    let mut state = loaded(now, &[feed(2)]);
    assert!(!state.note_listed_post(&[post(1), feed(2)], now + S));
    assert_eq!(state.saved_count(), Some(1));
}

/// Reads lag, so a refresh that started soon after a list read may predate it.
#[test]
fn a_list_read_stands_until_a_refresh_that_started_long_enough_after_it() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    assert!(state.note_listed_post(&[post(1)], now));
    refresh(&mut state, now + LAG - S, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved);
    assert_eq!(state.saved_count(), Some(1));

    refresh(&mut state, now + LAG, &[], 0);
    assert_eq!(state.status(&[post(1)]), NotSaved);
}

/// The list read started first but landed after a refresh that started just after it, which may
/// predate it.
#[test]
fn a_list_read_landing_after_a_refresh_that_started_just_after_it_still_counts() {
    let now = Instant::now();
    let mut state = loaded(now + S, &[]);
    assert!(state.note_listed_post(&[post(1)], now));
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn a_list_read_long_before_the_keys_is_ignored() {
    let now = Instant::now();
    let mut state = loaded(now + LAG, &[]);
    assert!(!state.note_listed_post(&[post(1)], now));
    assert_eq!(state.status(&[post(1)]), NotSaved);
}

/// The keys' total doesn't include a post only the list has shown, so a write to it counts from
/// the keys.
#[test]
fn a_write_to_a_listed_post_the_keys_do_not_have_yet_counts_once() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    state.note_listed_post(&[post(1)], now + S);
    assert_eq!(state.saved_count(), Some(1));

    let write = tap(&mut state, &post(1), true);
    state.abandon_write(write, now + S);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_slow_list_read_cannot_replace_a_newer_one() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    state.note_listed_post(&[post(1)], now + S * 10);
    assert!(!state.note_listed_post(&[post(1)], now));
    // The keys read started between the two list reads, so the newer one stands.
    refresh(&mut state, now + S, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn a_post_unsaved_here_is_hidden_from_a_list_read_that_may_predate_the_unsave() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    unsaved(&mut state, &post(1), now);
    assert!(!state.should_show_listed_post(&[post(1)], now + S));
    assert!(!state.should_show_listed_post(&[post(1)], now + LAG - S));
}

/// A list read that started long after the unsave includes it, so a post it shows was saved again.
#[test]
fn a_list_read_long_after_an_unsave_shows_the_post_saved_again() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    unsaved(&mut state, &post(1), now);

    assert!(state.should_show_listed_post(&[post(1)], now + LAG));
    state.note_listed_post(&[post(1)], now + LAG);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn a_post_the_list_shows_leaves_recent_saves() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    saved(&mut state, &post(2), &post(2), now);

    assert!(state.note_listed_post(&[post(1)], now + S));
    assert_eq!(state.recent_saves(), vec![post(2)]);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn an_import_is_not_a_recent_save() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let import = state.begin_tap();
    let write = state.begin_write(import, &post(1), true, false);
    state.complete_write(write, &post(1), Some(1), now);
    assert!(state.recent_saves().is_empty());
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn recent_saves_are_newest_first_and_leave_out_posts_unsaved_since() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    saved(&mut state, &post(2), &post(2), now);
    saved(&mut state, &post(3), &post(3), now);
    unsaved(&mut state, &post(2), now);
    assert_eq!(state.recent_saves(), vec![post(3), post(1)]);
}

// Past the 1,000 keys

#[test]
fn past_the_key_limit_an_unseen_post_is_unknown_and_a_listed_one_saved() {
    let now = Instant::now();
    let keys: Vec<_> = (1..=1000).map(post).collect();
    let mut state = ReaderSavedState::default();
    refresh(&mut state, now, &keys, 1200);

    assert_eq!(state.status(&[post(5000)]), Unknown);
    state.note_listed_post(&[post(5000)], now + S);
    assert_eq!(state.status(&[post(5000)]), Saved);
    // It may be one of the 200 already counted.
    assert_eq!(state.saved_count(), Some(1200));

    // A refresh can't see past the keys, so the list still says.
    refresh(&mut state, now + S * 2, &keys, 1200);
    assert_eq!(state.status(&[post(5000)]), Saved);
}

#[test]
fn past_the_key_limit_unsaving_a_listed_post_counts() {
    let now = Instant::now();
    let keys: Vec<_> = (1..=1000).map(post).collect();
    let mut state = ReaderSavedState::default();
    refresh(&mut state, now, &keys, 1200);
    state.note_listed_post(&[post(5000)], now + S);

    unsaved(&mut state, &post(5000), now + S * 2);
    assert_eq!(state.saved_count(), Some(1199));
}

// Counting

#[test]
fn a_post_saved_under_two_keys_counts_twice_like_the_server_total() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1), feed(2)]);
    assert_eq!(state.saved_count(), Some(2));
    unsaved(&mut state, &post(1), now);
    unsaved(&mut state, &feed(2), now);
    assert_eq!(state.saved_count(), Some(0));
}

/// An older save is under each of feed item 4 and post 3, and the device has learned they're one
/// post. Unsaving both counts both.
#[test]
fn unsaving_both_saves_of_a_post_under_aliased_keys_counts_both() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(3), feed(4)]);
    saved(&mut state, &feed(4), &post(3), now);
    assert_eq!(state.post_keys(&[feed(4)]), vec![post(3)]);
    assert_eq!(state.saved_count(), Some(2));

    let unsave = state.begin_tap();
    let by_post = state.begin_write(unsave, &post(3), false, true);
    let by_feed = state.begin_write(unsave, &feed(4), false, true);
    state.complete_write(by_post, &post(3), None, now);
    state.complete_write(by_feed, &feed(4), None, now);
    assert_eq!(state.saved_count(), Some(0));
}

/// The server's total in a write's answer is newer than the keys, so it's the count.
#[test]
fn a_write_answer_reports_the_count() {
    let now = Instant::now();
    let keys: Vec<_> = (1..=1000).map(post).collect();
    let mut state = ReaderSavedState::default();
    refresh(&mut state, now, &keys, 1200);

    // Post 5000 was already one of the 1,200.
    let write = tap(&mut state, &post(5000), true);
    assert_eq!(state.saved_count(), Some(1201), "not answered yet");
    state.complete_write(write, &post(5000), Some(1200), now);
    assert_eq!(state.saved_count(), Some(1200));

    // A write still in flight counts on top of the answered total.
    tap(&mut state, &post(6000), true);
    assert_eq!(state.saved_count(), Some(1201));

    // A stale refresh doesn't replace the answered total.
    refresh(&mut state, now + S, &keys, 1199);
    assert_eq!(state.saved_count(), Some(1201));
}

#[test]
fn a_write_answer_gives_a_count_before_the_first_refresh() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let write = tap(&mut state, &post(1), true);
    state.complete_write(write, &post(1), Some(7), now);
    assert_eq!(state.saved_count(), Some(7));
}
