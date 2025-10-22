# Design Decision 10: Cache Freshness Tracking with `last_fetched_at`

> **Last Updated:** 2025-10-21

## Decision

Add `last_fetched_at` timestamp field to cached data (posts) to track when data was fetched from the WordPress API.

## Context

When caching WordPress data, we need to track:
- When was this data last fetched from the API?
- Is the cached data stale?
- Should we refresh from the server?

## Rationale

### Cache Staleness Detection

**Enables logic to determine when cached data should be refreshed:**

```rust
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;

// Check if cache is stale
let age = calculate_age(&cached_post.last_fetched_at)?;
if age > Duration::from_hours(1) {
    // Refresh from WordPress API
    let fresh_post = api_client.get_post(post_id).await?;
    repo.upsert(&conn, &site, &fresh_post)?;
}
```

**Use cases:**
- Automatic refresh on app startup if data is old
- Background sync only for stale data
- Skip API calls for recently cached data
- Intelligent cache invalidation

### Offline UX

**Display "Last updated X time ago" in UI when network unavailable:**

```rust
// Pull-to-refresh header
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;

if !network_available() {
    let age = humanize_timestamp(&cached_post.last_fetched_at);
    show_banner(&format!("Showing cached data from {}", age));
    // "Showing cached data from 5 minutes ago"
    // "Showing cached data from 2 hours ago"
}
```

**UI patterns:**
- Pull-to-refresh header text
- Cache age indicator in post list
- "Last synced" timestamp in settings
- Offline mode indicators

### Sync Foundation

**Critical metadata for implementing intelligent sync strategies:**

```rust
// Sync only stale posts
let stale_posts = repo.select_posts_older_than(
    &conn,
    &site,
    Duration::from_hours(24),
)?;

for post in stale_posts {
    let fresh = api_client.get_post(post.post.id).await?;
    repo.upsert(&conn, &site, &fresh)?;
}
```

**Sync strategies enabled:**
- Incremental sync (only update old data)
- Priority sync (most stale first)
- Delta sync (compare modified dates)
- Conditional requests (If-Modified-Since headers)

### Debug/Monitoring

**Helps diagnose cache issues and data freshness problems:**

```rust
// Debugging cache issues
let posts = repo.select_all(&conn, &site)?;
for post in posts {
    println!(
        "Post {}: last_fetched_at={}",
        post.post.id,
        post.last_fetched_at
    );
}

// Find oldest cached post
let oldest = posts.iter()
    .min_by_key(|p| &p.last_fetched_at)
    .unwrap();
println!("Oldest cache: {}", oldest.last_fetched_at);
```

**Diagnostic use cases:**
- Cache analytics (average age, stalest data)
- Sync verification (did sync actually update?)
- Performance monitoring (cache hit rate over time)
- Bug investigation (was data fresh when error occurred?)

## Implementation

### Database Schema

**Field added to `posts_edit_context` table:**

```sql
CREATE TABLE `posts_edit_context` (
  `rowid` INTEGER PRIMARY KEY AUTOINCREMENT,
  `db_site_id` INTEGER NOT NULL,
  `id` INTEGER NOT NULL,
  -- ... other post fields ...
  `last_fetched_at` TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  FOREIGN KEY (db_site_id) REFERENCES sites(id) ON DELETE CASCADE
) STRICT;
```

**Why only in `posts_edit_context`?**

- `sites` table is just an internal identifier
- Actual site data will have its own timestamps when site tables are added
- Each entity table (`posts`, `pages`, `users`) gets its own `last_fetched_at`

### Automatic Timestamp Behavior

**SQLite manages the timestamp automatically:**

**INSERT** - Default sets current UTC time:
```sql
INSERT INTO posts_edit_context (db_site_id, id, title, ...)
VALUES (1, 123, 'Hello', ...);
-- last_fetched_at automatically set to current time
```

