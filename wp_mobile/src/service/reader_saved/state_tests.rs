use super::*;
use rand::{RngExt, SeedableRng, rngs::StdRng};
use rstest::rstest;
use wp_api::posts::PostId;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::reader_saved::ReaderFeedItemId;

const SECOND: Duration = Duration::from_secs(1);

fn post(id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(id),
    }
}

fn feed_item(id: u64) -> ReaderSavedKey {
    ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(id),
    }
}

fn url(url: &str) -> ReaderSavedKey {
    ReaderSavedKey::Url {
        url: url.to_string(),
    }
}

/// State after a refresh that returned `keys`.
fn loaded(keys: &[ReaderSavedKey], now: Instant) -> ReaderSavedState {
    let mut state = ReaderSavedState::default();
    refresh(&mut state, keys, now);
    state
}

fn refresh(state: &mut ReaderSavedState, keys: &[ReaderSavedKey], now: Instant) {
    let ticket = state.begin_refresh(now);
    state.apply_refresh(ticket, keys.to_vec(), keys.len() as u64);
}

/// A write the server accepted.
fn write(state: &mut ReaderSavedState, key: &ReaderSavedKey, saved: bool, now: Instant) {
    let id = state.begin_write(key, saved);
    state.complete_write(key, id, key, now);
}

// Loading

#[test]
fn a_post_is_unknown_before_the_first_refresh() {
    let state = ReaderSavedState::default();
    assert_eq!(
        state.status(&[post(1)], Instant::now()),
        ReaderSavedStatus::Unknown
    );
}

#[test]
fn a_post_without_keys_is_not_saved_even_before_the_first_refresh() {
    let state = ReaderSavedState::default();
    assert_eq!(
        state.status(&[], Instant::now()),
        ReaderSavedStatus::NotSaved
    );
}

#[test]
fn the_count_is_unknown_before_the_first_refresh() {
    let mut state = ReaderSavedState::default();
    let now = Instant::now();
    write(&mut state, &post(1), true, now);
    assert_eq!(state.saved_count(now), None);
}

#[test]
fn a_refresh_loads_the_keys() {
    let now = Instant::now();
    let state = loaded(&[post(1), feed_item(2)], now);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.status(&[feed_item(2)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.status(&[post(3)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(now), Some(2));
}

#[test]
fn a_refresh_replaces_the_keys() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    refresh(&mut state, &[post(2)], now);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
}

#[test]
fn an_empty_refresh_means_nothing_is_saved_rather_than_unknown() {
    let now = Instant::now();
    let state = loaded(&[], now);
    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(now), Some(0));
}

#[test]
fn the_count_uses_the_servers_total_rather_than_the_capped_keys() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);

    state.apply_refresh(ticket, [post(1)], 1003);

    assert_eq!(state.saved_count(now), Some(1003));
}

// A post's keys

#[rstest]
#[case::neither(&[], &[], ReaderSavedStatus::NotSaved)]
#[case::main_key(&[1], &[], ReaderSavedStatus::Saved)]
#[case::other_key(&[], &[2], ReaderSavedStatus::Saved)]
#[case::both_keys(&[1], &[2], ReaderSavedStatus::Saved)]
fn a_post_is_saved_if_any_of_its_keys_is(
    #[case] posts: &[i64],
    #[case] feed_items: &[u64],
    #[case] expected: ReaderSavedStatus,
) {
    let now = Instant::now();
    let keys: Vec<_> = posts
        .iter()
        .map(|id| post(*id))
        .chain(feed_items.iter().map(|id| feed_item(*id)))
        .collect();
    let state = loaded(&keys, now);

    assert_eq!(state.status(&[post(1), feed_item(2)], now), expected);
}

