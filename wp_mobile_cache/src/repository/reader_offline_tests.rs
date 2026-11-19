use super::*;
use crate::{
    MigrationManager,
    db_types::reader::ReaderPostRecord,
    repository::{
        reader::{ReaderPostRepository, ReaderSavedRepository},
        wordpress_com_accounts::WordPressComAccountRepository,
    },
};
use rusqlite::Connection;
use std::str::FromStr;
use wp_api::posts::PostId;
use wp_api::wp_com::me::WpComUserId;
use wp_api::wp_com::reader_posts::ReaderStreamOrder;
use wp_api::wp_com::reader_saved::{ReaderPostSaveIdentity, ReaderSavedOrderBy};

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

fn date(text: &str) -> WpGmtDateTime {
    WpGmtDateTime::from_str(text).unwrap()
}

fn post_key(post_id: i64) -> ReaderSavedKey {
    ReaderSavedKey::Post {
        blog_id: WpComSiteId(123),
        post_id: PostId(post_id),
    }
}

fn post(post_id: u64, content: Option<&str>) -> ReaderPostRecord {
    ReaderPostRecord {
        content: content.map(str::to_string),
        ..ReaderPostRecord::from_identity(ReaderPostSaveIdentity {
            id: post_id,
            site_id: Some(WpComSiteId(123)),
            feed_id: None,
            feed_item_id: None,
            is_external: false,
            url: None,
        })
        .unwrap()
    }
}

/// Saves post `id` at `saved_at`, with `content` when it has a post in the cache. Returns the
/// save's row and the post's.
fn save(
    conn: &Connection,
    account: RowId,
    id: u64,
    saved_at: &str,
    content: Option<Option<&str>>,
) -> (RowId, Option<RowId>) {
    let post_row = content
        .map(|content| ReaderPostRepository::upsert(conn, account, &post(id, content)).unwrap());
    let key = post_key(id as i64);
    let row = ReaderSavedRepository::tap_save(
        conn,
        account,
        &key,
        std::slice::from_ref(&key),
        post_row,
        &date(saved_at),
    )
    .unwrap();
    (row, post_row)
}

fn keys_without_content(conn: &Connection, account: RowId) -> Vec<ReaderSavedKey> {
    ReaderOfflineRepository::select_without_content(conn, account)
        .unwrap()
        .into_iter()
        .map(|save| save.key)
        .collect()
}

#[test]
fn test_a_save_has_no_offline_row_until_something_is_known() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", None);

    assert_eq!(
        ReaderOfflineRepository::select(&conn, row).unwrap(),
        DbReaderSavedOffline::default()
    );
}

#[test]
fn test_saves_without_content_are_the_ones_with_no_post_or_no_content() {
    let conn = conn();
    let account = account(&conn, 1);
    save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    save(&conn, account, 2, "2026-11-02T00:00:00Z", Some(None));
    save(
        &conn,
        account,
        3,
        "2026-11-03T00:00:00Z",
        Some(Some("<p>Hi</p>")),
    );

    assert_eq!(
        keys_without_content(&conn, account),
        [post_key(2), post_key(1)],
        "newest save first"
    );
    assert_eq!(
        ReaderOfflineRepository::count_without_content(&conn, account).unwrap(),
        2
    );
    assert_eq!(
        ReaderOfflineRepository::count_ready(&conn, account).unwrap(),
        (1, 0)
    );
}

/// A post stored in full that has no content has all there is: it isn't fetched again.
#[test]
fn test_a_fetched_post_without_content_has_its_content() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", Some(None));

    ReaderOfflineRepository::set_fetched(&conn, row).unwrap();

    assert_eq!(keys_without_content(&conn, account), []);
    assert!(ReaderOfflineRepository::select(&conn, row).unwrap().fetched);
}

#[test]
fn test_an_unsaved_post_is_not_waiting_for_content() {
    let conn = conn();
    let account = account(&conn, 1);
    save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(1)]).unwrap();

    assert_eq!(keys_without_content(&conn, account), []);
}

#[test]
fn test_a_failure_is_counted_and_a_fetch_clears_it() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    ReaderOfflineRepository::set_downloading(&conn, row, true).unwrap();

    let now = date("2026-11-05T10:00:00Z");
    ReaderOfflineRepository::record_failure(&conn, row, "Not found", &now).unwrap();
    ReaderOfflineRepository::record_failure(&conn, row, "Not found again", &now).unwrap();

    let failed = ReaderOfflineRepository::select(&conn, row).unwrap();
    assert_eq!(failed.error.as_deref(), Some("Not found again"));
    assert_eq!(failed.attempts, 2);
    assert_eq!(failed.attempted_at, Some(now));
    assert!(!failed.downloading, "a failure ends the download");

    ReaderOfflineRepository::set_fetched(&conn, row).unwrap();
    assert_eq!(
        ReaderOfflineRepository::select(&conn, row).unwrap(),
        DbReaderSavedOffline {
            fetched: true,
            ..Default::default()
        }
    );
}

