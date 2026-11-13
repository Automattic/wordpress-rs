use super::*;
use crate::{
    MigrationManager, repository::wordpress_com_accounts::WordPressComAccountRepository,
    test_fixtures::get_table_column_names,
};
use rusqlite::Connection;
use wp_api::posts::PostId;
use wp_api::wp_com::me::WpComUserId;

fn conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    MigrationManager::new(&conn)
        .unwrap()
        .perform_migrations()
        .unwrap();
    conn
}

fn account(conn: &Connection, user_id: u64) -> RowId {
    WordPressComAccountRepository::get_or_create(conn, WpComUserId(user_id)).unwrap()
}

fn identity(post_id: u64) -> ReaderPostSaveIdentity {
    ReaderPostSaveIdentity {
        id: post_id,
        site_id: Some(WpComSiteId(123)),
        feed_id: None,
        feed_item_id: None,
        is_external: false,
        url: None,
    }
}

fn post(post_id: u64) -> ReaderPostRecord {
    ReaderPostRecord::from_identity(identity(post_id)).unwrap()
}

fn post_key(post_id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(post_id),
    }
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

#[test]
fn test_reader_posts_columns() {
    // Phase 2 fills the rest of these from the full Reader post, without a migration.
    let columns = get_table_column_names(&conn(), "reader_posts");
    let read: Vec<&str> = POST_COLUMNS
        .split(',')
        .map(|column| column.trim().trim_start_matches("p."))
        .collect();
    assert_eq!(read.len(), POST_COLUMN_COUNT);
    for column in read {
        assert!(
            columns.iter().any(|other| other == column),
            "{column} is read but isn't in reader_posts"
        );
    }
    assert_eq!(columns.len(), 57, "{columns:?}");
}

#[test]
fn test_upsert_finds_the_post_by_its_key() {
    let conn = conn();
    let account = account(&conn, 1);
    let mut stored = post(456);
    stored.title = Some("Title".to_string());
    let row = ReaderPostRepository::upsert(&conn, account, &stored).unwrap();

    let (found_row, found) = ReaderPostRepository::select_by_key(&conn, account, &post_key(456))
        .unwrap()
        .unwrap();
    assert_eq!(found_row, row);
    assert_eq!(found, stored);
}

#[test]
fn test_upsert_keeps_fields_a_slimmer_source_leaves_out() {
    let conn = conn();
    let account = account(&conn, 1);
    let mut full = post(456);
    full.title = Some("Title".to_string());
    full.like_count = Some(3);
    full.date = Some("2026-01-02T03:04:05Z".parse().unwrap());
    let first = ReaderPostRepository::upsert(&conn, account, &full).unwrap();

    let mut slim = post(456);
    slim.like_count = Some(4);
    let second = ReaderPostRepository::upsert(&conn, account, &slim).unwrap();

    assert_eq!(first, second);
    let (_, found) = ReaderPostRepository::select_by_key(&conn, account, &post_key(456))
        .unwrap()
        .unwrap();
    assert_eq!(found.title.as_deref(), Some("Title"));
    assert_eq!(found.date, full.date);
    assert_eq!(
        found.like_count,
        Some(4),
        "New server state replaces the old"
    );
    assert_eq!(count(&conn, "reader_posts"), 1);
}

#[test]
fn test_two_spellings_of_a_url_are_one_post() {
    let conn = conn();
    let account = account(&conn, 1);
    let url_identity = |url: &str| ReaderPostSaveIdentity {
        id: 9,
        site_id: Some(WpComSiteId(0)),
        feed_id: Some(ReaderFeedId(0)),
        feed_item_id: None,
        is_external: true,
        url: Some(url.to_string()),
    };
    let first = ReaderPostRecord::from_identity(url_identity("https://example.com/a/")).unwrap();
    let second =
        ReaderPostRecord::from_identity(url_identity("https://example.com/a?utm_source=x"))
            .unwrap();
    assert_eq!(
        ReaderPostRepository::upsert(&conn, account, &first).unwrap(),
        ReaderPostRepository::upsert(&conn, account, &second).unwrap()
    );
}

