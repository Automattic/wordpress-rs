//! Mock post service for testing
//!
//! This module provides testing utilities for inserting and updating mock posts
//! without requiring the full API client stack. It should be removed once proper
//! data insertion is available through the API client.

use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use wp_api::posts::{
    AnyPostWithEditContext, PostContentWithEditContext, PostGuidWithEditContext, PostId,
    PostStatus, PostTitleWithEditContext,
};
use wp_mobile_cache::{
    WpApiCache, context::EditContext, db_types::db_site::DbSite,
    db_types::self_hosted_site::SelfHostedSite, entity::EntityId,
    repository::posts::PostRepository, repository::sites::SiteRepository,
};

/// Mock post service for testing purposes
///
/// This service provides utilities to insert and update mock posts directly
/// in the cache for testing the observer pattern and other functionality without
/// needing real API calls.
///
/// **TEMPORARY**: This should be removed once proper data insertion is available.
#[derive(uniffi::Object)]
pub struct MockPostService {
    cache: Arc<WpApiCache>,
    db_site: DbSite,
}

impl MockPostService {
    /// Get or create a test DbSite for the mock service
    fn get_or_create_test_db_site(
        cache: &WpApiCache,
        site_url: &str,
    ) -> Result<DbSite, wp_mobile_cache::SqliteDbError> {
        let site_repository = SiteRepository;
        let api_root = format!("{}/wp-json/wp/v2", site_url);

        cache.execute(|conn| {
            // Try to find existing test site
            if let Some(full_entity) =
                site_repository.select_self_hosted_site_by_url(conn, site_url)?
            {
                return Ok(full_entity.data.0);
            }

            // Site doesn't exist, create it
            let self_hosted_site = SelfHostedSite {
                url: site_url.to_string(),
                api_root,
            };

            let entity_id = site_repository.upsert_self_hosted_site(conn, &self_hosted_site)?;
            Ok(entity_id.db_site)
        })
    }
}

#[uniffi::export]
impl MockPostService {
    /// Create a new MockPostService for testing
    ///
    /// # Arguments
    /// * `cache` - The cache instance to use for database operations
    /// * `site_url` - The site URL to use (e.g., "https://test.example.com")
    #[uniffi::constructor]
    pub fn new(cache: Arc<WpApiCache>, site_url: String) -> Self {
        let db_site = Self::get_or_create_test_db_site(&cache, &site_url)
            .expect("Failed to create test DB site for mock service");
        Self { cache, db_site }
    }

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
        self.cache
            .execute(|conn| repo.upsert(conn, &self.db_site, &post))
            .expect("Failed to insert mock post")
    }

    /// Update a mock post for testing purposes
    ///
    /// Updates an existing post's title. Used for testing the observer pattern.
    pub fn update_mock_post(&self, id: PostId, new_title: String) {
        let repo = PostRepository::<EditContext>::new();
        self.cache
            .execute(|conn| {
                let mut post = repo
                    .select_by_post_id(conn, &self.db_site, id)?
                    .ok_or_else(|| wp_mobile_cache::SqliteDbError::SqliteError("Post not found".to_string()))?
                    .data
                    .post;
                post.title.rendered = new_title;
                repo.upsert(conn, &self.db_site, &post)?;
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
                .cache
                .execute(|conn| repo.upsert(conn, &self.db_site, &post))
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
        let cache = self.cache.clone();
        let db_site = self.db_site;

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

    /// Start a comprehensive stress test with variable batch sizes and timing
    ///
    /// This provides a more realistic stress test than `start_random_updates()`:
    /// - Updates multiple posts per batch (1-50 posts)
    /// - Variable timing (bursts and quiet periods)
    /// - Uses actual random selection instead of round-robin
    ///
    /// # Arguments
    /// * `entity_ids` - The entity IDs to randomly update
    /// * `min_delay_ms` - Minimum delay between batches in milliseconds
    /// * `max_delay_ms` - Maximum delay between batches in milliseconds
    /// * `min_batch_size` - Minimum number of posts to update per batch
    /// * `max_batch_size` - Maximum number of posts to update per batch
    pub fn start_comprehensive_stress_test(
        &self,
        entity_ids: Vec<Arc<EntityId>>,
        min_delay_ms: u64,
        max_delay_ms: u64,
        min_batch_size: u32,
        max_batch_size: u32,
    ) -> Arc<StressTestHandle> {
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let update_counter = Arc::new(AtomicU64::new(0));
        let update_counter_clone = update_counter.clone();
        let cache = self.cache.clone();
        let db_site = self.db_site;

        thread::spawn(move || {
            let repo = PostRepository::<EditContext>::new();
            let mut rng = rand::thread_rng();

            loop {
                // Check if we should stop
                {
                    let should_stop = *stop_flag_clone.lock().unwrap();
                    if should_stop {
                        break;
                    }
                }

                if entity_ids.is_empty() {
                    break;
                }

                // Determine batch size for this iteration
                let batch_size = rng.gen_range(min_batch_size..=max_batch_size);
                let batch_size = batch_size.min(entity_ids.len() as u32);

                // Select random posts for this batch
                let mut batch_indices = Vec::new();
                for _ in 0..batch_size {
                    let idx = rng.gen_range(0..entity_ids.len());
                    batch_indices.push(idx);
                }

                // Update all posts in the batch
                let current_count = update_counter_clone.load(Ordering::Relaxed);

                for idx in batch_indices {
                    let entity_id = &entity_ids[idx];

                    let _result = cache.execute(|conn| {
                        if let Some(full_entity) = repo.select_by_entity_id(conn, entity_id)? {
                            let mut post = full_entity.data.post;
                            post.title.rendered = format!(
                                "Updated Post {} (batch update #{})",
                                post.id.0, current_count
                            );
                            post.content.rendered = format!(
                                "<p>Content updated at batch #{}</p>",
                                current_count
                            );
                            repo.upsert(conn, &db_site, &post)?;
                        }
                        Ok::<_, wp_mobile_cache::SqliteDbError>(())
                    });

                    update_counter_clone.fetch_add(1, Ordering::Relaxed);
                }

                // Variable delay between batches
                let delay_ms = rng.gen_range(min_delay_ms..=max_delay_ms);
                thread::sleep(Duration::from_millis(delay_ms));
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