#[test]
fn test_forgetting_failures_keeps_what_was_fetched() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    ReaderOfflineRepository::record_failure(&conn, row, "No", &date("2026-11-05T10:00:00Z"))
        .unwrap();

    ReaderOfflineRepository::forget_failures(&conn, row).unwrap();

    assert_eq!(
        ReaderOfflineRepository::select(&conn, row).unwrap(),
        DbReaderSavedOffline::default()
    );
}

#[test]
fn test_downloads_left_running_are_reset() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    ReaderOfflineRepository::set_downloading(&conn, row, true).unwrap();

    ReaderOfflineRepository::reset_downloading(&conn).unwrap();

    assert!(
        !ReaderOfflineRepository::select(&conn, row)
            .unwrap()
            .downloading
    );
}

#[test]
fn test_offline_state_goes_with_its_save() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, _) = save(&conn, account, 1, "2026-11-01T00:00:00Z", None);
    ReaderOfflineRepository::set_fetched(&conn, row).unwrap();

    ReaderSavedRepository::delete_all(&conn, account).unwrap();

    let left: i64 = conn
        .query_row("SELECT COUNT(*) FROM reader_saved_offline", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(left, 0);
}

#[test]
fn test_posts_for_images_are_saved_posts_with_content_newest_first() {
    let conn = conn();
    let account = account(&conn, 1);
    save(
        &conn,
        account,
        1,
        "2026-11-01T00:00:00Z",
        Some(Some("<p>1</p>")),
    );
    save(
        &conn,
        account,
        2,
        "2026-11-03T00:00:00Z",
        Some(Some("<p>2</p>")),
    );
    save(&conn, account, 3, "2026-11-02T00:00:00Z", Some(None));
    save(&conn, account, 4, "2026-11-04T00:00:00Z", None);

    let posts = ReaderOfflineRepository::select_posts_for_images(&conn, account).unwrap();

    let contents: Vec<_> = posts.iter().map(|post| post.content.as_deref()).collect();
    assert_eq!(contents, [Some("<p>2</p>"), Some("<p>1</p>")]);
    assert!(
        posts
            .iter()
            .all(|post| post.images == DbReaderOfflineImages::NotStored)
    );
}

#[test]
fn test_images_state_is_set_on_every_save_of_a_post() {
    let conn = conn();
    let account = account(&conn, 1);
    let (row, post_row) = save(
        &conn,
        account,
        1,
        "2026-11-01T00:00:00Z",
        Some(Some("<p>1</p>")),
    );
    let post_row = post_row.unwrap();

    ReaderOfflineRepository::set_images(&conn, post_row, DbReaderOfflineImages::Stored).unwrap();

    assert_eq!(
        ReaderOfflineRepository::select(&conn, row).unwrap().images,
        DbReaderOfflineImages::Stored
    );
    assert_eq!(
        ReaderOfflineRepository::count_ready(&conn, account).unwrap(),
        (1, 1)
    );
    let listed = ReaderSavedRepository::select_list(&conn, account).unwrap();
    assert_eq!(listed[0].offline.images, DbReaderOfflineImages::Stored);
}

#[test]
fn test_stored_bytes_count_a_shared_file_once() {
    let conn = conn();
    let account = account(&conn, 1);
    let (_, first) = save(&conn, account, 1, "2026-11-01T00:00:00Z", Some(Some("1")));
    let (_, second) = save(&conn, account, 2, "2026-11-02T00:00:00Z", Some(Some("2")));
    let (first, second) = (first.unwrap(), second.unwrap());

    ReaderOfflineRepository::record_file(&conn, first, "https://e.com/a.jpg", Some("a.jpg"), 100)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, second, "https://e.com/a.jpg", Some("a.jpg"), 100)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, second, "https://e.com/b.jpg", Some("b.jpg"), 50)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, second, "https://e.com/gone.jpg", None, 0).unwrap();

    assert_eq!(ReaderOfflineRepository::stored_bytes(&conn).unwrap(), 150);
    assert_eq!(
        ReaderOfflineRepository::select_stored_file(&conn, account, "https://e.com/a.jpg").unwrap(),
        Some(("a.jpg".to_string(), 100))
    );
    assert_eq!(
        ReaderOfflineRepository::select_stored_file(&conn, account, "https://e.com/gone.jpg")
            .unwrap(),
        None
    );
    let files = ReaderOfflineRepository::select_files(&conn, second).unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(files[2].file_name, None);
}

