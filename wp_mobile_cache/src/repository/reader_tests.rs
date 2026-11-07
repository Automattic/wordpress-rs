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
    assert_eq!(stream.version, version);
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

#[test]
fn test_saved_state_round_trips() {
    let conn = conn();
    let account = account(&conn, 1);
    assert_eq!(
        ReaderSavedRepository::select_state(&conn, account).unwrap(),
        DbReaderSavedState::default()
    );

    ReaderSavedRepository::write_local(&conn, account, "local").unwrap();
    ReaderSavedRepository::write_snapshot(&conn, account, "snapshot").unwrap();
    ReaderSavedRepository::set_imported_local_saves(&conn, account).unwrap();
    ReaderSavedRepository::write_local(&conn, account, "local 2").unwrap();

    assert_eq!(
        ReaderSavedRepository::select_state(&conn, account).unwrap(),
        DbReaderSavedState {
            snapshot: Some("snapshot".to_string()),
            local: Some("local 2".to_string()),
            imported_local_saves: true,
        }
    );
}

#[test]
fn test_local_only_saves() {
    let conn = conn();
    let account = account(&conn, 1);
    let mut stored = post(1);
    stored.title = Some("Kept".to_string());
    let row = ReaderPostRepository::upsert(&conn, account, &stored).unwrap();
    ReaderSavedRepository::upsert_local_only(
        &conn,
        account,
        &post_key(1),
        Some(row),
        Some("2026-01-01T00:00:00.000Z"),
    )
    .unwrap();
    ReaderSavedRepository::upsert_local_only(
        &conn,
        account,
        &post_key(2),
        None,
        Some("2026-01-02T00:00:00.000Z"),
    )
    .unwrap();

    let saves = ReaderSavedRepository::select_local_only(&conn, account).unwrap();
    assert_eq!(saves.len(), 2);
    assert_eq!(saves[0].key, post_key(2));
    assert_eq!(saves[0].post, None);
    assert_eq!(saves[1].post, Some(stored));

    assert!(ReaderSavedRepository::delete_local_only(&conn, account, &post_key(2)).unwrap());
    assert!(!ReaderSavedRepository::delete_local_only(&conn, account, &post_key(2)).unwrap());
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
        ReaderSavedRepository::write_local(&conn, account, "{}").unwrap();
        ReaderSavedRepository::upsert_local_only(&conn, account, &post_key(1), Some(row), None)
            .unwrap();
    }

    assert!(WordPressComAccountRepository::delete(&conn, WpComUserId(1)).unwrap());

    for table in [
        "reader_posts",
        "reader_streams",
        "reader_stream_items",
        "reader_saved_state",
        "reader_saved_local_only",
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