#[test]
fn test_a_feed_item_key_finds_the_post_that_has_it() {
    let conn = conn();
    let account = account(&conn, 1);
    let jetpack = ReaderPostRecord::from_identity(ReaderPostSaveIdentity {
        feed_id: Some(ReaderFeedId(5)),
        feed_item_id: Some(ReaderFeedItemId(77)),
        ..identity(456)
    })
    .unwrap();
    let row = ReaderPostRepository::upsert(&conn, account, &jetpack).unwrap();

    let key = ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(77),
    };
    let (found, _) = ReaderPostRepository::select_by_key(&conn, account, &key)
        .unwrap()
        .unwrap();
    assert_eq!(found, row);
}

#[test]
fn test_accounts_dont_see_each_others_posts() {
    let conn = conn();
    let first = account(&conn, 1);
    let second = account(&conn, 2);
    ReaderPostRepository::upsert(&conn, first, &post(456)).unwrap();

    assert!(
        ReaderPostRepository::select_by_key(&conn, second, &post_key(456))
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_stream_items_are_newest_first() {
    let conn = conn();
    let account = account(&conn, 1);
    let stream = ReaderStreamRepository::get_or_create(&conn, account, "saved").unwrap();
    for (post_id, sort_key) in [(1, "2026-01-02"), (2, "2026-01-03"), (3, "2026-01-01")] {
        let row = ReaderPostRepository::upsert(&conn, account, &post(post_id)).unwrap();
        ReaderStreamRepository::upsert_item(&conn, stream.row_id, row, sort_key).unwrap();
    }
    // Moving a post doesn't add it twice.
    let row = ReaderPostRepository::upsert(&conn, account, &post(3)).unwrap();
    ReaderStreamRepository::upsert_item(&conn, stream.row_id, row, "2026-01-04").unwrap();

    let ids: Vec<u64> = ReaderStreamRepository::select_items(&conn, stream.row_id)
        .unwrap()
        .into_iter()
        .map(|item| item.post.identity.id)
        .collect();
    assert_eq!(ids, vec![3, 2, 1]);

    let items = ReaderStreamRepository::select_items(&conn, stream.row_id).unwrap();
    let identities = ReaderStreamRepository::select_item_identities(&conn, stream.row_id).unwrap();
    let expected: Vec<_> = items
        .into_iter()
        .map(|item| (item.post_row_id, item.post.identity))
        .collect();
    assert_eq!(identities, expected);
}

#[test]
fn test_fill_missing_keeps_what_is_stored() {
    let conn = conn();
    let account = account(&conn, 1);
    let mut from_server = post(456);
    from_server.title = Some("Server title".to_string());
    let row = ReaderPostRepository::upsert(&conn, account, &from_server).unwrap();

    let mut from_app = post(456);
    from_app.title = Some("Older app title".to_string());
    from_app.excerpt = Some("An excerpt".to_string());
    assert_eq!(
        ReaderPostRepository::fill_missing(&conn, account, &from_app).unwrap(),
        row
    );

    let (_, found) = ReaderPostRepository::select_by_key(&conn, account, &post_key(456))
        .unwrap()
        .unwrap();
    assert_eq!(found.title.as_deref(), Some("Server title"));
    assert_eq!(found.excerpt.as_deref(), Some("An excerpt"));
}

#[test]
fn test_stream_cursor_and_state() {
    let conn = conn();
    let account = account(&conn, 1);
    let stream = ReaderStreamRepository::get_or_create(&conn, account, "saved").unwrap();
    assert_eq!(stream.has_more, None);
    assert_eq!(stream.state, ListState::Idle);

    let version = ReaderStreamRepository::begin_refresh(&conn, stream.row_id).unwrap();
    assert_eq!(version, stream.version + 1);
    ReaderStreamRepository::set_cursor(&conn, stream.row_id, Some("next"), true).unwrap();

    let stream = ReaderStreamRepository::select(&conn, account, "saved")
        .unwrap()
        .unwrap();
    assert_eq!(stream.next_page_handle.as_deref(), Some("next"));
    assert_eq!(stream.has_more, Some(true));
    assert!(stream.last_refreshed_at.is_some());
    assert_eq!(stream.state, ListState::Idle);
    assert_eq!(
        stream.version,
        version + 1,
        "Storing the first page makes a load-more that started during the refresh stale"
    );

    // A later page leaves the version alone.
    ReaderStreamRepository::set_cursor(&conn, stream.row_id, None, false).unwrap();
    let stream = ReaderStreamRepository::select(&conn, account, "saved")
        .unwrap()
        .unwrap();
    assert_eq!(stream.version, version + 1);
    assert_eq!(stream.has_more, Some(false));
}

#[test]
fn test_reset_stale_fetching_states() {
    let conn = conn();
    let account = account(&conn, 1);
    let stream = ReaderStreamRepository::get_or_create(&conn, account, "saved").unwrap();
    ReaderStreamRepository::set_state(&conn, stream.row_id, ListState::FetchingNextPage, None)
        .unwrap();

    ReaderStreamRepository::reset_stale_fetching_states(&conn).unwrap();

    let stream = ReaderStreamRepository::select(&conn, account, "saved")
        .unwrap()
        .unwrap();
    assert_eq!(stream.state, ListState::Idle);
}

// Saved posts

fn feed_key(feed_item_id: u64) -> ReaderSavedKey {
    ReaderSavedKey::FeedItem {
        feed_item_id: ReaderFeedItemId(feed_item_id),
    }
}

fn url_key(url: &str) -> ReaderSavedKey {
    ReaderSavedKey::Url {
        url: url.to_string(),
    }
}

fn date(text: &str) -> WpGmtDateTime {
    text.parse().unwrap()
}

fn entry(key: ReaderSavedKey, date_saved: &str) -> DbReaderSavedKey {
    DbReaderSavedKey {
        key,
        date_saved: date(date_saved),
        url: None,
    }
}

fn row_for(conn: &Connection, account: RowId, key: &ReaderSavedKey) -> Option<DbReaderSavedRow> {
    ReaderSavedRepository::select_rows_for_keys(conn, account, std::slice::from_ref(key))
        .unwrap()
        .into_iter()
        .next()
}

fn rows(conn: &Connection, account: RowId) -> Vec<(String, bool, Option<bool>)> {
    let mut statement = conn
        .prepare(
            "SELECT key, saved, server FROM reader_saved_posts WHERE account_id = ?1 ORDER BY key",
        )
        .unwrap();
    statement
        .query_map([account], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn now() -> WpGmtDateTime {
    date("2026-11-12T00:00:00Z")
}

#[test]
fn test_saved_state_is_empty_before_anything() {
    let conn = conn();
    let account = account(&conn, 1);
    assert_eq!(
        ReaderSavedRepository::select_state(&conn, account).unwrap(),
        DbReaderSavedState::default()
    );
    assert_eq!(
        ReaderSavedRepository::saved_count(&conn, account).unwrap(),
        None
    );
}

#[test]
fn test_a_tapped_save_is_pending_and_listed_until_answered() {
    let conn = conn();
    let account = account(&conn, 1);
    let row =
        ReaderSavedRepository::tap_save(&conn, account, &post_key(1), &[post_key(1)], None, &now())
            .unwrap();
    let pending = ReaderSavedRepository::select_pending(&conn, account).unwrap();
    assert_eq!(pending.len(), 1);
    assert!(pending[0].saved && pending[0].listed && pending[0].server.is_none());
    assert_eq!(pending[0].date_saved, Some(now()));

    ReaderSavedRepository::apply_answer(&conn, account, row, true, &post_key(1), Some(1)).unwrap();
    assert!(
        ReaderSavedRepository::select_pending(&conn, account)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        ReaderSavedRepository::saved_count(&conn, account).unwrap(),
        Some(1)
    );
}

#[test]
fn test_a_row_in_doubt_is_not_pending() {
    let conn = conn();
    let account = account(&conn, 1);
    let row =
        ReaderSavedRepository::tap_save(&conn, account, &post_key(1), &[post_key(1)], None, &now())
            .unwrap();
    ReaderSavedRepository::set_in_doubt(&conn, row, true).unwrap();
    assert!(
        ReaderSavedRepository::select_pending(&conn, account)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_an_answer_under_another_key_moves_the_row_and_keeps_the_key_sent_as_an_alias() {
    let conn = conn();
    let account = account(&conn, 1);
    let url = url_key("https://example.com/p9");
    let row = ReaderSavedRepository::tap_save(
        &conn,
        account,
        &url,
        std::slice::from_ref(&url),
        None,
        &now(),
    )
    .unwrap();
    let kept =
        ReaderSavedRepository::apply_answer(&conn, account, row, true, &post_key(9), None).unwrap();
    assert_eq!(kept, row);
    let stored = row_for(&conn, account, &url).unwrap();
    assert_eq!(stored.key, post_key(9));
    assert_eq!(stored.url_key, Some(url));
    assert_eq!(stored.server, Some(true));
}

#[test]
fn test_an_answer_merges_into_the_stored_keys_row_and_the_newer_tap_decides() {
    let conn = conn();
    let account = account(&conn, 1);
    // F4 is unsaved first, then post 3 saved: the save is the newer tap.
    let unsave = ReaderSavedRepository::tap_unsave(&conn, account, &[feed_key(4)], true).unwrap();
    ReaderSavedRepository::tap_save(&conn, account, &post_key(3), &[post_key(3)], None, &now())
        .unwrap();
    let kept =
        ReaderSavedRepository::apply_answer(&conn, account, unsave[0], false, &post_key(3), None)
            .unwrap();
    let row = ReaderSavedRepository::select_row(&conn, kept)
        .unwrap()
        .unwrap();
    assert_eq!(row.key, post_key(3));
    assert_eq!(row.feed_item_key, Some(feed_key(4)));
    assert!(row.saved, "the newer tap");
    assert_eq!(row.server, Some(false), "the answer");
    assert_eq!(rows(&conn, account).len(), 1);
}

#[test]
fn test_a_refused_tap_is_undone_and_a_refused_import_is_kept_on_this_device() {
    let conn = conn();
    let account = account(&conn, 1);
    let tapped =
        ReaderSavedRepository::tap_save(&conn, account, &post_key(1), &[post_key(1)], None, &now())
            .unwrap();
    ReaderSavedRepository::apply_refusal(&conn, tapped).unwrap();
    assert!(
        row_for(&conn, account, &post_key(1)).is_none(),
        "nothing was known about it"
    );

    assert!(
        ReaderSavedRepository::insert_imported(&conn, account, &post_key(2), None, None).unwrap()
    );
    let imported = row_for(&conn, account, &post_key(2)).unwrap();
    ReaderSavedRepository::apply_refusal(&conn, imported.row_id).unwrap();
    let imported = row_for(&conn, account, &post_key(2)).unwrap();
    assert!(imported.local_only && imported.saved && !imported.is_pending());
}

#[test]
fn test_a_refused_unsave_goes_back_to_what_the_server_said() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[entry(post_key(1), "2026-11-01T00:00:00Z")],
        1,
        1,
    )
    .unwrap();
    let rows_ = ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(1)], false).unwrap();
    ReaderSavedRepository::apply_refusal(&conn, rows_[0]).unwrap();
    assert!(row_for(&conn, account, &post_key(1)).unwrap().saved);
}

#[test]
fn test_a_save_calls_off_a_pending_unsave_of_another_of_the_posts_rows() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[
            entry(post_key(3), "2026-11-02T00:00:00Z"),
            entry(feed_key(4), "2026-11-01T00:00:00Z"),
        ],
        2,
        2,
    )
    .unwrap();
    let keys = [post_key(3), feed_key(4)];
    ReaderSavedRepository::tap_unsave(&conn, account, &keys, false).unwrap();
    // A row this device only knows from its own unsave is removed.
    ReaderSavedRepository::tap_unsave(&conn, account, &[feed_key(5)], true).unwrap();
    ReaderSavedRepository::tap_save(&conn, account, &post_key(3), &keys, None, &now()).unwrap();
    ReaderSavedRepository::tap_save(
        &conn,
        account,
        &post_key(6),
        &[post_key(6), feed_key(5)],
        None,
        &now(),
    )
    .unwrap();

    assert_eq!(
        rows(&conn, account),
        [
            ("feed-item:4".to_string(), true, Some(true)),
            ("post:123:3".to_string(), true, Some(true)),
            ("post:123:6".to_string(), true, None),
        ]
    );
}