#[test]
fn a_local_save_of_one_key_decides_before_the_first_refresh() {
    let mut state = ReaderSavedState::default();
    let now = Instant::now();

    state.begin_write(&post(1), true);

    assert_eq!(
        state.status(&[post(1), feed_item(2)], now),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn a_local_unsave_of_one_key_leaves_the_post_unknown_before_the_first_refresh() {
    let mut state = ReaderSavedState::default();
    let now = Instant::now();

    state.begin_write(&post(1), false);

    assert_eq!(
        state.status(&[post(1), feed_item(2)], now),
        ReaderSavedStatus::Unknown
    );
    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

#[rstest]
#[case::the_matching_key(&[post(1)], &[post(1)])]
#[case::the_other_key(&[feed_item(2)], &[feed_item(2)])]
#[case::both_keys(&[post(1), feed_item(2)], &[post(1), feed_item(2)])]
#[case::the_main_key_when_neither_match(&[post(9)], &[post(1)])]
fn unsaving_uses_the_keys_that_matched(
    #[case] saved: &[ReaderSavedKey],
    #[case] expected: &[ReaderSavedKey],
) {
    let now = Instant::now();
    let state = loaded(saved, now);

    assert_eq!(
        state.keys_to_unsave(&[post(1), feed_item(2)], now),
        expected
    );
}

#[test]
fn unsaving_before_the_first_refresh_uses_the_main_key() {
    let state = ReaderSavedState::default();
    assert_eq!(
        state.keys_to_unsave(&[post(1), feed_item(2)], Instant::now()),
        vec![post(1)]
    );
}

#[test]
fn unsaving_a_post_without_keys_sends_nothing() {
    let state = ReaderSavedState::default();
    assert!(state.keys_to_unsave(&[], Instant::now()).is_empty());
}

#[test]
fn unsaving_counts_a_recent_local_save() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    state.begin_write(&feed_item(2), true);

    assert_eq!(
        state.keys_to_unsave(&[post(1), feed_item(2)], now),
        vec![feed_item(2)]
    );
}

// Writes

#[test]
fn a_save_applies_before_the_request_finishes() {
    let now = Instant::now();
    let mut state = loaded(&[], now);

    state.begin_write(&post(1), true);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
}

#[test]
fn an_unsave_applies_before_the_request_finishes() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    state.begin_write(&post(1), false);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

#[test]
fn a_failed_save_is_undone() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&post(1), true);

    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(now), Some(0));
}

#[test]
fn a_failed_unsave_is_undone() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    let id = state.begin_write(&post(1), false);

    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn a_failed_write_before_the_first_refresh_goes_back_to_unknown() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let id = state.begin_write(&post(1), true);

    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Unknown);
}

#[test]
fn a_failed_write_goes_back_to_the_write_before_it() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    let unsave = state.begin_write(&post(1), false);

    state.fail_write(&post(1), unsave);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
}

#[test]
fn the_newest_write_wins_while_an_older_one_is_in_flight() {
    let now = Instant::now();
    let mut state = loaded(&[], now);

    state.begin_write(&post(1), true);
    state.begin_write(&post(1), false);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

#[test]
fn an_older_write_finishing_late_does_not_override_a_newer_one() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let save = state.begin_write(&post(1), true);
    let unsave = state.begin_write(&post(1), false);

    state.complete_write(&post(1), unsave, &post(1), now);
    state.complete_write(&post(1), save, &post(1), now);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

#[test]
fn an_older_write_finishing_late_does_not_flip_the_post_once_both_expire() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let save = state.begin_write(&post(1), true);
    let unsave = state.begin_write(&post(1), false);

    state.complete_write(&post(1), unsave, &post(1), now);
    state.complete_write(&post(1), save, &post(1), now);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(later), Some(0));
}

/// The newer write's response lands first, the older one's some seconds later. Once the newer one
/// expires the post must not flip back to the older action.
#[rstest]
#[case::save_then_unsave(true)]
#[case::unsave_then_save(false)]
fn an_older_write_finishing_late_never_takes_over(#[case] first: bool) {
    let now = Instant::now();
    let initial = if first { vec![] } else { vec![post(1)] };
    let mut state = loaded(&initial, now);
    let older = state.begin_write(&post(1), first);
    let newer = state.begin_write(&post(1), !first);

    state.complete_write(&post(1), newer, &post(1), now);
    state.complete_write(&post(1), older, &post(1), now + SECOND * 5);

    let want = if first {
        ReaderSavedStatus::NotSaved
    } else {
        ReaderSavedStatus::Saved
    };
    for seconds in [0, 60, 119, 121, 123, 124, 130, 300] {
        let at = now + SECOND * seconds;
        assert_eq!(state.status(&[post(1)], at), want, "at {seconds}s");
        assert_eq!(
            state.saved_count(at),
            Some(u64::from(!first)),
            "at {seconds}s"
        );
        // Only the newer save, and only during its window.
        let in_window = seconds < 120;
        assert_eq!(
            state.recent_saves(at).is_empty(),
            first || !in_window,
            "at {seconds}s"
        );
    }
}

