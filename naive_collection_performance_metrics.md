# NaiveCollection Performance Metrics

This document tracks performance metrics for the `NaiveCollection` implementation under different database configurations.

## Test Configuration

- **Collection Type:** `AllAnyPostWithEditContextCollection`
- **Number of Posts:** 1000
- **Update Pattern:** Comprehensive stress test
  - Batch size: 1-20 posts per batch (variable)
  - Delay between batches: 10-100ms (variable)
- **Monitored Tables:** `PostsEditContext`, `TermRelationships`
- **Platform:** Desktop (macOS)
- **Measurement:** Rolling average over last 100 samples

## Metrics Tracked

- **Initial Load Time:** Time to load all 1000 posts on first access
- **Average Load Time:** Average time to reload collection from database
- **Min/Max Load Time:** Range of observed load times
- **Total Latency:** Time from observer trigger to StateFlow update (includes DB load + Kotlin mapping)
- **Sample Count:** Number of observer triggers measured

---

## In-Memory Database (SQLite `:memory:`)

**Test Date:** 2025-11-11
**Database Type:** In-memory (no disk I/O)

### Initial Load
```
⏱️ Initial load: 11ms (1000 posts)
```

### Steady State Performance (After Warmup)

| Metric | Value |
|--------|-------|
| **Average Load Time** | 12-22ms |
| **Min Load Time** | 2ms |
| **Max Load Time** | 55ms |
| **Average Total Latency** | 15-22ms |
| **Observer Overhead** | ~1-2ms |

### Performance Trend

**First 100 samples (warmup):**
- Sample 10: avg=17ms, min=11ms, max=23ms, latency=19ms
- Sample 20: avg=18ms, min=3ms, max=32ms, latency=19ms
- Sample 50: avg=16ms, min=3ms, max=32ms, latency=17ms
- Sample 100: avg=17ms, min=3ms, max=52ms, latency=17ms

**After warmup (stable):**
- Samples 100-200: avg=12-15ms, min=2ms, max=55ms, latency=12-15ms
- Samples 200-500: avg=13-22ms, min=2ms, max=55ms, latency=13-22ms

### Observations

✅ **Very fast** - Loading 1000 posts takes only 11-22ms
✅ **Consistent** - Performance remains stable over hundreds of updates
✅ **Low overhead** - Observer/coroutine system adds minimal latency (1-2ms)
✅ **No degradation** - No performance decline over time

### Conclusion

With in-memory database, the `NaiveCollection` handles 1000 posts efficiently with sub-25ms reload times. The "naive" approach (reload everything on each change) performs well when database I/O is not a bottleneck.

---

## Disk-Based Database (File System)

**Test Date:** TBD
**Database Type:** Temporary file on disk

### Results

_To be measured after switching to disk-based database configuration._

### Expected Changes

- **Slower I/O:** Disk reads will be significantly slower than memory
- **Cache Effects:** OS page cache may improve performance after warmup
- **Variance:** Higher variability due to disk I/O contention
- **Latency Impact:** Total latency will be dominated by DB load time

---

## Future: ManagedCollection Comparison

_Placeholder for comparing NaiveCollection vs ManagedCollection performance._

ManagedCollection would track items in memory and apply incremental updates, potentially offering:
- **Lower latency** - Only update changed items, not full reload
- **Higher memory usage** - Keeps collection in memory
- **More complexity** - Tracks adds/updates/deletes individually