#[test]
fn test_an_unsave_adds_rows_for_keys_without_one_only_when_asked() {
    let conn = conn();
    let account = account(&conn, 1);
    let keys = [post_key(3), feed_key(4)];
    assert!(
        ReaderSavedRepository::tap_unsave(&conn, account, &keys, false)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        ReaderSavedRepository::tap_unsave(&conn, account, &keys, true)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        ReaderSavedRepository::select_pending(&conn, account)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn test_unsaving_a_local_only_save_removes_it() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::insert_imported(&conn, account, &post_key(1), None, None).unwrap();
    let row = row_for(&conn, account, &post_key(1)).unwrap();
    ReaderSavedRepository::apply_refusal(&conn, row.row_id).unwrap();
    ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(1)], true).unwrap();
    assert!(rows(&conn, account).is_empty());
}

#[test]
fn test_a_complete_refresh_says_what_the_server_has_and_keeps_pending_rows() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[
            entry(post_key(1), "2026-11-02T00:00:00Z"),
            entry(post_key(2), "2026-11-01T00:00:00Z"),
        ],
        2,
        2,
    )
    .unwrap();
    // A save not sent yet, an unsave not sent yet, and a save in doubt.
    ReaderSavedRepository::tap_save(&conn, account, &post_key(3), &[post_key(3)], None, &now())
        .unwrap();
    ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(2)], false).unwrap();
    let doubt =
        ReaderSavedRepository::tap_save(&conn, account, &post_key(4), &[post_key(4)], None, &now())
            .unwrap();
    ReaderSavedRepository::set_in_doubt(&conn, doubt, true).unwrap();

    // Post 1 was unsaved elsewhere, and post 4 reached the server.
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[
            entry(post_key(4), "2026-11-12T00:00:00Z"),
            entry(post_key(2), "2026-11-01T00:00:00Z"),
        ],
        2,
        2,
    )
    .unwrap();

    assert_eq!(
        rows(&conn, account),
        [
            ("post:123:2".to_string(), false, Some(true)),
            ("post:123:3".to_string(), true, Some(false)),
            ("post:123:4".to_string(), true, Some(true)),
        ]
    );
    let pending: Vec<_> = ReaderSavedRepository::select_pending(&conn, account)
        .unwrap()
        .into_iter()
        .map(|row| row.key)
        .collect();
    assert_eq!(pending, [post_key(3), post_key(2)]);
    let state = ReaderSavedRepository::select_state(&conn, account).unwrap();
    assert!(state.keys_refreshed && state.keys_complete);
    assert_eq!(state.total, Some(2));
    // 2, plus post 3 to save, less post 2 to unsave.
    assert_eq!(
        ReaderSavedRepository::saved_count(&conn, account).unwrap(),
        Some(2)
    );
}

