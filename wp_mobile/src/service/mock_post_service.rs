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

/// Operation type for stress testing
enum StressTestOperation {
    Update,
    Delete,
    Insert,
}

/// Configuration for comprehensive stress testing
#[derive(uniffi::Record)]
pub struct StressTestConfig {
    /// Minimum delay between batches in milliseconds
    pub min_delay_ms: u64,
    /// Maximum delay between batches in milliseconds
    pub max_delay_ms: u64,
    /// Minimum number of posts to operate on per batch
    pub min_batch_size: u32,
    /// Maximum number of posts to operate on per batch
    pub max_batch_size: u32,
    /// Relative weight for update operations (e.g., 50 = 50% if all weights sum to 100)
    pub update_weight: u32,
    /// Relative weight for delete operations
    pub delete_weight: u32,
    /// Relative weight for insert operations
    pub insert_weight: u32,
}

/// Status values for randomizing post status during stress testing
const STRESS_TEST_STATUS_VALUES: [PostStatus; 4] = [
    PostStatus::Draft,
    PostStatus::Pending,
    PostStatus::Publish,
    PostStatus::Future,
];

/// Perform batch update operation for stress testing
fn stress_test_batch_update(
    cache: &Arc<WpApiCache>,
    db_site: &DbSite,
    repo: &PostRepository<EditContext>,
    entity_ids: &[EntityId],
    batch_indices: &[usize],
    current_count: u64,
) {
    batch_indices.iter().for_each(|&idx| {
        let entity_id = &entity_ids[idx];
        let _result = cache.execute(|conn| {
            if let Some(full_entity) = repo.select_by_entity_id(conn, entity_id)? {
                let mut post = full_entity.data.post;
                post.title.rendered =
                    format!("Updated Post {} (batch #{})", post.id.0, current_count);
                post.content.rendered =
                    format!("<p>Content updated at batch #{}</p>", current_count);

                // Randomize the post status
                let mut rng = rand::thread_rng();
                let status_index = rng.gen_range(0..STRESS_TEST_STATUS_VALUES.len());
                post.status = STRESS_TEST_STATUS_VALUES[status_index].clone();

                repo.upsert(conn, db_site, &post)?;
            }
            Ok::<_, wp_mobile_cache::SqliteDbError>(())
        });
    });
}

/// Perform batch delete operation for stress testing
fn stress_test_batch_delete(
    cache: &Arc<WpApiCache>,
    repo: &PostRepository<EditContext>,
    entity_ids: &[EntityId],
    batch_indices: &[usize],
) {
    batch_indices.iter().for_each(|&idx| {
        let entity_id = &entity_ids[idx];
        let _result = cache.execute(|conn| repo.delete_by_entity_id(conn, entity_id));
    });
}

/// Perform batch insert operation for stress testing
fn stress_test_batch_insert(
    cache: &Arc<WpApiCache>,
    db_site: &DbSite,
    repo: &PostRepository<EditContext>,
    batch_size: u32,
    next_insert_id: &mut i64,
    current_count: u64,
) {
    (0..batch_size).for_each(|_| {
        let post_id = PostId(*next_insert_id);
        *next_insert_id += 1;

        let title = format!("Stress Insert {} (batch #{})", post_id.0, current_count);
        let slug = format!("stress-insert-{}", post_id.0);
        let link = format!("https://example.com/{}", slug);
        let content = format!("<p>Inserted at batch #{}</p>", current_count);
        let post = create_test_post(post_id, &title, &slug, &link, &content);

        let _result = cache.execute(|conn| repo.upsert(conn, db_site, &post));
    });
}

