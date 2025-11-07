//! Mock post service for testing
//!
//! This module provides testing utilities for inserting and updating mock posts
//! without requiring the full API client stack. It should be removed once proper
//! data insertion is available through the API client.

use crate::service::posts::PostService;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
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
        self.post_service
            .cache()
            .execute(|conn| repo.upsert(conn, self.post_service.db_site(), &post))
            .expect("Failed to insert mock post")
    }

    /// Update a mock post for testing purposes
    ///
    /// Updates an existing post's title. Used for testing the observer pattern.
    pub fn update_mock_post(&self, id: PostId, new_title: String) {
        let repo = PostRepository::<EditContext>::new();
        self.post_service
            .cache()
            .execute(|conn| {
                let mut post = repo
                    .select_by_post_id(conn, self.post_service.db_site(), id)?
                    .ok_or_else(|| wp_mobile_cache::SqliteDbError::SqliteError("Post not found".to_string()))?
                    .data
                    .post;
                post.title.rendered = new_title;
                repo.upsert(conn, self.post_service.db_site(), &post)?;
                Ok::<_, wp_mobile_cache::SqliteDbError>(())
            })
            .expect("Failed to update mock post");
    }

    /// Generate and insert multiple mock posts for stress testing
    ///
    /// Creates and inserts the specified number of mock posts with sequential IDs.
    /// Starting from ID 10000 to avoid conflicts with real posts.
    ///
    /// Returns a vector of EntityIds for the inserted posts.
    pub fn generate_and_insert_posts(&self, count: u32) -> Vec<Arc<EntityId>> {
        let mut entity_ids = Vec::with_capacity(count as usize);
        let repo = PostRepository::<EditContext>::new();

        for i in 0..count {
            let post_id = PostId(10000 + i as i64);
            let mut post = self.create_temp_post(post_id);
            post.title.rendered = format!("Stress Test Post {}", i + 1);
            post.slug = format!("stress-test-post-{}", i + 1);
            post.link = format!("https://example.com/stress-test-post-{}", i + 1);

            let entity_id = self
                .post_service
                .cache()
                .execute(|conn| repo.upsert(conn, self.post_service.db_site(), &post))
                .expect("Failed to insert mock post");
            entity_ids.push(Arc::new(entity_id));
        }

        entity_ids
    }

    /// Start randomly updating posts in a background thread
    ///
    /// Spawns a thread that continuously picks random posts from the provided
    /// entity IDs and updates them with random titles. The thread sleeps for
    /// the specified delay (in seconds) between each update.
    ///
    /// Returns a handle that can be used to stop the background updates and
    /// query the current update count.
    ///
    /// # Arguments
    /// * `entity_ids` - The entity IDs to randomly update
    /// * `delay_seconds` - Delay between updates in seconds (can be fractional)
    pub fn start_random_updates(
        &self,
        entity_ids: Vec<Arc<EntityId>>,
        delay_seconds: f64,
    ) -> Arc<StressTestHandle> {
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let update_counter = Arc::new(AtomicU64::new(0));
        let update_counter_clone = update_counter.clone();
        let cache = self.post_service.cache().clone();
        let db_site = *self.post_service.db_site();

        thread::spawn(move || {
            let repo = PostRepository::<EditContext>::new();

            loop {
                // Check if we should stop
                {
                    let should_stop = *stop_flag_clone.lock().unwrap();
                    if should_stop {
                        break;
                    }
                }

                // Pick a random entity ID (using round-robin for simplicity)
                if !entity_ids.is_empty() {
                    let current_count = update_counter_clone.load(Ordering::Relaxed);
                    let random_index = (current_count as usize) % entity_ids.len();
                    let entity_id = &entity_ids[random_index];

                    // Update the post - using the execute() pattern
                    // This prevents the self-deadlock by ensuring the connection is only
                    // held during the closure execution
                    let _result = cache.execute(|conn| {
                        // Read and update in a single atomic operation
                        if let Some(full_entity) = repo.select_by_entity_id(conn, entity_id)? {
                            let mut post = full_entity.data.post;
                            post.title.rendered = format!(
                                "Updated Post {} (update #{})",
                                post.id.0, current_count
                            );
                            repo.upsert(conn, &db_site, &post)?;
                        }
                        Ok::<_, wp_mobile_cache::SqliteDbError>(())
                    });

                    update_counter_clone.fetch_add(1, Ordering::Relaxed);
                }

                // Sleep for the specified delay
                thread::sleep(Duration::from_secs_f64(delay_seconds));
            }
        });

        Arc::new(StressTestHandle {
            stop_flag,
            update_counter,
        })
    }
}

/// Handle for controlling background stress testing
///
/// Allows stopping the background thread that performs random updates
/// and querying the current update count.
#[derive(uniffi::Object)]
pub struct StressTestHandle {
    stop_flag: Arc<Mutex<bool>>,
    update_counter: Arc<AtomicU64>,
}

#[uniffi::export]
impl StressTestHandle {
    /// Stop the background updates
    ///
    /// Signals the background thread to stop. The thread will complete its
    /// current update and then exit.
    pub fn stop(&self) {
        *self.stop_flag.lock().unwrap() = true;
    }

    /// Get the current number of updates performed
    ///
    /// Returns the total count of post updates since starting.
    pub fn update_count(&self) -> u64 {
        self.update_counter.load(Ordering::Relaxed)
    }
}