/// A refresh landing between the two windows' ends used to bake the older action into the list.
#[test]
fn a_refresh_between_the_windows_keeps_the_newer_write() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let older = state.begin_write(&post(1), true);
    let newer = state.begin_write(&post(1), false);
    state.complete_write(&post(1), newer, &post(1), now);
    state.complete_write(&post(1), older, &post(1), now + SECOND * 5);

    // Fresh from the server, after the newer window: not saved.
    refresh(&mut state, &[], now + SECOND * 122);

    let later = now + SECOND * 400;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(later), Some(0));
}

/// An older write that hangs past the newer one's window still can't take over when it lands.
#[test]
fn a_write_that_hangs_past_a_newer_ones_window_is_ignored_when_it_lands() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let older = state.begin_write(&post(1), true);
    let newer = state.begin_write(&post(1), false);
    state.complete_write(&post(1), newer, &post(1), now);

    // A refresh after the newer window, while the older request is still in flight.
    refresh(&mut state, &[], now + SECOND * 200);
    state.complete_write(&post(1), older, &post(1), now + SECOND * 210);

    assert_eq!(
        state.status(&[post(1)], now + SECOND * 211),
        ReaderSavedStatus::NotSaved
    );
    assert_eq!(state.saved_count(now + SECOND * 211), Some(0));
}

/// The newer write's window ends first because it finished first. A refresh after that mustn't
/// leave the older write, whose window is still open, deciding the post.
#[test]
fn a_refresh_after_the_newer_writes_window_does_not_hand_the_post_to_an_older_write() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let older = state.begin_write(&post(1), false);
    let newer = state.begin_write(&post(1), true);
    state.complete_write(&post(1), newer, &post(1), now);
    state.complete_write(&post(1), older, &post(1), now + SECOND * 60);

    let later = now + RECENT_WRITE_TTL;
    let ticket = state.begin_refresh(later);
    state.apply_refresh(ticket, [post(1)], 1);

    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(later), Some(1));
}

#[test]
fn an_older_write_finishing_before_a_newer_one_fails_still_applies() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let save = state.begin_write(&post(1), true);
    let unsave = state.begin_write(&post(1), false);

    state.complete_write(&post(1), save, &post(1), now);
    state.fail_write(&post(1), unsave);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
}

#[test]
fn an_older_write_failing_late_does_not_undo_a_newer_one() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let save = state.begin_write(&post(1), true);
    let unsave = state.begin_write(&post(1), false);

    state.complete_write(&post(1), unsave, &post(1), now);
    state.fail_write(&post(1), save);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

#[test]
fn a_newer_write_failing_leaves_the_older_one_in_flight() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    state.begin_write(&post(1), true);
    let unsave = state.begin_write(&post(1), false);

    state.fail_write(&post(1), unsave);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
}

#[test]
fn finishing_a_write_twice_does_nothing() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&post(1), true);

    state.complete_write(&post(1), id, &post(1), now);
    state.complete_write(&post(1), id, &post(1), now);
    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn an_unknown_key_failing_does_nothing() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    let id = state.begin_write(&post(2), true);

    state.fail_write(&post(3), id);

    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
}

#[test]
fn writes_to_one_key_leave_others_alone() {
    let now = Instant::now();
    let mut state = loaded(&[post(1), post(2)], now);

    write(&mut state, &post(1), false, now);

    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
}

// The key the server stored