**UPDATE (via upsert)** - Explicitly set in ON CONFLICT clause:
```sql
INSERT INTO posts_edit_context (db_site_id, id, title, ...)
VALUES (1, 123, 'Hello World', ...)
ON CONFLICT(db_site_id, id) DO UPDATE SET
    title = excluded.title,
    -- ... other fields ...
    last_fetched_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now');
-- last_fetched_at updated to current time
```

**Format:** ISO 8601 UTC with milliseconds: `2025-10-21T19:49:22.667Z`

**Benefits:**
- ✅ Developers never pass timestamps manually
- ✅ Always accurate (reflects actual cache time)
- ✅ Consistent format across all cached data
- ✅ No timezone confusion (always UTC)

### Type Design

```rust
pub struct DbAnyPostWithEditContext {
    pub row_id: RowId,
    pub site: DbSite,
    pub post: AnyPostWithEditContext,
    pub last_fetched_at: String,  // ISO 8601 UTC: "2025-10-21T19:49:22.667Z"
}
```

**Why String?**

- Simple to store in SQLite (TEXT column)
- Easy to display in UI (already formatted)
- Standard format (ISO 8601)
- Can parse with `chrono` when needed

## Example Usage

### Checking Freshness

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
    // Refresh from API
}
```

### UI Display

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

// Usage
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;
let age_text = humanize_timestamp(&cached_post.last_fetched_at)?;
header.subtitle = format!("Last updated {}", age_text);
// "Last updated 5 minutes ago"
```

### Selective Sync

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
        // Returns posts ordered by stalest first
    }
}

// Sync stale posts
let cutoff = Utc::now() - Duration::hours(24);
let stale_posts = repo.select_posts_older_than(&conn, &site, cutoff)?;

for db_post in stale_posts {
    let fresh = api_client.get_post(db_post.post.id).await?;
    repo.upsert(&conn, &site, &fresh)?;
}
```

### Conditional Requests

```rust
// Use If-Modified-Since header based on cache time
let cached_post = repo.select_by_post_id(&conn, &site, post_id)?;

let response = api_client
    .get_post(post_id)
    .if_modified_since(&cached_post.last_fetched_at)
    .await?;

match response.status() {
    StatusCode::NOT_MODIFIED => {
        // Cache is still fresh, update timestamp
        repo.upsert(&conn, &site, &cached_post.post)?;
    }
    StatusCode::OK => {
        // Got new data, cache it
        let fresh_post = response.json()?;
        repo.upsert(&conn, &site, &fresh_post)?;
    }
    _ => {
        // Handle error
    }
}
```

## Alternatives Considered

### Alternative 1: No Timestamp

**Why rejected:**
- ❌ Cannot determine cache staleness
- ❌ No way to implement intelligent sync
- ❌ Poor offline UX (can't show "last updated")
- ❌ Harder to debug cache issues

### Alternative 2: Use WordPress `modified` Field

```rust
// Use post.modified instead of separate timestamp
let post_modified = DateTime::parse_from_rfc3339(&post.modified)?;
if Utc::now() - post_modified > Duration::hours(1) {
    // Refresh?
}
```

**Why rejected:**
- ❌ `modified` is when post was edited, not when we cached it
- ❌ Old post could be recently cached (modified=1 year ago, cached=5 min ago)
- ❌ Confuses content modification with cache time
- ❌ Still need to know when we last checked the API

### Alternative 3: Separate Cache Metadata Table

```sql
CREATE TABLE cache_metadata (
  table_name TEXT,
  row_id INTEGER,
  last_fetched_at TEXT,
  PRIMARY KEY (table_name, row_id)
);
```

**Why rejected:**
- ❌ Requires JOIN for every query
- ❌ More complex schema
- ❌ Two tables to update on each upsert
- ❌ Harder to enforce consistency
- ❌ No significant benefit

### Alternative 4: Timestamp Per Site (Not Per Post)

```sql
CREATE TABLE sites (
  id INTEGER PRIMARY KEY,
  last_synced_at TEXT  -- When all posts were synced
);
```

**Why rejected:**
- ❌ Too coarse-grained (individual posts may be newer)
- ❌ Cannot do selective sync (only stale posts)
- ❌ All-or-nothing sync (inefficient)
- ❌ Cannot track individual post freshness

## Trade-offs

### Advantages

✅ **Cache intelligence** - Know when data is stale
✅ **Offline UX** - Display cache age to users
✅ **Sync optimization** - Only refresh old data
✅ **Debugging** - Diagnose cache issues
✅ **Zero boilerplate** - SQLite manages timestamps automatically
✅ **Always accurate** - Timestamp reflects actual cache time
✅ **Standard format** - ISO 8601 UTC

### Disadvantages

❌ **Extra field** - Adds one TEXT column per entity table
❌ **Slight overhead** - UPDATE must set timestamp explicitly
❌ **String parsing** - Need chrono to work with timestamps

**Mitigation:**
- One TEXT field is minimal overhead
- Automatic timestamp generation is cheap
- chrono is already a dependency
- Benefits far outweigh costs

## Timestamp Format Details

### ISO 8601 with Milliseconds

```
2025-10-21T19:49:22.667Z
^         ^        ^   ^
Date      Time     ms  UTC
```

**Components:**
- `2025-10-21` - Date (YYYY-MM-DD)
- `T` - Separator
- `19:49:22.667` - Time with milliseconds
- `Z` - UTC timezone (Zulu)

### SQLite Function

```sql
strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
```

**Format specifiers:**
- `%Y` - Year (4 digits)
- `%m` - Month (01-12)
- `%d` - Day (01-31)
- `%H` - Hour (00-23)
- `%M` - Minute (00-59)
- `%f` - Second with milliseconds (00.000-59.999)
- `'now'` - Current UTC time

### Parsing with chrono

```rust
use chrono::{DateTime, Utc};