/// An unsave timed out, then the user saved again: the row reads as agreeing with the server, but
/// the late delete may have landed. And the other way round. Either way the last tap stands.
#[test]
fn test_a_refresh_keeps_the_last_tap_of_a_row_in_doubt() {
    let conn = conn();
    let account = account(&conn, 1);
    let one = post_key(1);
    let two = post_key(2);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[entry(one.clone(), "2026-11-02T00:00:00Z")],
        1,
        1,
    )
    .unwrap();
    let saved_again = row_for(&conn, account, &one).unwrap().row_id;
    ReaderSavedRepository::tap_unsave(&conn, account, std::slice::from_ref(&one), false).unwrap();
    ReaderSavedRepository::set_in_doubt(&conn, saved_again, true).unwrap();
    ReaderSavedRepository::tap_save(
        &conn,
        account,
        &one,
        std::slice::from_ref(&one),
        None,
        &now(),
    )
    .unwrap();
    // A save that timed out, then an unsave.
    let unsaved_again = ReaderSavedRepository::tap_save(
        &conn,
        account,
        &two,
        std::slice::from_ref(&two),
        None,
        &now(),
    )
    .unwrap();
    ReaderSavedRepository::apply_answer(&conn, account, unsaved_again, false, &two, None).unwrap();
    ReaderSavedRepository::tap_save(
        &conn,
        account,
        &two,
        std::slice::from_ref(&two),
        None,
        &now(),
    )
    .unwrap();
    ReaderSavedRepository::set_in_doubt(&conn, unsaved_again, true).unwrap();
    ReaderSavedRepository::tap_unsave(&conn, account, std::slice::from_ref(&two), false).unwrap();

    // Both late requests landed: the server has post 2 and not post 1.
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[entry(two.clone(), "2026-11-12T00:00:00Z")],
        1,
        1,
    )
    .unwrap();

    assert_eq!(
        rows(&conn, account),
        vec![
            ("post:123:1".to_string(), true, Some(false)),
            ("post:123:2".to_string(), false, Some(true)),
        ],
        "both are pending, as the user last tapped them"
    );
}