#[test]
fn a_save_is_recorded_under_the_key_the_server_stored() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&feed_item(2), true);

    state.complete_write(&feed_item(2), id, &post(1), now);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(now), Some(1));
}

/// Saving a feed item the server stores as a WordPress.com post leaves the post the user tapped
/// showing as saved, not flipping back when the response lands.
#[test]
fn a_post_saved_under_another_key_reads_as_saved_by_its_own_key() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&feed_item(2), true);
    state.complete_write(&feed_item(2), id, &post(1), now);

    assert_eq!(state.status(&[feed_item(2)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.keys_to_unsave(&[feed_item(2)], now), vec![post(1)]);

    // And after its window, once a refresh has the stored key.
    let later = now + RECENT_WRITE_TTL * 2;
    refresh(&mut state, &[post(1)], later);
    assert_eq!(
        state.status(&[feed_item(2)], later),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn unsaving_by_the_stored_key_unsaves_the_post_saved_under_another() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&feed_item(2), true);
    state.complete_write(&feed_item(2), id, &post(1), now);

    write(&mut state, &post(1), false, now);

    assert_eq!(
        state.status(&[feed_item(2)], now),
        ReaderSavedStatus::NotSaved
    );
    assert_eq!(state.saved_count(now), Some(0));
}

#[test]
fn a_url_saved_as_a_post_survives_a_stale_refresh() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let page = url("https://example.com/post");
    let id = state.begin_write(&page, true);
    state.complete_write(&page, id, &post(1), now);

    refresh(&mut state, &[], now + SECOND);

    assert_eq!(
        state.status(&[post(1)], now + SECOND),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn writes_moved_to_the_stored_key_stay_in_order() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let save = state.begin_write(&feed_item(2), true);
    let unsave = state.begin_write(&post(1), false);

    state.complete_write(&post(1), unsave, &post(1), now);
    state.complete_write(&feed_item(2), save, &post(1), now);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

// Stale reads

#[rstest]
#[case::straight_away(0)]
#[case::during_the_window(60)]
#[case::just_before_it_ends(119)]
fn a_stale_refresh_cannot_bring_back_an_unsaved_post(#[case] seconds: u32) {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);
    let later = now + SECOND * seconds;

    refresh(&mut state, &[post(1)], later);

    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
}

#[rstest]
#[case::straight_away(0)]
#[case::during_the_window(60)]
#[case::just_before_it_ends(119)]
fn a_stale_refresh_cannot_remove_a_saved_post(#[case] seconds: u32) {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    let later = now + SECOND * seconds;

    refresh(&mut state, &[], later);

    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
}

#[test]
fn a_refresh_in_flight_during_a_write_cannot_undo_it() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    let ticket = state.begin_refresh(now);
    write(&mut state, &post(1), false, now);

    state.apply_refresh(ticket, [post(1)], 1);

    assert_eq!(
        state.status(&[post(1)], now + SECOND),
        ReaderSavedStatus::NotSaved
    );
}

#[test]
fn a_refresh_during_a_slow_write_cannot_undo_it() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    state.begin_write(&post(1), false);

    refresh(&mut state, &[post(1)], now + RECENT_WRITE_TTL * 3);

    assert_eq!(
        state.status(&[post(1)], now + RECENT_WRITE_TTL * 3),
        ReaderSavedStatus::NotSaved
    );
}

#[test]
fn a_write_failing_after_a_refresh_shows_the_refreshed_state() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&post(1), true);
    refresh(&mut state, &[post(2)], now);

    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
}

#[test]
fn an_older_refresh_landing_late_is_ignored() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let older = state.begin_refresh(now);
    let newer = state.begin_refresh(now);

    state.apply_refresh(newer, [post(2)], 1);
    state.apply_refresh(older, [post(1)], 1);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
}

#[test]
fn refreshes_landing_in_order_all_apply() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let older = state.begin_refresh(now);
    let newer = state.begin_refresh(now);

    state.apply_refresh(older, [post(1)], 1);
    state.apply_refresh(newer, [post(2)], 1);

    assert_eq!(state.status(&[post(2)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
}

// Expiry

#[test]
fn the_server_wins_again_once_a_write_expires() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), false, now);
    let later = now + RECENT_WRITE_TTL;

    // Saved again from another device.
    refresh(&mut state, &[post(1)], later);

    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
}

