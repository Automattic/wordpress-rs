use std::sync::Arc;
use wp_mobile::service::local_content::LocalContentService;
use wp_mobile_local_content::{
    LocalContentError, WpLocalContentStore,
    db_types::post_working_copy::{PostWorkingCopy, PostWorkingCopyId},
};

fn open_store(path: Option<String>) -> Arc<WpLocalContentStore> {
    let store = WpLocalContentStore::new(path).expect("Opening the store should succeed");
    store
        .perform_migrations()
        .expect("Migrations should succeed");
    Arc::new(store)
}

fn working_copy(title: &str, content: &str) -> PostWorkingCopy {
    PostWorkingCopy {
        title: title.to_string(),
        content: content.to_string(),
    }
}

#[test]
fn test_create_then_read_post_working_copy() {
    let service = LocalContentService::new(open_store(None));
    let created = working_copy("Draft title", "Draft content");

    let id = service
        .create_post_working_copy(created.clone())
        .expect("Create should succeed");
    let read = service
        .read_post_working_copy(id.clone())
        .expect("Read should succeed")
        .expect("Created working copy should exist");

    assert_eq!(read.id, id);
    assert_eq!(read.working_copy, created);
}

#[test]
fn test_save_then_read_post_working_copy() {
    let service = LocalContentService::new(open_store(None));
    let id = service
        .create_post_working_copy(working_copy("First", "first"))
        .expect("Create should succeed");

    let saved = working_copy("Second", "second");
    service
        .save_post_working_copy(id.clone(), saved.clone())
        .expect("Save should succeed");
    let read = service
        .read_post_working_copy(id)
        .expect("Read should succeed")
        .expect("Saved working copy should exist");

    assert_eq!(read.working_copy, saved);
}

#[test]
fn test_save_unknown_post_working_copy_fails() {
    let service = LocalContentService::new(open_store(None));
    let id = PostWorkingCopyId("00000000-0000-4000-8000-000000000000".to_string());

    let result = service.save_post_working_copy(id.clone(), working_copy("Title", "content"));

    assert_eq!(
        result,
        Err(LocalContentError::PostWorkingCopyNotFound { id })
    );
}

#[test]
fn test_post_working_copy_survives_reopening_the_database() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir
        .path()
        .join("local_content.db")
        .to_str()
        .expect("Temp path should be UTF-8")
        .to_string();
    let created = working_copy("Kept", "kept content");

    let id = LocalContentService::new(open_store(Some(path.clone())))
        .create_post_working_copy(created.clone())
        .expect("Create should succeed");

    let read = LocalContentService::new(open_store(Some(path)))
        .read_post_working_copy(id)
        .expect("Read should succeed")
        .expect("Working copy should exist after reopening");

    assert_eq!(read.working_copy, created);
}