#[test]
fn test_past_the_key_limit_a_row_older_than_the_oldest_key_returned_is_left_alone() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[
            entry(post_key(2), "2026-11-02T00:00:00Z"),
            entry(post_key(1), "2026-11-01T00:00:00Z"),
            entry(post_key(3), "2026-01-01T00:00:00Z"),
        ],
        3,
        3,
    )
    .unwrap();

    // Now 4 saves, and only 2 keys returned: posts 2 and 3 aren't, but only post 2 is newer than
    // the oldest key returned.
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[
            entry(post_key(5), "2026-11-04T00:00:00Z"),
            entry(post_key(1), "2026-11-01T00:00:00Z"),
        ],
        2,
        4,
    )
    .unwrap();

    assert_eq!(
        rows(&conn, account),
        [
            ("post:123:1".to_string(), true, Some(true)),
            ("post:123:3".to_string(), true, Some(true)),
            ("post:123:5".to_string(), true, Some(true)),
        ]
    );
    assert!(
        !ReaderSavedRepository::select_state(&conn, account)
            .unwrap()
            .keys_complete
    );
}

#[test]
fn test_a_refresh_merges_a_row_saved_by_url_into_its_post() {
    let conn = conn();
    let account = account(&conn, 1);
    let url = url_key("https://example.com/p9");
    let row = ReaderSavedRepository::tap_save(
        &conn,
        account,
        &url,
        std::slice::from_ref(&url),
        None,
        &now(),
    )
    .unwrap();
    ReaderSavedRepository::set_in_doubt(&conn, row, true).unwrap();
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[DbReaderSavedKey {
            key: post_key(9),
            date_saved: now(),
            url: Some("https://example.com/p9/".to_string()),
        }],
        1,
        1,
    )
    .unwrap();

    let stored = row_for(&conn, account, &url).unwrap();
    assert_eq!(stored.key, post_key(9));
    assert!(stored.saved && stored.server == Some(true) && !stored.in_doubt && stored.listed);
    assert_eq!(rows(&conn, account).len(), 1);
}