/// Create a temporary post with default values for testing
fn create_test_post(
    id: PostId,
    title: &str,
    slug: &str,
    link: &str,
    content: &str,
) -> AnyPostWithEditContext {
    AnyPostWithEditContext {
        id,
        date: "2025-01-01T00:00:00".to_string(),
        date_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
        guid: PostGuidWithEditContext {
            raw: None,
            rendered: format!("https://example.com/?p={}", id.0),
        },
        link: link.to_string(),
        modified: "2025-01-01T00:00:00".to_string(),
        modified_gmt: "2025-01-01T00:00:00Z".parse().unwrap(),
        slug: slug.to_string(),
        status: PostStatus::Publish,
        post_type: "post".to_string(),
        password: "".to_string(),
        permalink_template: None,
        generated_slug: None,
        title: PostTitleWithEditContext {
            raw: None,
            rendered: title.to_string(),
        },
        content: PostContentWithEditContext {
            raw: None,
            rendered: content.to_string(),
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

/// Mock post service for testing purposes
///
/// This service provides utilities to insert and update mock posts directly
/// in the cache for testing the observer pattern and other functionality without
/// needing real API calls.
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
        api_root: &str,
    ) -> Result<DbSite, wp_mobile_cache::SqliteDbError> {
        let site_repository = SiteRepository;

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
                api_root: api_root.to_string(),
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
    /// * `api_root` - The API root URL to use (e.g., "https://test.example.com/wp-json")
    #[uniffi::constructor]
    pub fn new(cache: Arc<WpApiCache>, site_url: String, api_root: String) -> Self {
        let db_site = Self::get_or_create_test_db_site(&cache, &site_url, &api_root)
            .expect("Failed to create test DB site for mock service");
        Self { cache, db_site }
    }

    /// Insert a mock post for testing purposes
    ///
    /// Returns the EntityId of the inserted post, which can be used to create
    /// observable entities or fetch the post later.
    pub fn insert_mock_post(&self, id: PostId, title: String) -> EntityId {
        let slug = format!("test-post-{}", id.0);
        let link = format!("https://example.com/{}", slug);
        let post = create_test_post(id, &title, &slug, &link, "<p>Test content</p>");

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
                    .ok_or_else(|| {
                        wp_mobile_cache::SqliteDbError::SqliteError("Post not found".to_string())
                    })?
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
    pub fn generate_and_insert_posts(&self, count: u32) -> Vec<EntityId> {
        let mut entity_ids = Vec::with_capacity(count as usize);
        let repo = PostRepository::<EditContext>::new();

        for i in 0..count {
            let post_id = PostId(10000 + i as i64);
            let title = format!("Stress Test Post {}", i + 1);
            let slug = format!("stress-test-post-{}", i + 1);
            let link = format!("https://example.com/{}", slug);
            let post = create_test_post(post_id, &title, &slug, &link, "<p>Test content</p>");

            let entity_id = self
                .cache
                .execute(|conn| repo.upsert(conn, &self.db_site, &post))
                .expect("Failed to insert mock post");
            entity_ids.push(entity_id);
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
        entity_ids: Vec<EntityId>,
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
                            post.title.rendered =
                                format!("Updated Post {} (update #{})", post.id.0, current_count);
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
            insert_counter: Arc::new(AtomicU64::new(0)),
            delete_counter: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Start a comprehensive stress test with variable batch sizes and timing
    ///
    /// This provides a more realistic stress test than `start_random_updates()`:
    /// - Randomly updates, deletes, and inserts posts based on operation weights
    /// - Variable batch sizes and timing (bursts and quiet periods)
    /// - Uses actual random selection instead of round-robin
    ///
    /// # Arguments
    /// * `entity_ids` - The entity IDs to randomly update/delete
    /// * `config` - Configuration for the stress test behavior
    pub fn start_comprehensive_stress_test(
        &self,
        entity_ids: Vec<EntityId>,
        config: StressTestConfig,
    ) -> Arc<StressTestHandle> {
        let stop_flag = Arc::new(Mutex::new(false));
        let stop_flag_clone = stop_flag.clone();
        let update_counter = Arc::new(AtomicU64::new(0));
        let update_counter_clone = update_counter.clone();
        let insert_counter = Arc::new(AtomicU64::new(0));
        let insert_counter_clone = insert_counter.clone();
        let delete_counter = Arc::new(AtomicU64::new(0));
        let delete_counter_clone = delete_counter.clone();
        let cache = self.cache.clone();
        let db_site = self.db_site;

        thread::spawn(move || {
            let repo = PostRepository::<EditContext>::new();
            let mut rng = rand::thread_rng();
            let mut next_insert_id: i64 = 20000; // Start IDs for inserted posts

            // Calculate total weight for operation selection
            let total_weight = config.update_weight + config.delete_weight + config.insert_weight;
            if total_weight == 0 {
                return; // No operations to perform
            }

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
                let batch_size = rng.gen_range(config.min_batch_size..=config.max_batch_size);
                let batch_size = batch_size.min(entity_ids.len() as u32);

                // Choose operation based on weights
                let roll = rng.gen_range(0..total_weight);
                let operation = if roll < config.update_weight {
                    StressTestOperation::Update
                } else if roll < config.update_weight + config.delete_weight {
                    StressTestOperation::Delete
                } else {
                    StressTestOperation::Insert
                };

                let current_count = update_counter_clone.load(Ordering::Relaxed);

                // Select random posts for this batch
                let batch_indices: Vec<usize> = (0..batch_size)
                    .map(|_| rng.gen_range(0..entity_ids.len()))
                    .collect();

                match operation {
                    StressTestOperation::Update => {
                        stress_test_batch_update(
                            &cache,
                            &db_site,
                            &repo,
                            &entity_ids,
                            &batch_indices,
                            current_count,
                        );
                        update_counter_clone.fetch_add(batch_size as u64, Ordering::Relaxed);
                    }
                    StressTestOperation::Delete => {
                        stress_test_batch_delete(&cache, &repo, &entity_ids, &batch_indices);
                        delete_counter_clone.fetch_add(batch_size as u64, Ordering::Relaxed);
                    }
                    StressTestOperation::Insert => {
                        stress_test_batch_insert(
                            &cache,
                            &db_site,
                            &repo,
                            batch_size,
                            &mut next_insert_id,
                            current_count,
                        );
                        insert_counter_clone.fetch_add(batch_size as u64, Ordering::Relaxed);
                    }
                }

                // Variable delay between batches
                let delay_ms = rng.gen_range(config.min_delay_ms..=config.max_delay_ms);
                thread::sleep(Duration::from_millis(delay_ms));
            }
        });

        Arc::new(StressTestHandle {
            stop_flag,
            update_counter,
            insert_counter,
            delete_counter,
        })
    }
}

/// Handle for controlling background stress testing
///
/// Allows stopping the background thread that performs random updates
/// and querying the current update, insert, and delete counts.
#[derive(uniffi::Object)]
pub struct StressTestHandle {
    stop_flag: Arc<Mutex<bool>>,
    update_counter: Arc<AtomicU64>,
    insert_counter: Arc<AtomicU64>,
    delete_counter: Arc<AtomicU64>,
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

    /// Get the current number of inserts performed
    ///
    /// Returns the total count of post inserts since starting.
    pub fn insert_count(&self) -> u64 {
        self.insert_counter.load(Ordering::Relaxed)
    }

    /// Get the current number of deletes performed
    ///
    /// Returns the total count of post deletes since starting.
    pub fn delete_count(&self) -> u64 {
        self.delete_counter.load(Ordering::Relaxed)
    }
}
