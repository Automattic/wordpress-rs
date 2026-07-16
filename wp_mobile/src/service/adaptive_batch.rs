use crate::collection::FetchError;
use std::future::Future;

/// Load items by ID, degrading the batch size on request timeouts.
///
/// `batch_sizes` lists the sizes to try, largest first (e.g. `[100, 20, 5]`).
/// IDs are split into chunks of the current size and passed to `load_chunk`.
/// When a chunk request times out (per [`FetchError::is_timeout`]) and a smaller
/// size is available, that chunk is requeued at the next size down and retried.
/// Items that still time out at the smallest size — or fail for any non-timeout
/// reason — are counted as failed. This lets sites that can't render a large
/// batch within the request timeout still sync in smaller pieces.
///
/// `load_chunk` performs the actual fetch for one chunk and returns the number
/// of items in that chunk that failed to load (e.g. requested but not returned).
/// It is responsible for any per-item state tracking, such as marking entities
/// `Failed`. `entity_label` names the items in log messages (e.g. `"posts"`).
///
/// Returns the total number of items that failed to load.
///
/// ## State Access
/// - None directly; all state is read/written by the caller's `load_chunk`.
pub(crate) async fn load_with_adaptive_batching<Id, F, Fut>(
    ids: &[Id],
    batch_sizes: &[usize],
    entity_label: &str,
    load_chunk: F,
) -> u32
where
    Id: Clone + std::fmt::Debug,
    F: Fn(Vec<Id>) -> Fut,
    Fut: Future<Output = Result<u32, FetchError>>,
{
    let mut failed_count: u32 = 0;
    // Work items: a set of IDs plus the index into `batch_sizes` of the size to
    // use for them. Used as a stack — a timed-out chunk is requeued at the next
    // smaller size and retried before the remaining same-size chunks.
    let mut pending: Vec<(Vec<Id>, usize)> = vec![(ids.to_vec(), 0)];

    while let Some((batch_ids, size_index)) = pending.pop() {
        let batch_size = batch_sizes[size_index];
        for chunk in batch_ids.chunks(batch_size) {
            match load_chunk(chunk.to_vec()).await {
                Ok(chunk_failed_count) => failed_count += chunk_failed_count,
                Err(e) => {
                    if e.is_timeout() && size_index + 1 < batch_sizes.len() {
                        log::debug!(
                            "Fetching {} {} by ID timed out; retrying with batch size {}",
                            chunk.len(),
                            entity_label,
                            batch_sizes[size_index + 1]
                        );
                        pending.push((chunk.to_vec(), size_index + 1));
                    } else {
                        log::warn!(
                            "Failed to load {} {} (IDs: {:?}): {}",
                            chunk.len(),
                            entity_label,
                            chunk,
                            e
                        );
                        failed_count += chunk.len() as u32;
                    }
                }
            }
        }
    }

    failed_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use wp_api::prelude::{RequestExecutionErrorReason, RequestMethod, WpApiError};

    const SIZES: &[usize] = &[100, 20, 5];

    fn timeout_error() -> FetchError {
        FetchError::Api(WpApiError::RequestExecutionFailed {
            status_code: None,
            redirects: None,
            reason: RequestExecutionErrorReason::HttpTimeoutError,
            request_url: "https://example.com".to_string(),
            request_method: RequestMethod::GET,
        })
    }

    fn non_timeout_error() -> FetchError {
        FetchError::Database {
            err_message: "boom".to_string(),
        }
    }

    /// With no failures the largest batch size is used and nothing degrades.
    #[tokio::test]
    async fn uses_largest_size_and_returns_zero_when_all_succeed() {
        let calls = RefCell::new(Vec::new());
        let ids: Vec<i64> = (1..=250).collect();

        let failed = load_with_adaptive_batching(&ids, SIZES, "posts", |chunk| {
            calls.borrow_mut().push(chunk.len());
            async move { Ok::<u32, FetchError>(0) }
        })
        .await;

        assert_eq!(failed, 0);
        assert_eq!(*calls.borrow(), vec![100, 100, 50]);
    }

    /// The per-chunk failed counts reported by `load_chunk` are summed.
    #[tokio::test]
    async fn accumulates_failed_counts_from_successful_loads() {
        let ids: Vec<i64> = (1..=10).collect();

        let failed = load_with_adaptive_batching(&ids, &[5], "posts", |_chunk| async move {
            Ok::<u32, FetchError>(2)
        })
        .await;

        // 10 ids / size 5 = 2 chunks, each reporting 2 failures.
        assert_eq!(failed, 4);
    }

    /// A timeout degrades the batch size until the chunk is small enough to
    /// succeed; every item loads with no failures.
    #[tokio::test]
    async fn degrades_batch_size_on_timeout() {
        let calls = RefCell::new(Vec::new());
        let ids: Vec<i64> = (1..=12).collect();

        let failed = load_with_adaptive_batching(&ids, SIZES, "posts", |chunk| {
            let len = chunk.len();
            calls.borrow_mut().push(len);
            async move { if len > 5 { Err(timeout_error()) } else { Ok(0) } }
        })
        .await;

        assert_eq!(failed, 0);
        // 12 @100 -> timeout, 12 @20 -> timeout, then [5, 5, 2] @5 all succeed.
        assert_eq!(*calls.borrow(), vec![12, 12, 5, 5, 2]);
    }

    /// Non-timeout errors are counted as failed immediately, without retrying at
    /// a smaller size.
    #[tokio::test]
    async fn non_timeout_errors_are_not_retried() {
        let calls = RefCell::new(Vec::new());
        let ids: Vec<i64> = (1..=12).collect();

        let failed = load_with_adaptive_batching(&ids, SIZES, "posts", |chunk| {
            calls.borrow_mut().push(chunk.len());
            async move { Err::<u32, FetchError>(non_timeout_error()) }
        })
        .await;

        assert_eq!(failed, 12);
        assert_eq!(*calls.borrow(), vec![12]);
    }

    /// Items that keep timing out at the smallest size are counted as failed.
    #[tokio::test]
    async fn marks_failed_when_smallest_batch_still_times_out() {
        let ids: Vec<i64> = (1..=12).collect();

        let failed = load_with_adaptive_batching(&ids, SIZES, "posts", |_chunk| async move {
            Err::<u32, FetchError>(timeout_error())
        })
        .await;

        assert_eq!(failed, 12);
    }

    /// A single persistently-failing ID only charges its smallest-size chunk as
    /// failed; the other chunks still load.
    #[tokio::test]
    async fn only_the_failing_chunk_is_charged_at_the_smallest_size() {
        let ids: Vec<i64> = (1..=12).collect();

        let failed = load_with_adaptive_batching(&ids, SIZES, "posts", |chunk| {
            let len = chunk.len();
            let has_poison = chunk.contains(&7);
            async move {
                if len > 5 || has_poison {
                    Err(timeout_error())
                } else {
                    Ok(0)
                }
            }
        })
        .await;

        // Degrades to size 5: [1..=5] ok, [6..=10] contains 7 -> timeout at the
        // smallest size -> all 5 charged, [11, 12] ok.
        assert_eq!(failed, 5);
    }
}
