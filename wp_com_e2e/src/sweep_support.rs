//! What the sweep binaries share: an executor that holds on to each response, and the check that
//! a run's directory is outside any Git checkout.
//!
//! Each binary includes this file with `#[path]`, since `wp_com_e2e` has no library target.

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use wp_api::prelude::*;
use wp_api::request::{RequestContext, WpMultipartFormRequest};

/// A response a sweep received, held until it's known whether it's worth keeping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Captured {
    pub seq: u64,
    pub method: String,
    pub url: String,
    pub status: u32,
    pub millis: u64,
    pub body: Vec<u8>,
}

/// A request executor that holds on to each response before handing it back.
///
/// Each worker has its own, so [`Self::take_last`] is the response to that worker's last request.
/// Request headers are never held, so the access token isn't either.
pub struct RecordingExecutor {
    inner: Arc<dyn RequestExecutor>,
    /// Numbers the responses of a run, shared by every worker's executor.
    seq: Arc<AtomicU64>,
    last: Mutex<Option<Captured>>,
}

impl RecordingExecutor {
    pub fn new(inner: Arc<dyn RequestExecutor>, seq: Arc<AtomicU64>) -> Self {
        Self {
            inner,
            seq,
            last: Mutex::new(None),
        }
    }

    /// The response received since the last call, if the last request got one.
    pub fn take_last(&self) -> Option<Captured> {
        self.last.lock().expect("last lock poisoned").take()
    }
}

#[async_trait]
impl RequestExecutor for RecordingExecutor {
    async fn execute(
        &self,
        request: Arc<WpNetworkRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        let started = Instant::now();
        let result = self.inner.execute(request).await;
        if let Ok(response) = &result {
            *self.last.lock().expect("last lock poisoned") = Some(Captured {
                seq: self.seq.fetch_add(1, Ordering::SeqCst) + 1,
                method: format!("{:?}", response.request_method),
                url: response.request_url.0.clone(),
                status: response.status_code,
                millis: started.elapsed().as_millis() as u64,
                body: response.body.clone(),
            });
        }
        result
    }

    async fn upload(
        &self,
        request: Arc<WpMultipartFormRequest>,
    ) -> Result<WpNetworkResponse, RequestExecutionError> {
        self.inner.upload(request).await
    }

    async fn sleep(&self, millis: u64) {
        self.inner.sleep(millis).await
    }

    fn cancel(&self, context: Arc<RequestContext>) {
        self.inner.cancel(context)
    }
}

/// Refuses an output directory inside a Git checkout.
///
/// A run holds a real person's data, some of it private, and it mustn't end up committed.
pub fn check_output_dir(dir: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let mut ancestor = Some(absolute.as_path());
    while let Some(path) = ancestor {
        // A worktree has a `.git` file rather than a directory.
        if path.join(".git").exists() {
            return Err(format!(
                "{} is inside the Git checkout at {}. A run holds a real person's data, some of \
                 it private, so choose a directory outside any checkout.",
                absolute.display(),
                path.display()
            ));
        }
        ancestor = path.parent();
    }
    Ok(absolute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_output_directory_inside_a_checkout_is_refused() {
        let inside = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/sweep");
        assert!(
            check_output_dir(&inside)
                .unwrap_err()
                .contains("Git checkout")
        );
        let outside = std::env::temp_dir().join("sweep-outside");
        assert_eq!(
            check_output_dir(&outside),
            Ok(std::path::absolute(&outside).unwrap())
        );
    }
}