#[test]
fn test_deleting_a_posts_files_returns_only_files_no_post_has() {
    let conn = conn();
    let account = account(&conn, 1);
    let (_, first) = save(&conn, account, 1, "2026-11-01T00:00:00Z", Some(Some("1")));
    let (_, second) = save(&conn, account, 2, "2026-11-02T00:00:00Z", Some(Some("2")));
    let (first, second) = (first.unwrap(), second.unwrap());
    ReaderOfflineRepository::record_file(&conn, first, "https://e.com/a.jpg", Some("a.jpg"), 100)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, second, "https://e.com/a.jpg", Some("a.jpg"), 100)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, second, "https://e.com/b.jpg", Some("b.jpg"), 50)
        .unwrap();

    ReaderOfflineRepository::set_images(&conn, second, DbReaderOfflineImages::Stored).unwrap();

    assert_eq!(
        ReaderOfflineRepository::delete_files_of_post(&conn, second, DbReaderOfflineImages::NoRoom)
            .unwrap(),
        ["b.jpg"],
        "the first post still has a.jpg"
    );
    let listed = ReaderSavedRepository::select_list(&conn, account).unwrap();
    assert_eq!(listed[0].row.key, post_key(2));
    assert_eq!(listed[0].offline.images, DbReaderOfflineImages::NoRoom);
    assert_eq!(
        ReaderOfflineRepository::delete_files_of_post(
            &conn,
            first,
            DbReaderOfflineImages::NotStored
        )
        .unwrap(),
        ["a.jpg"]
    );
    assert_eq!(ReaderOfflineRepository::stored_bytes(&conn).unwrap(), 0);
}

#[test]
fn test_deleting_every_file_of_an_account_leaves_the_others() {
    let conn = conn();
    let account = account(&conn, 1);
    let other = self::account(&conn, 2);
    let (row, mine) = save(&conn, account, 1, "2026-11-01T00:00:00Z", Some(Some("1")));
    let (_, others) = save(&conn, other, 2, "2026-11-02T00:00:00Z", Some(Some("2")));
    let (mine, others) = (mine.unwrap(), others.unwrap());
    ReaderOfflineRepository::record_file(&conn, mine, "https://e.com/a.jpg", Some("a.jpg"), 10)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, mine, "https://e.com/b.jpg", Some("b.jpg"), 10)
        .unwrap();
    ReaderOfflineRepository::record_file(&conn, others, "https://e.com/a.jpg", Some("a.jpg"), 10)
        .unwrap();
    ReaderOfflineRepository::set_images(&conn, mine, DbReaderOfflineImages::Stored).unwrap();

    assert_eq!(
        ReaderOfflineRepository::delete_all_files(&conn, account).unwrap(),
        ["b.jpg"],
        "the other account's post still has a.jpg"
    );
    assert_eq!(
        ReaderOfflineRepository::select(&conn, row).unwrap().images,
        DbReaderOfflineImages::NotStored
    );
    assert_eq!(ReaderOfflineRepository::stored_bytes(&conn).unwrap(), 10);
}

#[test]
fn test_a_post_is_read_by_its_row() {
    let conn = conn();
    let account = account(&conn, 1);
    let (_, row) = save(
        &conn,
        account,
        1,
        "2026-11-01T00:00:00Z",
        Some(Some("<p>Hi</p>")),
    );

    let found = ReaderPostRepository::select_by_row_id(&conn, row.unwrap())
        .unwrap()
        .unwrap();

    assert_eq!(found.content.as_deref(), Some("<p>Hi</p>"));
    assert_eq!(
        ReaderPostRepository::select_by_row_id(&conn, RowId(999)).unwrap(),
        None
    );
}

