use std::sync::Arc;
use tempfile::TempDir;
use wp_api::{
    date::WpGmtDateTime,
    media::MediaId,
    parsed_url::ParsedUrl,
    posts::{PostCommentStatus, PostFormat, PostId},
    terms::TermId,
    users::UserId,
    wp_com::WpComSiteId,
};
use wp_mobile::service::local_content::LocalContentService;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        working_copy::{
            FeaturedImage, PageWorkingCopy, PostWorkingCopy, PublishDate, StatusSetting, Tag,
            WorkingCopyCommon, WorkingCopyId,
        },
        working_copy_target::{RemotePostReference, SiteIdentity, WorkingCopyTarget},
    },
};

/// The database file lives in `dir`, which is deleted when it's dropped.
fn database_path(dir: &TempDir) -> String {
    dir.path()
        .join("local_content.db")
        .to_str()
        .expect("Temp path should be UTF-8")
        .to_string()
}

fn open_service(dir: &TempDir) -> LocalContentService {
    let store =
        WpLocalContentStore::new(database_path(dir)).expect("Opening the store should succeed");
    store
        .perform_migrations()
        .expect("Migrations should succeed");
    LocalContentService::new(Arc::new(store))
}

fn date(rfc3339: &str) -> WpGmtDateTime {
    rfc3339.parse().expect("Date should parse")
}

fn common(title: &str) -> WorkingCopyCommon {
    WorkingCopyCommon {
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

fn post(title: &str) -> PostWorkingCopy {
    PostWorkingCopy {
        common: common(title),
        categories: vec![],
        tags: vec![],
        format: PostFormat::Standard,
        sticky: false,
    }
}

fn self_hosted_site() -> SiteIdentity {
    SiteIdentity {
        url: Arc::new(ParsedUrl::parse("https://example.com").expect("URL should parse")),
        wordpress_com_site_id: None,
    }
}

fn new_post_on(site: SiteIdentity) -> WorkingCopyTarget {
    WorkingCopyTarget {
        site,
        remote_post: None,
    }
}

#[test]
fn test_saved_post_working_copy_survives_reopening_the_database() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let site = SiteIdentity {
        url: Arc::new(ParsedUrl::parse("https://example.wordpress.com").expect("URL should parse")),
        wordpress_com_site_id: Some(WpComSiteId(123_456_789)),
    };
    let edited = PostWorkingCopy {
        common: WorkingCopyCommon {
            status: StatusSetting::Publish {
                password: Some("secret".to_string()),
            },
            publish_date: PublishDate::At {
                date: date("2030-03-17T17:46:40Z"),
            },
            author: Some(UserId(7)),
            featured_image: FeaturedImage::Remote {
                media_id: MediaId(42),
            },
            comment_status: Some(PostCommentStatus::Closed),
            ..common("Edited")
        },
        categories: vec![TermId(1)],
        tags: vec![
            Tag::Existing { id: TermId(3) },
            Tag::New {
                name: "new tag".to_string(),
            },
        ],
        format: PostFormat::Aside,
        sticky: true,
    };

    let id = {
        let service = open_service(&dir);
        let id = service
            .read_or_create_post_working_copy(new_post_on(site.clone()), post("Original"))
            .expect("Create should succeed")
            .id;
        service
            .save_post_working_copy(id.clone(), edited.clone())
            .expect("Save should succeed");
        id
    };
    let read = open_service(&dir)
        .read_post_working_copy(id)
        .expect("Read should succeed")
        .expect("Working copy should exist after reopening");

    assert_eq!(read.working_copy, edited);
    assert_eq!(
        read.target,
        WorkingCopyTarget {
            site,
            remote_post: None
        }
    );
}

#[test]
fn test_saved_page_working_copy_survives_reopening_the_database() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let edited = PageWorkingCopy {
        common: common("Edited page"),
        parent: Some(PostId(3)),
    };

    let id = {
        let service = open_service(&dir);
        let id = service
            .read_or_create_page_working_copy(
                new_post_on(self_hosted_site()),
                PageWorkingCopy {
                    common: common("Original page"),
                    parent: None,
                },
            )
            .expect("Create should succeed")
            .id;
        service
            .save_page_working_copy(id.clone(), edited.clone())
            .expect("Save should succeed");
        id
    };
    let read = open_service(&dir)
        .read_page_working_copy(id)
        .expect("Read should succeed")
        .expect("Working copy should exist after reopening");

    assert_eq!(read.working_copy, edited);
    assert_eq!(
        read.target,
        WorkingCopyTarget {
            site: self_hosted_site(),
            remote_post: None
        }
    );
}