#[test]
fn test_the_list_adds_a_row_but_doesnt_change_one_that_is_there() {
    let conn = conn();
    let account = account(&conn, 1);
    let one = ReaderPostRepository::upsert(&conn, account, &post(1)).unwrap();
    let two = ReaderPostRepository::upsert(&conn, account, &post(2)).unwrap();
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[entry(post_key(2), "2026-11-01T00:00:00Z")],
        1,
        1,
    )
    .unwrap();
    ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(2)], false).unwrap();

    let listed = date("2026-11-05T00:00:00Z");
    ReaderSavedRepository::apply_listed(
        &conn,
        account,
        &post_key(1),
        &[post_key(1)],
        one,
        Some(&listed),
    )
    .unwrap();
    ReaderSavedRepository::apply_listed(&conn, account, &post_key(2), &[post_key(2)], two, None)
        .unwrap();

    assert_eq!(
        rows(&conn, account),
        [
            ("post:123:1".to_string(), true, Some(true)),
            ("post:123:2".to_string(), false, Some(true)),
        ]
    );
    let list = ReaderSavedRepository::select_list(&conn, account).unwrap();
    assert_eq!(list.len(), 1, "post 2 is being unsaved");
    assert_eq!(list[0].row.key, post_key(1));
    assert_eq!(list[0].post.as_ref().unwrap().identity, identity(1));
}