#[test]
fn test_files_of_unsaved_posts_are_deleted() {
    let conn = conn();
    let account = account(&conn, 1);
    let other = self::account(&conn, 2);
    let (_, kept) = save(&conn, account, 1, "2026-11-01T00:00:00Z", Some(Some("1")));
    let (_, unsaved) = save(&conn, account, 2, "2026-11-02T00:00:00Z", Some(Some("2")));
    let (_, others) = save(&conn, other, 3, "2026-11-02T00:00:00Z", Some(Some("3")));
    for (post, name) in [
        (kept, "kept.jpg"),
        (unsaved, "unsaved.jpg"),
        (others, "others.jpg"),
    ] {
        ReaderOfflineRepository::record_file(
            &conn,
            post.unwrap(),
            "https://e.com/a.jpg",
            Some(name),
            10,
        )
        .unwrap();
    }
    ReaderOfflineRepository::set_images(&conn, unsaved.unwrap(), DbReaderOfflineImages::Stored)
        .unwrap();
    let unsaved_rows = ReaderSavedRepository::tap_unsave(&conn, account, &[post_key(2)]).unwrap();

    assert_eq!(
        ReaderOfflineRepository::delete_files_of_unsaved_posts(&conn, account).unwrap(),
        ["unsaved.jpg"]
    );
    assert_eq!(
        ReaderOfflineRepository::select(&conn, unsaved_rows[0])
            .unwrap()
            .images,
        DbReaderOfflineImages::NotStored,
        "a save restored later starts without images"
    );
    let mut names: Vec<_> = ReaderOfflineRepository::select_file_names(&conn)
        .unwrap()
        .into_iter()
        .collect();
    names.sort();
    assert_eq!(names, ["kept.jpg", "others.jpg"]);
    assert_eq!(
        ReaderOfflineRepository::delete_files_of_unsaved_posts(&conn, account).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn test_the_list_can_be_ordered_by_either_date_both_ways() {
    let conn = conn();
    let account = account(&conn, 1);
    // Saved in the order 1, 2, 3, and published in the order 2, 3, 1. Post 4 has no post yet.
    for (id, saved_at, published) in [
        (1, "2026-11-01T00:00:00Z", "2026-10-03T00:00:00Z"),
        (2, "2026-11-02T00:00:00Z", "2026-10-01T00:00:00Z"),
        (3, "2026-11-03T00:00:00Z", "2026-10-02T00:00:00Z"),
    ] {
        let record = ReaderPostRecord {
            date: Some(date(published)),
            ..post(id, Some("<p>Hi</p>"))
        };
        let post_row = ReaderPostRepository::upsert(&conn, account, &record).unwrap();
        let key = post_key(id as i64);
        ReaderSavedRepository::tap_save(
            &conn,
            account,
            &key,
            std::slice::from_ref(&key),
            Some(post_row),
            &date(saved_at),
        )
        .unwrap();
    }
    save(&conn, account, 4, "2026-11-04T00:00:00Z", None);

    let order = |order_by, order| -> Vec<ReaderSavedKey> {
        ReaderSavedRepository::select_list_ordered(&conn, account, order_by, order)
            .unwrap()
            .into_iter()
            .map(|item| item.row.key)
            .collect()
    };
    let keys = |ids: &[i64]| -> Vec<ReaderSavedKey> { ids.iter().copied().map(post_key).collect() };

    assert_eq!(
        order(ReaderSavedOrderBy::Date, ReaderStreamOrder::Desc),
        keys(&[4, 3, 2, 1])
    );
    assert_eq!(
        order(ReaderSavedOrderBy::Date, ReaderStreamOrder::Asc),
        keys(&[1, 2, 3, 4])
    );
    assert_eq!(
        order(ReaderSavedOrderBy::DatePublished, ReaderStreamOrder::Desc),
        keys(&[1, 3, 2, 4]),
        "a save without the date comes last"
    );
    assert_eq!(
        order(ReaderSavedOrderBy::DatePublished, ReaderStreamOrder::Asc),
        keys(&[2, 3, 1, 4]),
        "a save without the date comes last"
    );
}

/// The saved list in another order keeps where it's up to, as the saved list does.
#[test]
fn test_stale_streams_are_deleted_except_the_saved_list_in_any_order() {
    use crate::repository::reader::ReaderStreamRepository;
    let conn = conn();
    let account = account(&conn, 1);
    for key in [
        "saved",
        "saved:date_published:ASC",
        "savedness",
        "following",
    ] {
        ReaderStreamRepository::get_or_create(&conn, account, key).unwrap();
    }

    ReaderStreamRepository::delete_stale(&conn, account, &date("2026-11-05T10:00:00Z"), "saved")
        .unwrap();

    let mut statement = conn
        .prepare("SELECT key FROM reader_streams ORDER BY key")
        .unwrap();
    let keys: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(keys, ["saved", "saved:date_published:ASC"]);
}
