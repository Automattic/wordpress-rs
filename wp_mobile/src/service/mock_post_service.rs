//! Mock post service for testing
//!
//! This module provides testing utilities for inserting and updating mock posts
//! without requiring the full API client stack. It should be removed once proper
//! data insertion is available through the API client.

use crate::service::posts::PostService;
use std::sync::Arc;
use wp_api::posts::{
    AnyPostWithEditContext, PostContentWithEditContext, PostGuidWithEditContext, PostId,
    PostStatus, PostTitleWithEditContext,
};
use wp_mobile_cache::{context::EditContext, entity::EntityId, repository::posts::PostRepository};

/// Mock post service for testing purposes
///
/// This service wraps PostService and provides utilities to insert and update
/// mock posts for testing the observer pattern and other functionality without
/// needing real API calls.
///
/// **TEMPORARY**: This should be removed once proper data insertion is available.
#[derive(uniffi::Object)]
pub struct MockPostService {
    post_service: Arc<PostService>,
}

impl MockPostService {
    pub fn new(post_service: Arc<PostService>) -> Self {
        Self { post_service }
    }
}

#[uniffi::export]
impl MockPostService {
    /// Create a temporary post with default values
    fn create_temp_post(&self, id: PostId) -> AnyPostWithEditContext {
        AnyPostWithEditContext {
            id,
            date: "2025-01-01T00:00:00".to_string(),
            date_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
            guid: PostGuidWithEditContext {
                raw: None,
                rendered: format!("https://example.com/?p={}", id.0),
            },
            link: format!("https://example.com/test-post-{}", id.0),
            modified: "2025-01-01T00:00:00".to_string(),
            modified_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
            slug: format!("test-post-{}", id.0),
            status: PostStatus::Publish,
            post_type: "post".to_string(),
            password: "".to_string(),
            permalink_template: None,
            generated_slug: None,
            title: PostTitleWithEditContext {
                raw: None,
                rendered: "Test Post".to_string(),
            },
            content: PostContentWithEditContext {
                raw: None,
                rendered: "<p>Test content</p>".to_string(),
                protected: None,
                block_version: None,
            },
            author: None,
            excerpt: None,
            featured_media: None,
            comment_status: None,
            ping_status: None,
            format: None,
            meta: None,
            sticky: None,
            template: "".to_string(),
            categories: None,
            tags: None,
            parent: None,
            menu_order: None,
        }
    }

    /// Insert a mock post for testing purposes
    ///
    /// Returns the EntityId of the inserted post, which can be used to create
    /// observable entities or fetch the post later.
    pub fn insert_mock_post(&self, id: PostId, title: String) -> EntityId {
        let mut post = self.create_temp_post(id);
        post.title.rendered = title;

        let repo = PostRepository::<EditContext>::new();
        let mut conn = self.post_service.cache().connection();
        repo.upsert(&mut *conn, self.post_service.db_site(), &post)
            .expect("Failed to insert mock post")
    }

    /// Update a mock post for testing purposes
    ///
    /// Updates an existing post's title. Used for testing the observer pattern.
    pub fn update_mock_post(&self, id: PostId, new_title: String) {
        let repo = PostRepository::<EditContext>::new();
        let mut conn = self.post_service.cache().connection();
        let mut post = repo
            .select_by_post_id(&*conn, self.post_service.db_site(), id)
            .expect("Failed to read post")
            .expect("Post not found")
            .data
            .post;
        post.title.rendered = new_title;
        repo.upsert(&mut *conn, self.post_service.db_site(), &post)
            .expect("Failed to update mock post");
    }
}