#[test]
fn a_write_counts_until_the_moment_it_expires() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    let id = state.begin_write(&post(1), false);
    state.complete_write(&post(1), id, &post(1), now);

    let just_before = now + RECENT_WRITE_TTL - Duration::from_millis(1);
    refresh(&mut state, &[post(1)], just_before);
    assert_eq!(
        state.status(&[post(1)], just_before),
        ReaderSavedStatus::NotSaved
    );

    refresh(&mut state, &[post(1)], now + RECENT_WRITE_TTL);
    assert_eq!(
        state.status(&[post(1)], now + RECENT_WRITE_TTL),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn a_stale_refresh_does_not_come_back_when_the_write_expires() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);
    refresh(&mut state, &[post(1)], now + SECOND);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(later), Some(0));
}

#[test]
fn a_stale_refresh_does_not_lose_a_save_when_the_write_expires() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    refresh(&mut state, &[], now + SECOND);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(later), Some(1));
}

#[test]
fn a_refresh_only_keeps_the_newest_finished_write_for_a_key() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    write(&mut state, &post(1), false, now);
    refresh(&mut state, &[], now);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
}

#[test]
fn a_refresh_does_not_keep_a_write_still_in_flight() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&post(1), true);
    refresh(&mut state, &[], now);

    state.fail_write(&post(1), id);

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(now), Some(0));
}

#[test]
fn the_window_starts_when_the_write_finishes() {
    let now = Instant::now();
    let finished = now + SECOND * 90;
    let mut state = loaded(&[post(1)], now);
    let id = state.begin_write(&post(1), false);
    state.complete_write(&post(1), id, &post(1), finished);

    refresh(&mut state, &[post(1)], finished + SECOND * 60);

    assert_eq!(
        state.status(&[post(1)], finished + SECOND * 60),
        ReaderSavedStatus::NotSaved
    );
}

#[test]
fn an_in_flight_write_never_expires() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    state.begin_write(&post(1), true);

    assert_eq!(
        state.status(&[post(1)], now + RECENT_WRITE_TTL * 100),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn a_finished_write_stays_applied_after_it_expires_without_a_refresh() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(later), Some(1));
}

#[test]
fn a_finished_unsave_stays_applied_after_it_expires_without_a_refresh() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);

    let later = now + RECENT_WRITE_TTL * 2;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
    assert_eq!(state.saved_count(later), Some(0));
}

#[test]
fn a_refresh_clears_out_expired_writes() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);

    refresh(&mut state, &[], now + RECENT_WRITE_TTL);

    assert!(state.recent_writes.is_empty());
}

#[test]
fn a_refresh_keeps_writes_that_have_not_expired() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);

    refresh(&mut state, &[], now + SECOND);

    assert_eq!(state.recent_writes.len(), 1);
}

#[test]
fn an_older_write_expiring_falls_back_to_a_newer_live_one() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    write(&mut state, &post(1), false, now + SECOND * 100);

    let later = now + SECOND * 130;
    assert_eq!(state.status(&[post(1)], later), ReaderSavedStatus::NotSaved);
}

// The count

#[test]
fn the_count_includes_a_write_in_flight() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    state.begin_write(&post(2), true);

    assert_eq!(state.saved_count(now), Some(2));
}

#[test]
fn the_count_excludes_an_unsave_in_flight() {
    let now = Instant::now();
    let mut state = loaded(&[post(1), post(2)], now);

    state.begin_write(&post(2), false);

    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn the_count_includes_a_finished_write() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    write(&mut state, &post(2), true, now);

    assert_eq!(state.saved_count(now), Some(2));
}

#[test]
fn the_count_keeps_saves_beyond_the_key_limit_after_a_write() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);
    state.apply_refresh(ticket, [post(1)], 1003);

    write(&mut state, &post(1), false, now);

    assert_eq!(state.saved_count(now), Some(1002));
}

