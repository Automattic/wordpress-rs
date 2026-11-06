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

/// Starts a tap with one write. A save tap is a recent save.
fn tap(state: &mut ReaderSavedState, key: &ReaderSavedKey, saved: bool) -> WriteId {
    let tap = state.begin_tap();
    if saved {
        state.note_save_tap(key);
    }
    state.begin_write(tap, key, saved)
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

/// Given up on after its request went out: the server may still apply it, up to `MAX_READ_LAG`
/// later, so it stands until a read that includes that.
#[test]
fn a_write_given_up_on_after_it_was_sent_stands_until_a_read_says() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let write = tap(&mut state, &post(1), true);
    state.abandon_write(write, now);
    assert_eq!(state.status(&[post(1)]), Saved);

    refresh(&mut state, now + LAG * 2 - S, &[], 0);
    assert_eq!(state.status(&[post(1)]), Saved, "this read may predate it");

    refresh(&mut state, now + LAG * 2, &[], 0);
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
    state.begin_write(unsave, &feed(4), false);
    tap(&mut state, &post(3), true);
    state.begin_write(unsave, &post(3), false);

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
    let by_post = state.begin_write(unsave, &post(1), false);
    let by_feed = state.begin_write(unsave, &feed(2), false);
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

/// A newer write decides, so a read that includes it includes the older ones too, even one that
/// was given up on and so waits longer.
#[test]
fn a_read_that_includes_a_write_includes_the_older_writes_to_its_key() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    let unsave = tap(&mut state, &post(1), false);
    state.abandon_write(unsave, now);
    saved(&mut state, &post(1), &post(1), now + S);

    refresh(&mut state, now + S + LAG, &[post(1)], 1);
    assert!(state.writes.is_empty());
    assert_eq!(state.status(&[post(1)]), Saved);
}

// The saved list

#[test]
fn the_saved_list_does_not_say_a_post_is_saved() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    assert!(!state.note_listed_post(&[post(1)]));
    assert_eq!(state.status(&[post(1)]), NotSaved);
    assert_eq!(state.saved_count(), Some(0));
}

#[test]
fn a_post_unsaved_here_is_hidden_from_a_list_read_that_may_predate_the_unsave() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    unsaved(&mut state, &post(1), now);
    assert!(!state.should_show_listed_post(&[post(1)], now + S));
    assert!(!state.should_show_listed_post(&[post(1)], now + LAG - S));
    assert!(state.should_show_listed_post(&[post(1)], now + LAG));
}

#[test]
fn a_post_unsaved_here_and_given_up_on_is_hidden_twice_as_long() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    let write = tap(&mut state, &post(1), false);
    state.abandon_write(write, now);
    assert!(!state.should_show_listed_post(&[post(1)], now + LAG * 2 - S));
    assert!(state.should_show_listed_post(&[post(1)], now + LAG * 2));
}

/// Unsaved by the feed item alone, while the server holds the post under it.
#[test]
fn a_jetpack_post_unsaved_by_its_feed_item_is_hidden() {
    let now = Instant::now();
    let mut state = loaded(now, &[feed(2)]);
    unsaved(&mut state, &feed(2), now);
    assert!(!state.should_show_listed_post(&[post(1), feed(2)], now + S));
}

/// Unsaved by its post key, but still saved under its feed item.
#[test]
fn a_jetpack_post_still_saved_under_another_key_is_shown() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1), feed(2)]);
    unsaved(&mut state, &post(1), now);
    assert_eq!(state.status(&[post(1), feed(2)]), Saved);
    assert!(state.should_show_listed_post(&[post(1), feed(2)], now + S));
}

// Recent saves

#[test]
fn a_post_the_list_shows_leaves_recent_saves() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    saved(&mut state, &post(2), &post(2), now);

    assert!(state.note_listed_post(&[post(1)]));
    assert_eq!(state.recent_saves(), vec![post(2)]);
    assert_eq!(state.status(&[post(1)]), Saved);
}

#[test]
fn an_import_is_not_a_recent_save() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let import = state.begin_tap();
    let write = state.begin_write(import, &post(1), true);
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

/// A refresh that includes the save says nothing about whether the list has shown it.
#[test]
fn a_refresh_does_not_take_a_post_out_of_recent_saves() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    refresh(&mut state, now + LAG, &[post(1)], 1);
    assert_eq!(state.recent_saves(), vec![post(1)]);
}

