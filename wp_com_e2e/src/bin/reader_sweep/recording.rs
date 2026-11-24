//! Keeps what a sweep receives, so a run can be replayed later and a response that didn't parse, or
//! that holds something odd, can be looked at.
//!
//! A run's directory holds:
//!
//! - `responses/000001.json` and so on: the body of a response exactly as the server sent it. A run
//!   keeps all of them, or only the ones worth a look.
//! - `index.jsonl`: one line per request, written once its result is known.
//! - `anomalies.jsonl`: one line per value that parsed but isn't right.
//! - `summary.json` and `census.json`, written at the end.
//!
//! Request headers are never written, so the access token isn't either.

use crate::sweep_support::Captured;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

/// One line of `index.jsonl`.
#[derive(Debug, Serialize)]
pub struct IndexLine<'a> {
    pub stream: &'a str,
    /// Milliseconds from the start of the run to when this request's result was known.
    pub at_millis: u64,
    /// `None` when the request got no response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// The kept body's path within the run's directory. `None` when it wasn't kept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub millis: Option<u64>,
    /// `ok`, `parse_error`, `http_error` or `request_failed`.
    pub outcome: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub posts: Option<usize>,
    /// How many more posts the page had that the library couldn't read. Left out when none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreadable_posts: Option<usize>,
    /// For a stream of cards, how many of each kind the page had. Left out for other streams.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cards: Option<&'a BTreeMap<String, u64>>,
    /// How many things in it aren't right, counting those posts. Left out when none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub findings: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
}

/// Writes a run's files. Shared by every worker.
pub struct Recorder {
    dir: PathBuf,
    /// Numbers the run's responses: every worker's executor shares it.
    seq: Arc<AtomicU64>,
    index: Mutex<File>,
    anomalies: Mutex<File>,
}

impl Recorder {
    /// Creates the run's directory, which must not hold a run already: mixing two runs' files would
    /// make neither replayable.
    pub fn create(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir.join("responses"))?;
        let new_file = |name: &str| {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(name))
        };
        Ok(Self {
            dir: dir.to_path_buf(),
            seq: Arc::new(AtomicU64::new(0)),
            index: Mutex::new(new_file("index.jsonl")?),
            anomalies: Mutex::new(new_file("anomalies.jsonl")?),
        })
    }

    /// The counter that numbers the run's responses, for each worker's executor.
    pub fn sequence(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.seq)
    }

    /// Writes a response's body, and returns its path within the run's directory.
    pub fn keep_body(&self, captured: &Captured) -> io::Result<String> {
        let file = format!("responses/{:06}.json", captured.seq);
        fs::write(self.dir.join(&file), &captured.body)?;
        Ok(file)
    }

    pub fn write_index_line(&self, line: &IndexLine) -> io::Result<()> {
        append(&self.index, line)
    }

    pub fn write_anomaly(&self, line: &impl Serialize) -> io::Result<()> {
        append(&self.anomalies, line)
    }

    /// Writes one of the run's closing files, such as `summary.json`.
    pub fn write_json(&self, name: &str, value: &impl Serialize) -> io::Result<()> {
        let text = serde_json::to_string_pretty(value).map_err(io::Error::other)? + "\n";
        fs::write(self.dir.join(name), text)
    }
}

fn append(file: &Mutex<File>, line: &impl Serialize) -> io::Result<()> {
    let mut text = serde_json::to_string(line).map_err(io::Error::other)?;
    text.push('\n');
    file.lock()
        .expect("file lock poisoned")
        .write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sweep_support::RecordingExecutor;
    use async_trait::async_trait;
    use wp_api::prelude::*;
    use wp_api::request::endpoint::WpEndpointUrl;
    use wp_api::request::{RequestContext, WpMultipartFormRequest};

    struct Fixed {
        body: &'static str,
    }

    #[async_trait]
    impl RequestExecutor for Fixed {
        async fn execute(
            &self,
            _request: Arc<WpNetworkRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            Ok(WpNetworkResponse {
                body: self.body.as_bytes().to_vec(),
                status_code: 200,
                response_header_map: Arc::new(WpNetworkHeaderMap::default()),
                request_url: WpEndpointUrl("https://example.com/read/following?number=2".into()),
                request_method: RequestMethod::GET,
                request_header_map: Arc::new(WpNetworkHeaderMap::default()),
            })
        }

        async fn upload(
            &self,
            _request: Arc<WpMultipartFormRequest>,
        ) -> Result<WpNetworkResponse, RequestExecutionError> {
            unreachable!("the sweep uploads nothing")
        }

        async fn sleep(&self, _millis: u64) {}

        fn cancel(&self, _context: Arc<RequestContext>) {}
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("reader-sweep-{name}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn request() -> Arc<WpNetworkRequest> {
        Arc::new(WpNetworkRequest::get(WpEndpointUrl(
            "https://example.com/read/following?number=2".into(),
        )))
    }

    #[tokio::test]
    async fn a_response_is_held_until_asked_for_and_kept_as_the_server_sent_it() {
        let dir = temp_dir("bodies");
        let recorder = Arc::new(Recorder::create(&dir).unwrap());
        let body = "{\"posts\": [ ] }\n";
        let executor = RecordingExecutor::new(Arc::new(Fixed { body }), recorder.sequence());

        executor.execute(request()).await.unwrap();
        let first = executor.take_last().unwrap();
        executor.execute(request()).await.unwrap();
        let second = executor.take_last().unwrap();

        assert_eq!(first.seq, 1);
        assert_eq!(first.method, "GET");
        assert_eq!(first.url, "https://example.com/read/following?number=2");
        assert_eq!(first.status, 200);
        assert_eq!(first.body, body.as_bytes());
        assert_eq!(second.seq, 2);
        // Taken once: a later request that gets no response mustn't be paired with this one.
        assert_eq!(executor.take_last(), None);

        // Nothing is on disk until a response is kept.
        assert_eq!(fs::read_dir(dir.join("responses")).unwrap().count(), 0);
        let file = recorder.keep_body(&second).unwrap();
        assert_eq!(file, "responses/000002.json");
        assert_eq!(fs::read_to_string(dir.join(&file)).unwrap(), body);
    }

    #[test]
    fn a_directory_that_holds_a_run_is_refused() {
        let dir = temp_dir("twice");
        Recorder::create(&dir).unwrap();
        assert!(Recorder::create(&dir).is_err());
    }

    #[test]
    fn an_index_line_leaves_out_what_it_does_not_have() {
        let dir = temp_dir("index");
        let recorder = Recorder::create(&dir).unwrap();
        recorder
            .write_index_line(&IndexLine {
                stream: "site:1",
                at_millis: 1500,
                seq: None,
                file: None,
                method: None,
                url: None,
                status: None,
                bytes: None,
                millis: None,
                outcome: "request_failed",
                posts: None,
                unreadable_posts: None,
                cards: None,
                findings: None,
                error: Some("timed out"),
            })
            .unwrap();
        assert_eq!(
            fs::read_to_string(dir.join("index.jsonl")).unwrap(),
            "{\"stream\":\"site:1\",\"at_millis\":1500,\"outcome\":\"request_failed\",\"error\":\"timed out\"}\n"
        );
    }
}
