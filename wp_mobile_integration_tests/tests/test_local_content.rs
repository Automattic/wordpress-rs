use std::sync::Arc;
use tempfile::TempDir;
use wp_api::{
    date::WpGmtDateTime,
    media::MediaId,
    posts::{PostFormat, PostId},
    terms::TermId,
    users::UserId,
    wp_com::WpComSiteId,
};
use wp_mobile::service::local_content::LocalContentService;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::{
        working_copy::{
            FeaturedImage, PageWorkingCopy, PostWorkingCopy, PublishDate, StatusSetting,
            Visibility, WorkingCopyCommon, WorkingCopyId,
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
        status: StatusSetting::Draft,
        visibility: Visibility::Public,
        publish_date: PublishDate::Immediately,
        author: None,
        featured_image: FeaturedImage::None,
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

fn new_post_target() -> WorkingCopyTarget {
    WorkingCopyTarget {
        site: SiteIdentity::SelfHosted {
            url: "https://example.com".to_string(),
        },
        remote_post: None,
    }
}

#[test]
fn test_saved_post_working_copy_survives_reopening_the_database() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let target = WorkingCopyTarget {
        site: SiteIdentity::WordPressCom {
            site_id: WpComSiteId(123_456_789),
        },
        remote_post: Some(RemotePostReference {
            post_id: PostId(99),
            base_modified_gmt: date("2023-11-14T22:13:20Z"),
        }),
    };
    let edited = PostWorkingCopy {
        common: WorkingCopyCommon {
            status: StatusSetting::Publish,
            visibility: Visibility::PasswordProtected {
                password: "secret".to_string(),
            },
            publish_date: PublishDate::At {
                date: date("2030-03-17T17:46:40Z"),
            },
            author: Some(UserId(7)),
            featured_image: FeaturedImage::Remote {
                media_id: MediaId(42),
            },
            ..common("Edited")
        },
        categories: vec![TermId(1)],
        tags: vec!["new tag".to_string()],
        format: PostFormat::Aside,
        sticky: true,
    };

    let id = {
        let service = open_service(&dir);
        let id = service
            .create_post_working_copy(post("Original"), target.clone())
            .expect("Create should succeed");
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
    assert_eq!(read.target, target);
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
            .create_page_working_copy(
                PageWorkingCopy {
                    common: common("Original page"),
                    parent: None,
                },
                new_post_target(),
            )
            .expect("Create should succeed");
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
    assert_eq!(read.target, new_post_target());
}

#[test]
fn test_saving_a_missing_post_working_copy_fails() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let service = open_service(&dir);
    let page_id = service
        .create_page_working_copy(
            PageWorkingCopy {
                common: common("A page"),
                parent: None,
            },
            new_post_target(),
        )
        .expect("Create should succeed");
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