#[test]
fn test_the_list_is_newest_first_and_only_has_listed_or_local_only_saves() {
    let conn = conn();
    let account = account(&conn, 1);
    ReaderSavedRepository::apply_keys(
        &conn,
        account,
        &[entry(post_key(1), "2026-11-01T00:00:00Z")],
        1,
        1,
    )
    .unwrap();
    ReaderSavedRepository::tap_save(&conn, account, &post_key(2), &[post_key(2)], None, &now())
        .unwrap();
    ReaderSavedRepository::insert_imported(
        &conn,
        account,
        &post_key(3),
        None,
        Some(&date("2026-10-01T00:00:00Z")),
    )
    .unwrap();
    let keys: Vec<_> = ReaderSavedRepository::select_list(&conn, account)
        .unwrap()
        .into_iter()
        .map(|item| item.row.key)
        .collect();
    assert_eq!(keys, [post_key(2), post_key(3)], "post 1 isn't listed yet");
}

#[test]
fn test_deleting_an_account_deletes_its_reader_data() {
    let conn = conn();
    let removed = account(&conn, 1);
    let kept = account(&conn, 2);
    for account in [removed, kept] {
        let row = ReaderPostRepository::upsert(&conn, account, &post(1)).unwrap();
        let stream = ReaderStreamRepository::get_or_create(&conn, account, "saved").unwrap();
        ReaderStreamRepository::upsert_item(&conn, stream.row_id, row, "a").unwrap();
        ReaderSavedRepository::apply_keys(
            &conn,
            account,
            &[entry(post_key(1), "2026-11-01T00:00:00Z")],
            1,
            1,
        )
        .unwrap();
    }

    assert!(WordPressComAccountRepository::delete(&conn, WpComUserId(1)).unwrap());

    for table in [
        "reader_posts",
        "reader_streams",
        "reader_stream_items",
        "reader_saved_state",
        "reader_saved_posts",
    ] {
        assert_eq!(
            count(&conn, table),
            1,
            "{table} should only have the other account's row"
        );
    }
    assert!(
        ReaderPostRepository::select_by_key(&conn, kept, &post_key(1))
            .unwrap()
            .is_some()
    );
}