#[test]
fn test_saving_a_missing_post_working_copy_fails() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let page_id = service
        .read_or_create_page_working_copy(
            new_post_on(self_hosted_site()),
            PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
        )
        .expect("Create should succeed")
        .id;
    let unknown_id = WorkingCopyId("00000000-0000-4000-8000-000000000000".to_string());

    for id in [unknown_id, page_id] {
        let result = service.save_post_working_copy(id.clone(), post("Title"));
        assert_eq!(
            result,
            Err(LocalContentError::WorkingCopyNotFound { id }),
            "Saving as a post should fail for an unknown ID and for a page's ID"
        );
    }
}

#[test]
fn test_deleting_a_post_working_copy_deletes_only_that_post() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let post_id = service
        .read_or_create_post_working_copy(new_post_on(self_hosted_site()), post("To delete"))
        .expect("Create should succeed")
        .id;
    let page_id = service
        .read_or_create_page_working_copy(
            new_post_on(self_hosted_site()),
            PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
        )
        .expect("Create should succeed")
        .id;

    service
        .delete_post_working_copy(post_id.clone())
        .expect("Delete should succeed");

    assert_eq!(
        service.read_post_working_copy(post_id.clone()),
        Ok(None),
        "The deleted post should be gone"
    );
    for id in [post_id, page_id.clone()] {
        assert_eq!(
            service.delete_post_working_copy(id.clone()),
            Err(LocalContentError::WorkingCopyNotFound { id }),
            "Deleting as a post should fail for a deleted post and for a page's ID"
        );
    }
    assert!(
        matches!(service.read_page_working_copy(page_id), Ok(Some(_))),
        "The page should survive being deleted as a post"
    );
}

#[test]
fn test_a_remote_post_gets_its_working_copy_back_and_a_new_post_never_does() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let remote_target = WorkingCopyTarget {
        site: self_hosted_site(),
        remote_post: Some(RemotePostReference {
            post_id: PostId(42),
            base_modified_gmt: date("2030-03-17T17:46:40Z"),
        }),
    };

    let created = service
        .read_or_create_post_working_copy(remote_target.clone(), post("As fetched"))
        .expect("Create should succeed");
    let read = service
        .read_or_create_post_working_copy(remote_target.clone(), post("Fetched again"))
        .expect("Read should succeed");
    let first_new = service
        .read_or_create_post_working_copy(new_post_on(self_hosted_site()), post("New"))
        .expect("Create should succeed");
    let second_new = service
        .read_or_create_post_working_copy(new_post_on(self_hosted_site()), post("New"))
        .expect("Create should succeed");

    assert_eq!(read.id, created.id);
    assert_eq!(read.working_copy, post("As fetched"));
    assert_eq!(read.target, remote_target);
    assert_ne!(first_new.id, second_new.id);
}

#[test]
fn test_a_new_post_recorded_as_pushed_is_found_by_its_remote_post() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let created = service
        .read_or_create_post_working_copy(new_post_on(self_hosted_site()), post("Draft"))
        .expect("Create should succeed");
    let pushed = RemotePostReference {
        post_id: PostId(2007),
        base_modified_gmt: date("2030-03-17T17:46:40Z"),
    };
    let unknown_id = WorkingCopyId("00000000-0000-4000-8000-000000000000".to_string());

    service
        .record_post_working_copy_push(created.id.clone(), pushed)
        .expect("Recording the push should succeed");
    let found = service
        .read_or_create_post_working_copy(
            WorkingCopyTarget {
                site: self_hosted_site(),
                remote_post: Some(pushed),
            },
            post("Fetched"),
        )
        .expect("Read should succeed");

    assert_eq!(found.id, created.id);
    assert_eq!(found.working_copy, post("Draft"));
    assert_eq!(found.target.remote_post, Some(pushed));
    assert_eq!(
        service.record_post_working_copy_push(unknown_id.clone(), pushed),
        Err(LocalContentError::WorkingCopyNotFound { id: unknown_id })
    );
}