let timestamp = "2025-10-21T19:49:22.667Z";
let dt = DateTime::parse_from_rfc3339(timestamp)?;
let utc_dt: DateTime<Utc> = dt.into();
```

## Why UTC, Not Local Time?

**Always use UTC for cache timestamps:**

✅ **Unambiguous** - No timezone conversion confusion
✅ **Sortable** - Lexicographic order matches chronological order
✅ **Portable** - Works across timezones
✅ **Standard** - ISO 8601 best practice

**Convert to local time only for display:**

```rust
use chrono::Local;

let dt = DateTime::parse_from_rfc3339(&cached_post.last_fetched_at)?;
let local_dt = dt.with_timezone(&Local);
println!("Cached at: {}", local_dt.format("%Y-%m-%d %H:%M:%S %Z"));
// "Cached at: 2025-10-21 12:49:22 PDT"
```

## Future Enhancements

### Planned Additions

1. **Cache validation queries:**
   ```rust
   pub fn count_stale_posts(&self, cutoff: Duration) -> Result<usize>;
   pub fn get_cache_statistics(&self) -> Result<CacheStats>;
   ```

2. **Automatic cache expiration:**
   ```sql
   DELETE FROM posts_edit_context
   WHERE last_fetched_at < datetime('now', '-30 days');
   ```

3. **Sync strategy selection:**
   ```rust
   pub enum SyncStrategy {
       All,
       StalerThan(Duration),
       OldestFirst(usize),  // N oldest posts
   }
   ```

## Related Decisions

- [Entity vs Wrapper Types](05-entity-vs-wrapper.md) - Why timestamp in wrapper
- [UPSERT Pattern](06-upsert-pattern.md) - Automatic timestamp update
- [Database Schema](../architecture/database-schema.md) - Schema definition

## References

- [ISO 8601](https://en.wikipedia.org/wiki/ISO_8601)
- [SQLite Date And Time Functions](https://www.sqlite.org/lang_datefunc.html)
- [chrono Documentation](https://docs.rs/chrono/)

## See Also

- [Type System](../architecture/type-system.md) - DbAnyPostWithEditContext type
- [Usage Examples](../usage-examples.md) - Cache freshness patterns