/// The unsave was refused, so the post is saved again, and still recent.
#[test]
fn a_refused_unsave_puts_the_post_back_in_recent_saves() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    let unsave = tap(&mut state, &post(1), false);
    assert!(state.recent_saves().is_empty());
    state.fail_write(unsave);
    assert_eq!(state.recent_saves(), vec![post(1)]);
}

/// Saved by a URL the device doesn't know is post 9 yet, then unsaved from post 9's card.
#[test]
fn a_post_saved_by_url_and_unsaved_by_its_ids_leaves_recent_saves() {
    let now = Instant::now();
    let page = url("https://example.com/p9");
    let mut state = loaded(now, &[]);
    let save = tap(&mut state, &page, true);
    let unsave = tap(&mut state, &post(9), false);
    state.complete_write(save, &post(9), Some(1), now);
    state.complete_write(unsave, &post(9), Some(0), now);
    assert!(state.recent_saves().is_empty());
}

/// Saved by a URL before the device knew it's post 9, then by its IDs: one post, one recent save.
#[test]
fn a_post_saved_twice_under_keys_found_to_be_one_is_one_recent_save() {
    let now = Instant::now();
    let page = url("https://example.com/p9");
    let mut state = loaded(now, &[]);
    let by_url = tap(&mut state, &page, true);
    let by_ids = tap(&mut state, &post(9), true);
    state.complete_write(by_url, &post(9), Some(1), now);
    state.complete_write(by_ids, &post(9), Some(1), now);
    assert_eq!(state.recent_saves(), vec![post(9)]);
}

// Past the 1,000 keys

fn past_the_keys(at: Instant) -> ReaderSavedState {
    let keys: Vec<_> = (1..=1000).map(post).collect();
    let mut state = ReaderSavedState::default();
    refresh(&mut state, at, &keys, 1002);
    state
}

#[test]
fn past_the_key_limit_a_post_the_keys_do_not_have_is_unknown() {
    let now = Instant::now();
    let state = past_the_keys(now);
    assert_eq!(state.status(&[post(5000)]), Unknown);
    assert_eq!(state.status(&[post(1)]), Saved);
}

/// Two saved URLs that normalize to one are still two of the total, so nothing is past the keys.
#[test]
fn keys_that_normalize_to_one_are_not_past_the_key_limit() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let keys = [url("https://example.com/a"), url("https://example.com/a/")];
    refresh(&mut state, now, &keys, 2);
    assert_eq!(state.status(&[post(1)]), NotSaved);
}

/// The refresh can't see post 5000, so it can't say the answered write is out of date.
#[test]
fn past_the_key_limit_an_answered_write_to_a_post_the_keys_do_not_have_stands() {
    let now = Instant::now();
    let mut state = past_the_keys(now);
    unsaved(&mut state, &post(5000), now);
    saved(&mut state, &post(6000), &post(6000), now);
    for later in [now + LAG, now + LAG * 10] {
        refresh(
            &mut state,
            later,
            &(1..=1000).map(post).collect::<Vec<_>>(),
            1002,
        );
        assert_eq!(state.status(&[post(5000)]), NotSaved);
        assert_eq!(state.status(&[post(6000)]), Saved);
        assert_eq!(state.saved_count(), Some(1002));
    }
}

/// Given up on, so it may never have reached the server: past the key limit that's unknown.
#[test]
fn past_the_key_limit_a_write_given_up_on_reads_unknown_once_a_read_includes_it() {
    let now = Instant::now();
    let mut state = past_the_keys(now);
    let write = tap(&mut state, &post(5000), false);
    state.abandon_write(write, now);
    refresh(
        &mut state,
        now + LAG * 2,
        &(1..=1000).map(post).collect::<Vec<_>>(),
        1002,
    );
    assert_eq!(state.status(&[post(5000)]), Unknown);
}

#[test]
fn back_within_the_key_limit_a_post_the_keys_do_not_have_is_not_saved() {
    let now = Instant::now();
    let mut state = past_the_keys(now);
    unsaved(&mut state, &post(5000), now);
    let keys: Vec<_> = (1..=1000).map(post).collect();
    refresh(&mut state, now + LAG, &keys, 1000);
    assert!(state.writes.is_empty());
    assert_eq!(state.status(&[post(5000)]), NotSaved);
    assert_eq!(state.status(&[post(6000)]), NotSaved);
}