#[test]
fn the_count_stays_right_when_a_write_lands_on_a_stale_list() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);
    refresh(&mut state, &[post(1)], now + SECOND);

    write(&mut state, &post(2), true, now + SECOND);

    assert_eq!(state.saved_count(now + SECOND), Some(1));
}

#[test]
fn saving_a_saved_post_does_not_change_the_count() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    state.begin_write(&post(1), true);

    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn unsaving_an_unsaved_post_does_not_change_the_count() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    state.begin_write(&post(2), false);

    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn the_count_stays_right_through_a_stale_refresh() {
    let now = Instant::now();
    let mut state = loaded(&[post(1), post(2)], now);
    write(&mut state, &post(2), false, now);

    refresh(&mut state, &[post(1), post(2)], now + SECOND);

    assert_eq!(state.saved_count(now + SECOND), Some(1));
}

#[test]
fn the_count_cannot_go_below_zero() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);
    state.apply_refresh(ticket, [post(1), post(2)], 1);

    state.begin_write(&post(1), false);
    state.begin_write(&post(2), false);

    assert_eq!(state.saved_count(now), Some(0));
}

// The saved list

#[test]
fn a_listed_post_is_shown() {
    let now = Instant::now();
    let state = loaded(&[post(1)], now);
    assert!(state.should_show_listed_post(&[post(1)], now));
}

#[test]
fn a_listed_post_is_shown_before_the_first_refresh() {
    let state = ReaderSavedState::default();
    assert!(state.should_show_listed_post(&[post(1)], Instant::now()));
}

#[test]
fn a_listed_post_just_unsaved_is_hidden() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);

    assert!(!state.should_show_listed_post(&[post(1)], now + SECOND));
}

#[test]
fn a_listed_post_being_unsaved_is_hidden() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    state.begin_write(&post(1), false);

    assert!(!state.should_show_listed_post(&[post(1)], now));
}

#[test]
fn a_listed_post_unsaved_a_while_ago_is_shown_again() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);

    assert!(state.should_show_listed_post(&[post(1)], now + RECENT_WRITE_TTL));
}

#[test]
fn a_listed_post_stays_while_another_of_its_keys_is_saved() {
    let now = Instant::now();
    let mut state = loaded(&[post(1), feed_item(2)], now);
    write(&mut state, &post(1), false, now);

    assert!(state.should_show_listed_post(&[post(1), feed_item(2)], now));
}

#[test]
fn a_listed_post_unsaved_under_all_its_keys_is_hidden() {
    let now = Instant::now();
    let mut state = loaded(&[post(1), feed_item(2)], now);
    write(&mut state, &post(1), false, now);
    write(&mut state, &feed_item(2), false, now);

    assert!(!state.should_show_listed_post(&[post(1), feed_item(2)], now));
}

#[test]
fn a_post_reaching_the_list_before_the_keys_counts_as_saved() {
    let now = Instant::now();
    let mut state = loaded(&[], now);

    assert!(state.note_listed_post(&[post(1)], now));

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Saved);
    assert_eq!(state.saved_count(now), Some(1));
}

#[test]
fn noting_a_post_already_known_changes_nothing() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);

    assert!(!state.note_listed_post(&[post(1)], now));
    assert_eq!(state.saved_count(now), Some(1));
}

/// A Jetpack post saved under its feed item can be listed with its blog and post IDs.
#[test]
fn a_listed_post_saved_under_its_other_key_is_not_counted_twice() {
    let now = Instant::now();
    let mut state = loaded(&[feed_item(2)], now);

    assert!(!state.note_listed_post(&[post(1), feed_item(2)], now));

    assert_eq!(state.saved_count(now), Some(1));
    assert_eq!(
        state.keys_to_unsave(&[post(1), feed_item(2)], now),
        vec![feed_item(2)]
    );
}

#[test]
fn listing_past_the_key_limit_does_not_count_posts_twice() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();
    let ticket = state.begin_refresh(now);
    state.apply_refresh(ticket, [post(1)], 1005);

    assert!(!state.note_listed_post(&[post(2)], now));

    assert_eq!(state.saved_count(now), Some(1005));
}

