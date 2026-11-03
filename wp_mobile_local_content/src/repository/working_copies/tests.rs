//! Tests for `WorkingCopyRepository`, grouped by topic, and the fixtures they
//! share.

use super::*;
use crate::{
    MIGRATIONS,
    db_types::working_copy::{StatusSetting, WorkingCopyId, WorkingCopyTag, date_from_timestamp},
};
use rusqlite::Connection;
use std::sync::Arc;
use wp_api::{
    date::WpGmtDateTime,
    media::MediaId,
    parsed_url::ParsedUrl,
    posts::{PostCommentStatus, PostPingStatus},
    terms::TermId,
    wp_com::WpComSiteId,
};
use wp_mobile_sqlite::MigrationManager;

mod fields;
mod remote_posts;
mod schema;
mod sites;

fn migrated_connection() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to open in-memory database");
    MigrationManager::new(&conn, &MIGRATIONS)
        .expect("Failed to create MigrationManager")
        .perform_migrations()
        .expect("Migrations should succeed");
    conn
}

fn date(seconds: i64) -> WpGmtDateTime {
    date_from_timestamp(seconds).expect("Timestamp should be in range")
}

fn common(title: &str) -> WorkingCopyCommonFields {
    WorkingCopyCommonFields {
        title: title.to_string(),
        content: format!("<!-- wp:paragraph --><p>{title}</p><!-- /wp:paragraph -->"),
        excerpt: String::new(),
        slug: None,
        status: StatusSetting::Draft { password: None },
        publish_date: PublishDate::Immediately,
        author: None,
        featured_image: FeaturedImage::None,
        comment_status: None,
        ping_status: None,
    }
}

/// A post with every optional or enum field set away from its default.
fn detailed_post() -> PostWorkingCopyFields {
    PostWorkingCopyFields {
        common: WorkingCopyCommonFields {
            excerpt: "An excerpt".to_string(),
            slug: Some("a-slug".to_string()),
            status: StatusSetting::Custom {
                status: "in-review".to_string(),
                password: Some("secret".to_string()),
            },
            publish_date: PublishDate::At {
                date: date(1_900_000_000),
            },
            author: Some(UserId(7)),
            featured_image: FeaturedImage::Remote {
                media_id: MediaId(42),
            },
            comment_status: Some(PostCommentStatus::Closed),
            ping_status: Some(PostPingStatus::Custom("moderated".to_string())),
            ..common("Detailed post")
        },
        categories: vec![TermId(1), TermId(5)],
        tags: vec![
            WorkingCopyTag::Existing { id: TermId(3) },
            WorkingCopyTag::New {
                name: "not on the site yet".to_string(),
            },
        ],
        format: PostFormat::Custom("theme-specific".to_string()),
        sticky: true,
    }
}

fn simple_post(title: &str) -> PostWorkingCopyFields {
    PostWorkingCopyFields {
        common: common(title),
        categories: vec![],
        tags: vec![],
        format: PostFormat::Standard,
        sticky: false,
    }
}

fn site(url: &str, wordpress_com_site_id: Option<u64>) -> SiteIdentity {
    SiteIdentity {
        url: Arc::new(ParsedUrl::parse(url).expect("URL should parse")),
        wordpress_com_site_id: wordpress_com_site_id.map(WpComSiteId),
    }
}

fn new_post_on(site: &SiteIdentity) -> WorkingCopyRemoteTarget {
    WorkingCopyRemoteTarget {
        site: site.clone(),
        remote_post: None,
    }
}

fn wordpress_com_target_editing(post_id: i64) -> WorkingCopyRemoteTarget {
    WorkingCopyRemoteTarget {
        site: site("https://example.wordpress.com", Some(123_456_789)),
        remote_post: Some(RemotePostReference {
            post_id: PostId(post_id),
            base_modified_gmt: date(1_700_000_000),
        }),
    }
}

fn self_hosted_new_post_target() -> WorkingCopyRemoteTarget {
    new_post_on(&site("https://example.com", None))
}

fn insert_post(
    conn: &mut Connection,
    fields: &PostWorkingCopyFields,
    remote_target: &WorkingCopyRemoteTarget,
) -> WorkingCopyId {
    let tx = conn.transaction().expect("Failed to begin transaction");
    let id = WorkingCopyRepository
        .insert_post(&tx, fields, remote_target)
        .expect("Insert should succeed");
    tx.commit().expect("Commit should succeed");
    id
}

fn insert_page(
    conn: &mut Connection,
    fields: &PageWorkingCopyFields,
    remote_target: &WorkingCopyRemoteTarget,
) -> WorkingCopyId {
    let tx = conn.transaction().expect("Failed to begin transaction");
    let id = WorkingCopyRepository
        .insert_page(&tx, fields, remote_target)
        .expect("Insert should succeed");
    tx.commit().expect("Commit should succeed");
    id
}

fn count_rows(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("Count should succeed")
}