/// The keys' total may or may not count post 5000, so a write to it can't change the count until
/// its answer gives one.
#[test]
fn past_the_key_limit_a_write_to_a_post_the_keys_do_not_have_counts_once_answered() {
    let now = Instant::now();
    let mut state = past_the_keys(now);
    let write = tap(&mut state, &post(5000), false);
    assert_eq!(state.saved_count(), Some(1002));
    state.complete_write(write, &post(5000), Some(1001), now);
    assert_eq!(state.saved_count(), Some(1001));
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
    let by_post = state.begin_write(unsave, &post(3), false);
    let by_feed = state.begin_write(unsave, &feed(4), false);
    state.complete_write(by_post, &post(3), None, now);
    state.complete_write(by_feed, &feed(4), None, now);
    assert_eq!(state.saved_count(), Some(0));
}

/// The server's total in a write's answer is newer than the keys, so it's the count.
#[test]
fn a_write_answer_reports_the_count() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1), post(2)]);

    // Post 3 was saved elsewhere since the keys were read.
    let write = tap(&mut state, &post(4), true);
    assert_eq!(state.saved_count(), Some(3), "not answered yet");
    state.complete_write(write, &post(4), Some(4), now);
    assert_eq!(state.saved_count(), Some(4));

    // A write still in flight counts on top of the answered total.
    tap(&mut state, &post(5), true);
    assert_eq!(state.saved_count(), Some(5));

    // A refresh that started before the answer, but landed after it, doesn't replace it.
    let older = state.begin_refresh(now - S);
    state.apply_refresh(older, [(post(1), None), (post(2), None)], 2);
    assert_eq!(state.saved_count(), Some(5));

    // One that started after it does, with its own keys.
    let keys = [post(1), post(2), post(3), post(4)];
    refresh(&mut state, now + S, &keys, 4);
    assert_eq!(state.saved_count(), Some(5));
    refresh(
        &mut state,
        now + S * 2,
        &[post(1), post(2), post(3), post(4), post(5)],
        5,
    );
    assert_eq!(state.saved_count(), Some(5));
}

/// A save that timed out was applied, and a refresh after the last answer already has it.
#[test]
fn a_refresh_after_an_answer_counts_a_write_it_already_has_once() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);
    let write = tap(&mut state, &post(2), true);
    state.abandon_write(write, now + S);
    refresh(&mut state, now + S * 5, &[post(1), post(2)], 2);
    assert_eq!(state.saved_count(), Some(2));
}

/// The answered total already includes a write given up on before it, if the server applied it.
#[test]
fn a_write_given_up_on_before_an_answered_one_is_not_counted_twice() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let first = tap(&mut state, &post(1), true);
    state.abandon_write(first, now);
    let second = tap(&mut state, &post(2), true);
    state.complete_write(second, &post(2), Some(2), now + S);
    assert_eq!(state.saved_count(), Some(2));
}

/// A write answered without a total, after the one whose total is the count.
#[test]
fn a_write_answered_without_a_total_counts_on_top_of_the_last_total() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    saved(&mut state, &post(2), &post(2), now);
    let with_total = tap(&mut state, &post(3), true);
    state.complete_write(with_total, &post(3), Some(3), now);
    unsaved(&mut state, &post(1), now);
    assert_eq!(state.saved_count(), Some(2));
}

#[test]
fn a_write_answer_gives_a_count_before_the_first_refresh() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let write = tap(&mut state, &post(1), true);
    state.complete_write(write, &post(1), Some(7), now);
    assert_eq!(state.saved_count(), Some(7));
}

// Counting after a refresh, and writes to two keys of one post

/// The keys said 0. Save P1 is answered with a total of 1. Save P2 times out, but the server
/// applied it. A refresh 5s later reads both saves: the server holds 2.
#[test]
fn a_refresh_after_an_answer_counts_a_save_given_up_on_that_it_has_once() {
    let t0 = Instant::now();
    let mut state = loaded(t0, &[]);
    let first = tap(&mut state, &post(1), true);
    state.complete_write(first, &post(1), Some(1), t0);
    let second = tap(&mut state, &post(2), true);
    state.abandon_write(second, t0 + S);
    assert_eq!(state.saved_count(), Some(2), "before the refresh");

    refresh(&mut state, t0 + S * 5, &[post(1), post(2)], 2);
    assert_eq!(state.saved_count(), Some(2), "the server holds 2");
}

/// The same with an unsave: the keys said P5. Save P1 is answered with 2. Unsave P5 times out but
/// was applied. A refresh 5s later reads P1 alone: the server holds 1.
#[test]
fn a_refresh_after_an_answer_counts_an_unsave_given_up_on_that_it_has_once() {
    let t0 = Instant::now();
    let mut state = loaded(t0, &[post(5)]);
    let first = tap(&mut state, &post(1), true);
    state.complete_write(first, &post(1), Some(2), t0);
    let second = tap(&mut state, &post(5), false);
    state.abandon_write(second, t0 + S);
    assert_eq!(state.saved_count(), Some(1), "before the refresh");

    refresh(&mut state, t0 + S * 5, &[post(1)], 1);
    assert_eq!(state.saved_count(), Some(1), "the server holds 1");
}