#[test]
fn a_listed_post_before_the_first_refresh_stays_unknown() {
    let now = Instant::now();
    let mut state = ReaderSavedState::default();

    assert!(!state.note_listed_post(&[post(1)], now));

    assert_eq!(state.status(&[post(1)], now), ReaderSavedStatus::Unknown);
}

// Recent saves

#[test]
fn recent_saves_list_the_newest_first() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    write(&mut state, &post(2), true, now);
    state.begin_write(&post(3), true);

    assert_eq!(state.recent_saves(now), vec![post(3), post(2), post(1)]);
}

#[test]
fn recent_saves_skip_unsaves() {
    let now = Instant::now();
    let mut state = loaded(&[post(1)], now);
    write(&mut state, &post(1), false, now);

    assert!(state.recent_saves(now).is_empty());
}

#[test]
fn recent_saves_skip_a_post_saved_then_unsaved() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);
    write(&mut state, &post(1), false, now);

    assert!(state.recent_saves(now).is_empty());
}

#[test]
fn recent_saves_skip_a_failed_save() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let id = state.begin_write(&post(1), true);
    state.fail_write(&post(1), id);

    assert!(state.recent_saves(now).is_empty());
}

#[test]
fn recent_saves_end_when_the_write_expires() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    write(&mut state, &post(1), true, now);

    assert!(state.recent_saves(now + RECENT_WRITE_TTL).is_empty());
}

#[test]
fn recent_saves_keep_the_urls_spelling() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let page = url("https://Example.com/Page/?utm_source=x");
    let stored = url("https://example.com/Page");
    let id = state.begin_write(&page, true);

    assert_eq!(state.recent_saves(now), vec![page.clone()]);

    state.complete_write(&page, id, &stored, now);
    assert_eq!(state.recent_saves(now), vec![stored]);
}

#[test]
fn recent_saves_use_the_key_the_server_stored() {
    let now = Instant::now();
    let mut state = loaded(&[], now);
    let page = url("https://example.com/post");
    let id = state.begin_write(&page, true);
    state.complete_write(&page, id, &post(1), now);

    assert_eq!(state.recent_saves(now), vec![post(1)]);
}

// URLs

#[rstest]
#[case::trailing_slash("https://example.com/page/")]
#[case::fragment("https://example.com/page#top")]
#[case::tracking("https://example.com/page?utm_source=mail")]
#[case::case("HTTPS://EXAMPLE.COM/page")]
#[case::port("https://example.com:443/page")]
fn two_spellings_of_a_url_are_one_save(#[case] spelling: &str) {
    let now = Instant::now();
    let state = loaded(&[url("https://example.com/page")], now);

    assert_eq!(
        state.status(&[url(spelling)], now),
        ReaderSavedStatus::Saved
    );
}

#[test]
fn a_write_applies_to_every_spelling_of_a_url() {
    let now = Instant::now();
    let mut state = loaded(&[url("https://example.com/page")], now);

    write(
        &mut state,
        &url("https://example.com/page/?utm_source=x"),
        false,
        now,
    );

    assert_eq!(
        state.status(&[url("https://example.com/page")], now),
        ReaderSavedStatus::NotSaved
    );
}

// Randomised runs

/// A server whose reads replay its writes after a delay, like a lagging replica.
struct LaggyServer {
    saved: HashSet<ReaderSavedKey>,
    history: Vec<(Instant, ReaderSavedKey, bool)>,
    lag: Duration,
}

impl LaggyServer {
    fn write(&mut self, key: &ReaderSavedKey, saved: bool, now: Instant) {
        if saved {
            self.saved.insert(key.clone());
        } else {
            self.saved.remove(key);
        }
        self.history.push((now, key.clone(), saved));
    }

    /// What a replica returns: the current state with the last `lag` of writes undone.
    fn stale_read(&self, now: Instant) -> Vec<ReaderSavedKey> {
        let mut replica = HashSet::new();
        for (at, key, saved) in &self.history {
            if *at + self.lag <= now {
                if *saved {
                    replica.insert(key.clone());
                } else {
                    replica.remove(key);
                }
            }
        }
        replica.into_iter().collect()
    }
}

