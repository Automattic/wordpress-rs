//! The rules, one example each. `fake_server_tests.rs` drives them through random sessions.

use super::*;
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

fn saved(state: &mut ReaderSavedState, key: &ReaderSavedKey, stored: &ReaderSavedKey, at: Instant) {
    let write = state.begin_write(key, true);
    state.complete_write(write, stored, at);
}

fn unsaved(state: &mut ReaderSavedState, key: &ReaderSavedKey, at: Instant) {
    let write = state.begin_write(key, false);
    state.complete_write(write, key, at);
}

const LAG: Duration = MAX_READ_LAG;
const S: Duration = Duration::from_secs(1);

#[test]
fn a_post_is_unknown_before_the_first_refresh() {
    let state = ReaderSavedState::default();
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Unknown);
    assert_eq!(state.saved_count(), None);
}

#[test]
fn a_write_shows_straight_away_and_while_in_flight() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    let write = state.begin_write(&post(1), true);
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(), Some(1));

    // A read that started while the write was in flight can't undo it.
    refresh(&mut state, now + LAG * 10, &[], 0);
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    state.complete_write(write, &post(1), now + LAG * 10);
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
}

#[test]
fn a_failed_write_is_undone() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    let write = state.begin_write(&post(1), false);
    state.fail_write(write);
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn time_passing_changes_nothing_on_its_own() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);

    // No read has included the save, so it still decides, however long ago it was.
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.recent_saves(), vec![post(1)]);
}

#[test]
fn a_read_that_started_inside_the_window_cannot_undo_a_write_however_late_it_lands() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);

    let ticket = state.begin_refresh(now + LAG - S);
    state.apply_refresh(ticket, [], 0);

    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_read_that_started_after_the_window_decides() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &post(1), &post(1), now);

    // The save was undone elsewhere since. A read this late includes that.
    refresh(&mut state, now + LAG, &[], 0);

    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::NotSaved);
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
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
}

#[test]
fn a_post_is_saved_if_any_of_its_keys_is() {
    let now = Instant::now();
    let state = loaded(now, &[feed(2)]);
    assert_eq!(state.status(&[post(1), feed(2)]), ReaderSavedStatus::Saved);
}

#[test]
fn unsave_sends_every_key_including_the_one_the_server_stores_the_post_as() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    saved(&mut state, &feed(20), &post(2), now);

    // No refresh yet, so nothing says which key is saved.
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
    let by_post = state.begin_write(&post(1), false);
    unsaved(&mut state, &feed(2), now);
    state.fail_write(by_post);

    assert_eq!(state.status(&[post(1), feed(2)]), ReaderSavedStatus::Saved);
    assert_eq!(state.status(&[feed(2)]), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_post_saved_under_another_key_reads_saved_under_the_key_it_was_saved_by() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    saved(&mut state, &feed(7), &post(1), now);

    assert_eq!(state.status(&[feed(7)]), ReaderSavedStatus::Saved);
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(), Some(1));
    assert_eq!(state.recent_saves(), vec![post(1)]);

    // And once the server has read it, too.
    refresh(&mut state, now + LAG, &[post(1)], 1);
    assert_eq!(state.status(&[feed(7)]), ReaderSavedStatus::Saved);
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

    assert_eq!(state.status(&[post(3)]), ReaderSavedStatus::NotSaved);
    assert_eq!(
        state.status(std::slice::from_ref(&page)),
        ReaderSavedStatus::Saved
    );
    assert_eq!(
        state.keys_to_unsave(std::slice::from_ref(&page)),
        vec![page]
    );
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
    assert_eq!(
        state.status(std::slice::from_ref(&page)),
        ReaderSavedStatus::Saved
    );
    assert_eq!(
        state.keys_to_unsave(std::slice::from_ref(&page)),
        vec![page, post(3)]
    );
}

#[test]
fn beyond_the_key_limit_an_unseen_post_is_unknown_and_a_listed_one_saved() {
    let now = Instant::now();
    let keys: Vec<_> = (1..=1000).map(post).collect();
    let mut state = ReaderSavedState::default();
    refresh(&mut state, now, &keys, 1200);

    assert_eq!(state.status(&[post(5000)]), ReaderSavedStatus::Unknown);
    assert_eq!(state.status(&[post(5001)]), ReaderSavedStatus::Unknown);
    state.note_listed_post(&[post(5000)], now + S);
    assert_eq!(state.status(&[post(5000)]), ReaderSavedStatus::Saved);
    // It may be one of the 200 already counted.
    assert_eq!(state.saved_count(), Some(1200));
}

#[test]
fn a_listed_post_counts_as_saved_before_the_keys_catch_up() {
    let now = Instant::now();
    let mut state = loaded(now, &[]);
    assert!(state.note_listed_post(&[post(1)], now + S));
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_listed_jetpack_post_already_saved_under_its_feed_item_counts_once() {
    let now = Instant::now();
    let mut state = loaded(now, &[feed(2)]);
    assert!(!state.note_listed_post(&[post(1), feed(2)], now + S));
    assert_eq!(state.saved_count(), Some(1));
}

#[test]
fn a_list_read_that_started_before_the_keys_is_ignored() {
    let now = Instant::now();
    let mut state = loaded(now + S, &[]);
    assert!(!state.note_listed_post(&[post(1)], now));
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::NotSaved);
}

#[test]
fn a_post_unsaved_here_is_hidden_from_the_list_until_a_read_includes_it() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    unsaved(&mut state, &post(1), now);
    assert!(!state.should_show_listed_post(&[post(1)]));

    // A list read this late includes the unsave, so showing the post means it was saved again.
    assert!(state.should_show_listed_post(&[post(2)]));
    refresh(&mut state, now + LAG, &[post(1)], 1);
    assert!(state.should_show_listed_post(&[post(1)]));
    assert_eq!(state.status(&[post(1)]), ReaderSavedStatus::Saved);
}

#[test]
fn a_post_saved_under_two_keys_counts_twice_like_the_server_total() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1), feed(2)]);
    assert_eq!(state.saved_count(), Some(2));
    unsaved(&mut state, &post(1), now);
    unsaved(&mut state, &feed(2), now);
    assert_eq!(state.saved_count(), Some(0));
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

#[test]
fn known_saved_under_ignores_the_write_being_sent() {
    let now = Instant::now();
    let mut state = loaded(now, &[post(1)]);
    state.begin_write(&post(1), false);
    assert!(state.known_saved_under(&post(1)));
    assert!(!state.known_saved_under(&feed(2)));
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
