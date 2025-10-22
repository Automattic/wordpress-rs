# Usage Examples

> **Last Updated:** 2025-10-21

Common usage patterns and examples for the wp_mobile_cache repository system.

## Table of Contents

- [Basic Operations](#basic-operations)
- [Batch Operations](#batch-operations)
- [Upsert with Terms](#upsert-with-terms)
- [Transaction Management](#transaction-management)
- [Multi-Site Operations](#multi-site-operations)
- [Cache Freshness](#cache-freshness)
- [Error Handling](#error-handling)

## Basic Operations

### Simple Insert and Query

```rust
use wp_mobile_cache::{WpApiCache, DbSite, RowId};
use wp_mobile_cache::repository::{PostRepository, Repository};
use wp_api::posts::AnyPostWithEditContext;

// Open database
let cache = WpApiCache::new("cache.db")?;
cache.perform_migrations()?;
let conn = cache.connection();

// Create repository and site
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Create a post
let post = AnyPostWithEditContext {
    id: PostId(123),
    title: PostTitleWithEditContext { raw: "Hello World".into() },
    content: PostContentWithEditContext { raw: "Post content".into() },
    author: UserId(1),
    // ... other fields
};

// Insert
let rowid = repo.insert(&conn, &post, &site)?;
println!("Inserted with rowid: {}", rowid.0);

// Query by rowid
let db_post = repo.select_by_rowid(&conn, &site, rowid)?;
assert_eq!(db_post.post.id, PostId(123));
assert_eq!(db_post.row_id, rowid);

// Query by WordPress post ID
let db_post = repo.select_by_post_id(&conn, &site, PostId(123))?;
println!("Title: {}", db_post.post.title.raw);
println!("Cached at: {}", db_post.last_fetched_at);
```

### Query All Posts

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Get all posts for site
let all_posts = repo.select_all(&conn, &site)?;
println!("Total posts: {}", all_posts.len());

for db_post in all_posts {
    println!("- {} (ID: {}, cached: {})",
        db_post.post.title.raw,
        db_post.post.id.0,
        db_post.last_fetched_at
    );
}
```

### Query by Author

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Get posts by specific author
let author_posts = repo.select_by_author(&conn, &site, UserId(1))?;
println!("Author has {} posts", author_posts.len());
```

### Query by Status

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Get drafts
let drafts = repo.select_by_status(&conn, &site, "draft")?;
println!("Draft posts: {}", drafts.len());

// Get published posts
let published = repo.select_by_status(&conn, &site, "publish")?;
println!("Published posts: {}", published.len());
```

### Delete Post

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Delete by WordPress post ID
let deleted = repo.delete_by_post_id(&conn, &site, PostId(123))?;
if deleted > 0 {
    println!("Post deleted");
} else {
    println!("Post not found");
}
```

### Count Posts

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

let total = repo.count(&conn, &site)?;
println!("Total posts in cache: {}", total);
```

## Batch Operations

### Insert Multiple Posts

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

let posts = vec![
    create_post(PostId(1), "First Post"),
    create_post(PostId(2), "Second Post"),
    create_post(PostId(3), "Third Post"),
];

// Batch insert (atomic transaction)
let mut conn = Connection::open("cache.db")?;
let rowids = repo.insert_batch(&mut conn, &posts, &site)?;

println!("Inserted {} posts", rowids.len());
for (post, rowid) in posts.iter().zip(rowids.iter()) {
    println!("- {} => rowid {}", post.title.raw, rowid.0);
}
```

### Sync Posts from API

```rust
use wp_api::WpApiClient;

// Fetch posts from WordPress API
let api_client = WpApiClient::new("https://example.com")?;
let api_posts = api_client.get_posts().await?;

// Upsert all posts (insert new, update existing)
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

for post in api_posts {
    let rowid = repo.upsert(&conn, &site, &post)?;
    println!("Upserted post {} => rowid {}", post.id.0, rowid.0);
}
```

## Upsert with Terms

### Basic Upsert with Categories and Tags

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Create post with terms
let mut post = AnyPostWithEditContext {
    id: PostId(123),
    title: PostTitleWithEditContext { raw: "Hello".into() },
    categories: Some(vec![TermId(1), TermId(2)]),
    tags: Some(vec![TermId(10), TermId(20), TermId(30)]),
    // ... other fields
};

// Upsert post with terms (atomic transaction)
let mut conn = Connection::open("cache.db")?;
let post_rowid = repo.upsert_with_terms(&mut conn, &site, &post)?;
println!("Upserted post with rowid: {}", post_rowid.0);

// Reading automatically includes terms
let db_post = repo.select_by_rowid(&conn, &site, post_rowid)?;
assert_eq!(db_post.post.categories, Some(vec![TermId(1), TermId(2)]));
assert_eq!(db_post.post.tags, Some(vec![TermId(10), TermId(20), TermId(30)]));
```

### Update with Different Terms

```rust
// Update with different terms
post.categories = Some(vec![TermId(1), TermId(3)]);  // Removed 2, added 3
post.tags = Some(vec![TermId(10)]);                   // Removed 20, 30

repo.upsert_with_terms(&mut conn, &site, &post)?;

// Observer sees only changes:
// - DELETE for term 2 (category)
// - INSERT for term 3 (category)
// - DELETE for terms 20, 30 (tags)
// - No events for terms 1, 10 (unchanged)
```

### Manual Term Management

```rust
use wp_mobile_cache::repository::TermRelationshipRepository;

let post_repo = PostRepository;
let term_repo = TermRelationshipRepository;
let site = DbSite { row_id: RowId(1) };

// Upsert post without terms
let post_rowid = post_repo.upsert(&conn, &site, &post)?;

// Manually sync terms
term_repo.sync_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::Category,
    &[TermId(1), TermId(2), TermId(3)],
)?;

term_repo.sync_terms_for_object(
    &conn,
    &site,
    post_rowid,
    &TaxonomyType::PostTag,
    &[TermId(10), TermId(20)],
)?;
```

### Custom Taxonomies

```rust
let term_repo = TermRelationshipRepository;
let product_post_rowid = RowId(100);

// Sync product categories (custom taxonomy)
term_repo.sync_terms_for_object(
    &conn,
    &site,
    product_post_rowid,
    &TaxonomyType::Custom("product_category".into()),
    &[TermId(50), TermId(51), TermId(52)],
)?;

// Query product categories
let product_categories = term_repo.get_terms_for_object(
    &conn,
    &site,
    product_post_rowid,
    &TaxonomyType::Custom("product_category".into()),
)?;
```

## Transaction Management

### Manual Transaction Control

```rust
let repo = PostRepository;
let site = DbSite { row_id: RowId(1) };

// Start transaction
let tx = conn.transaction()?;

// Multiple operations in transaction
let rowid1 = repo.insert(&tx, &post1, &site)?;
let rowid2 = repo.insert(&tx, &post2, &site)?;
repo.delete_by_post_id(&tx, &site, PostId(999))?;

// Commit all or rollback all
tx.commit()?;
println!("All operations succeeded");
```

### Error Handling with Rollback

```rust
let tx = conn.transaction()?;

let result = (|| -> Result<(), SqliteDbError> {
    repo.insert(&tx, &post1, &site)?;
    repo.insert(&tx, &post2, &site)?;

    // Simulate error
    if some_condition {
        return Err(SqliteDbError::Query("Validation failed".into()));
    }

    Ok(())
})();

match result {
    Ok(()) => {
        tx.commit()?;
        println!("Transaction committed");
    }
    Err(e) => {
        // Transaction automatically rolled back on drop
        eprintln!("Transaction failed: {}", e);
    }
}
```

### Generic Functions with QueryExecutor

```rust
fn sync_posts<E: QueryExecutor>(
    executor: &E,
    site: &DbSite,
    posts: Vec<AnyPostWithEditContext>,
) -> Result<()> {
    let repo = PostRepository;

    for post in posts {
        repo.upsert(executor, site, &post)?;
    }

    Ok(())
}

// Works with Connection
sync_posts(&conn, &site, posts.clone())?;

// Also works with Transaction
let tx = conn.transaction()?;
sync_posts(&tx, &site, posts)?;
tx.commit()?;
```

## Multi-Site Operations

### Managing Multiple Sites

```rust
// Site 1
let site1 = DbSite { row_id: RowId(1) };
repo.insert(&conn, &post1, &site1)?;

// Site 2
let site2 = DbSite { row_id: RowId(2) };
repo.insert(&conn, &post2, &site2)?;

// Queries are scoped to site
let site1_posts = repo.select_all(&conn, &site1)?;
let site2_posts = repo.select_all(&conn, &site2)?;

// Completely isolated
assert_ne!(site1_posts.len(), site2_posts.len());
```

### Same Post ID on Different Sites

```rust
let site1 = DbSite { row_id: RowId(1) };
let site2 = DbSite { row_id: RowId(2) };

// Same WordPress post ID on different sites
let post_site1 = AnyPostWithEditContext {
    id: PostId(123),
    title: PostTitleWithEditContext { raw: "Site 1 Post".into() },
    // ...
};

let post_site2 = AnyPostWithEditContext {
    id: PostId(123),
    title: PostTitleWithEditContext { raw: "Site 2 Post".into() },
    // ...
};

// Both succeed - different sites
repo.insert(&conn, &post_site1, &site1)?;
repo.insert(&conn, &post_site2, &site2)?;

// Query by site + post ID
let db_post1 = repo.select_by_post_id(&conn, &site1, PostId(123))?;
let db_post2 = repo.select_by_post_id(&conn, &site2, PostId(123))?;

assert_eq!(db_post1.post.title.raw, "Site 1 Post");
assert_eq!(db_post2.post.title.raw, "Site 2 Post");
```

### Site Deletion (Cascade)

```rust
// Delete site (cascades to all posts and term relationships)
conn.execute("DELETE FROM sites WHERE id = ?", params![site.row_id.0])?;

// All posts for that site are automatically deleted
let posts = repo.select_all(&conn, &site)?;
assert!(posts.is_empty());
```

## Cache Freshness

### Check Cache Staleness

```rust
use chrono::{DateTime, Utc, Duration};

fn is_stale(db_post: &DbAnyPostWithEditContext, max_age: Duration) -> Result<bool> {
    let fetched_at = DateTime::parse_from_rfc3339(&db_post.last_fetched_at)?;
    let age = Utc::now() - fetched_at;
    Ok(age > max_age)
}

// Usage
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;

if is_stale(&cached_post, Duration::hours(1))? {
    // Refresh from WordPress API
    let fresh_post = api_client.get_post(post_id).await?;
    repo.upsert(&conn, &site, &fresh_post)?;
}
```

### Display Cache Age in UI

```rust
fn humanize_timestamp(iso_timestamp: &str) -> Result<String> {
    let dt = DateTime::parse_from_rfc3339(iso_timestamp)?;
    let now = Utc::now();
    let duration = now - dt;

    if duration.num_seconds() < 60 {
        Ok("just now".to_string())
    } else if duration.num_minutes() < 60 {
        Ok(format!("{} minutes ago", duration.num_minutes()))
    } else if duration.num_hours() < 24 {
        Ok(format!("{} hours ago", duration.num_hours()))
    } else {
        Ok(format!("{} days ago", duration.num_days()))
    }
}

// UI display
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;
let age_text = humanize_timestamp(&cached_post.last_fetched_at)?;

if !network_available() {
    show_banner(&format!("Showing cached data from {}", age_text));
}
```

### Selective Sync (Stale Posts Only)

```rust
impl PostRepository {
    pub fn select_posts_older_than(
        &self,
        executor: &impl QueryExecutor,
        site: &DbSite,
        cutoff: DateTime<Utc>,
    ) -> Result<Vec<DbAnyPostWithEditContext>> {
        let cutoff_str = cutoff.to_rfc3339();
        let sql = r#"
            SELECT * FROM posts_edit_context
            WHERE db_site_id = ? AND last_fetched_at < ?
            ORDER BY last_fetched_at ASC
        "#;
        // Implementation...
    }
}

// Sync only stale posts
let cutoff = Utc::now() - Duration::hours(24);
let stale_posts = repo.select_posts_older_than(&conn, &site, cutoff)?;

for db_post in stale_posts {
    let fresh = api_client.get_post(db_post.post.id).await?;
    repo.upsert(&conn, &site, &fresh)?;
}
```

## Error Handling

### Pattern Matching Errors

```rust
match repo.select_by_post_id(&conn, &site, post_id) {
    Ok(db_post) => {
        println!("Found: {}", db_post.post.title.raw);
    }
    Err(SqliteDbError::NotFound) => {
        println!("Post not found in cache");
        // Fetch from API?
    }
    Err(SqliteDbError::ForeignKeyViolation) => {
        eprintln!("Invalid site reference");
    }
    Err(e) => {
        eprintln!("Database error: {}", e);
    }
}
```

### Graceful Fallback

```rust
fn get_post_with_fallback(
    repo: &PostRepository,
    conn: &Connection,
    site: &DbSite,
    post_id: PostId,
    api_client: &WpApiClient,
) -> Result<AnyPostWithEditContext> {
    // Try cache first
    match repo.select_by_post_id(conn, site, post_id) {
        Ok(db_post) => {
            println!("Cache hit");
            Ok(db_post.post)
        }
        Err(SqliteDbError::NotFound) => {
            println!("Cache miss - fetching from API");
            let post = api_client.get_post(post_id).await?;

            // Cache for next time
            repo.upsert(conn, site, &post)?;

            Ok(post)
        }
        Err(e) => Err(e.into()),
    }
}
```

### Transaction Error Handling

```rust
fn bulk_upsert_with_validation(
    posts: Vec<AnyPostWithEditContext>,
) -> Result<Vec<RowId>> {
    let mut conn = Connection::open("cache.db")?;
    let site = DbSite { row_id: RowId(1) };
    let repo = PostRepository;

    let tx = conn.transaction()?;
    let mut rowids = Vec::new();

    for post in posts {
        // Validate before upserting
        if post.title.raw.is_empty() {
            // Rollback happens on drop
            return Err(SqliteDbError::Query("Empty title".into()));
        }

        rowids.push(repo.upsert(&tx, &site, &post)?);
    }

    tx.commit()?;
    Ok(rowids)
}
```

## Helper Functions

### Create Test Post

```rust
fn create_post(id: PostId, title: &str) -> AnyPostWithEditContext {
    AnyPostWithEditContext {
        id,
        title: PostTitleWithEditContext { raw: title.to_string() },
        content: PostContentWithEditContext { raw: "Content".to_string() },
        author: UserId(1),
        status: Some("publish".to_string()),
        categories: None,
        tags: None,
        // ... other fields with defaults
    }
}
```

### Database Setup

```rust
fn setup_cache() -> Result<(WpApiCache, DbSite)> {
    let cache = WpApiCache::new(":memory:")?;  // In-memory for tests
    cache.perform_migrations()?;

    // Create site
    let conn = cache.connection();
    conn.execute("INSERT INTO sites DEFAULT VALUES", [])?;
    let site = DbSite { row_id: RowId(conn.last_insert_rowid()) };

    Ok((cache, site))
}
```

## See Also

- [PostRepository API](repositories/post-repository.md)
- [TermRelationshipRepository API](repositories/term-relationship-repository.md)
- [Migration Guide](migration-guide.md)
- [Design Decisions](design-decisions/)