/// Whatever order writes and stale reads arrive in, this device always shows what it last did,
/// as long as replication catches up within the window.
#[test]
fn local_writes_always_win_over_stale_reads() {
    for seed in 0..300 {
        let mut rng = StdRng::seed_from_u64(seed);
        let start = Instant::now();
        let mut now = start;
        let keys: Vec<_> = (1..=6).map(post).collect();
        let lag = Duration::from_secs(rng.random_range(0..90));

        let mut server = LaggyServer {
            saved: HashSet::new(),
            history: Vec::new(),
            lag,
        };
        let mut state = loaded(&[], now);
        let mut expected: HashMap<ReaderSavedKey, bool> = HashMap::new();

        for _ in 0..200 {
            now += Duration::from_secs(rng.random_range(0..20));
            let key = keys[rng.random_range(0..keys.len())].clone();

            match rng.random_range(0..5) {
                4 => {
                    // Two writes to one key, whose responses arrive in the opposite order.
                    let first = !expected.get(&key).copied().unwrap_or(false);
                    let older = state.begin_write(&key, first);
                    let newer = state.begin_write(&key, !first);
                    server.write(&key, first, now);
                    server.write(&key, !first, now);
                    state.complete_write(&key, newer, &key, now);
                    let late = now + Duration::from_secs(rng.random_range(0..30));
                    state.complete_write(&key, older, &key, late);
                    expected.insert(key, !first);
                }
                0 => {
                    let saved = rng.random_bool(0.5);
                    let id = state.begin_write(&key, saved);
                    server.write(&key, saved, now);
                    state.complete_write(&key, id, &key, now);
                    expected.insert(key, saved);
                }
                1 => {
                    // A write the server refuses changes nothing.
                    let saved = !expected.get(&key).copied().unwrap_or(false);
                    let id = state.begin_write(&key, saved);
                    state.fail_write(&key, id);
                }
                _ => {
                    let read = server.stale_read(now);
                    let total = read.len() as u64;
                    let ticket = state.begin_refresh(now);
                    state.apply_refresh(ticket, read, total);
                }
            }

            for key in &keys {
                let saved = expected.get(key).copied().unwrap_or(false);
                let want = if saved {
                    ReaderSavedStatus::Saved
                } else {
                    ReaderSavedStatus::NotSaved
                };
                assert_eq!(
                    state.status(std::slice::from_ref(key), now),
                    want,
                    "seed {seed}, lag {lag:?}, {key:?}"
                );
            }
            assert_eq!(
                state.saved_count(now),
                Some(server.saved.len() as u64),
                "seed {seed}, lag {lag:?}"
            );
        }
    }
}

/// Once replication has caught up and writes have expired, the server's state is what's shown.
#[test]
fn the_state_settles_on_the_server_after_everything_expires() {
    for seed in 0..100 {
        let mut rng = StdRng::seed_from_u64(seed);
        let now = Instant::now();
        let keys: Vec<_> = (1..=6).map(post).collect();
        let mut state = loaded(&[], now);

        for _ in 0..50 {
            let key = &keys[rng.random_range(0..keys.len())];
            let id = state.begin_write(key, rng.random_bool(0.5));
            if rng.random_bool(0.8) {
                state.complete_write(key, id, key, now);
            } else {
                state.fail_write(key, id);
            }
        }

        let server = [post(2), post(5)];
        let later = now + RECENT_WRITE_TTL;
        refresh(&mut state, &server, later);

        assert!(state.recent_writes.is_empty(), "seed {seed}");
        for key in &keys {
            let want = if server.contains(key) {
                ReaderSavedStatus::Saved
            } else {
                ReaderSavedStatus::NotSaved
            };
            assert_eq!(
                state.status(std::slice::from_ref(key), later),
                want,
                "seed {seed}"
            );
        }
        assert_eq!(state.saved_count(later), Some(2));
    }
}