/// The same while the second save is still in flight: the server applied it, its answer is slow,
/// and a refresh lands first.
#[test]
fn a_refresh_that_has_a_write_still_in_flight_counts_it_once() {
    let t0 = Instant::now();
    let mut state = loaded(t0, &[]);
    let first = tap(&mut state, &post(1), true);
    state.complete_write(first, &post(1), Some(1), t0);
    let _second = tap(&mut state, &post(2), true);
    assert_eq!(state.saved_count(), Some(2));
    refresh(&mut state, t0 + S, &[post(1), post(2)], 2);
    assert_eq!(state.saved_count(), Some(2), "the server holds 2");
}

/// A Jetpack post saved under both P3 and F4, and the device knows F4 is stored as P3. Unsaving it
/// sends delete P3, which times out (the server applies it 30s later), then delete F4, answered.
/// A read that started LAG after F4's answer includes F4's delete, but not necessarily the late P3
/// delete, which fact 4 only covers from 2 x LAG after giving up.
#[test]
fn a_read_that_includes_one_key_keeps_a_write_given_up_on_to_another_key_of_the_post() {
    let t0 = Instant::now();
    let mut state = loaded(t0, &[feed(4)]);
    saved(&mut state, &feed(4), &post(3), t0);
    let t1 = t0 + LAG;
    refresh(&mut state, t1, &[post(3), feed(4)], 2);
    assert!(state.writes.is_empty());
    assert_eq!(
        state.keys_to_unsave(&[post(3), feed(4)]),
        vec![post(3), feed(4)]
    );

    let unsave = state.begin_tap();
    let by_post = state.begin_write(unsave, &post(3), false);
    let by_feed = state.begin_write(unsave, &feed(4), false);
    let t2 = t1 + S;
    state.abandon_write(by_post, t2);
    state.complete_write(by_feed, &feed(4), Some(1), t2);
    assert_eq!(state.status(&[post(3), feed(4)]), NotSaved);

    refresh(&mut state, t2 + LAG, &[post(3)], 1);
    assert_eq!(
        state.status(&[post(3), feed(4)]),
        NotSaved,
        "the delete of P3 may still land, so it should stand until t2 + 2 x LAG"
    );
}

/// The keys have P9, with U9 as its URL. Saving U9 is a no-op on the server, so it counts once
/// while it's in flight.
#[test]
fn saving_a_url_already_saved_as_its_post_counts_once() {
    let t0 = Instant::now();
    let page = url("https://example.com/p9");
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(t0);
    state.apply_refresh(
        ticket,
        [(post(9), Some("https://example.com/p9".to_string()))],
        1,
    );
    assert_eq!(state.status(std::slice::from_ref(&page)), Saved);
    let _write = tap(&mut state, &page, true);
    assert_eq!(state.saved_count(), Some(1));
}

/// The external view (feed item 4) is saved and times out, but the server stores it as post 3.
/// Unsaving from the WordPress.com view deletes post 3, and the answers teach the device that feed
/// item 4 is stored as post 3, so the save given up on counts under post 3 and is undone with it.
#[test]
fn a_save_given_up_on_counts_under_the_key_the_server_stores_it_as() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let save = tap(&mut state, &feed(4), true);
    state.abandon_write(save, now);
    let later = now + S * 10;
    let unsave = state.begin_tap();
    let by_post = state.begin_write(unsave, &post(3), false);
    let by_feed = state.begin_write(unsave, &feed(4), false);
    state.complete_write(by_post, &post(3), Some(0), later);
    state.complete_write(by_feed, &post(3), Some(0), later);
    assert_eq!(state.saved_count(), Some(0));

    refresh(&mut state, later + LAG, &[], 0);
    assert_eq!(state.saved_count(), Some(0));
    assert_eq!(state.status(&[post(3), feed(4)]), NotSaved);
}

/// A post this device never unsaved isn't hidden because a refresh that started after the list
/// read doesn't have it yet.
#[test]
fn a_list_read_older_than_a_refresh_still_shows_a_post_this_device_did_not_unsave() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    refresh(&mut state, now + S, &[], 0);
    assert!(state.should_show_listed_post(&[post(1)], now));
}
