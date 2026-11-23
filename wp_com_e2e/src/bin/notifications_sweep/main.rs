//! Walks an account's whole notification history with the library's client, and checks that
//! every note can be read and shown.
//!
//! ```bash
//! # Keep every response, so the run can be looked at again
//! WP_COM_API_KEY=… cargo run -p wp_com_e2e --bin notifications_sweep -- \
//!     --out ~/notifications-sweep/first
//!
//! # Write only a report, with no note content in it, to send to someone else
//! WP_COM_API_KEY=… cargo run -p wp_com_e2e --bin notifications_sweep -- \
//!     --out ~/notifications-sweep/report --report-only
//! ```
//!
//! The sweep pages from the newest note to the oldest, one request at a time with a pause
//! between them. Every request is a GET: nothing is marked read or seen. Each page is asked for as
//! whole notes, with their body and note context, and as IDs only; `walk.rs` says how that tells
//! the sweep it has reached the end.
//!
//! For each note it checks that it parsed, that every text in it becomes styled runs, and that it
//! gets a layout. It counts note types, the kinds of blocks, ranges, media and actions, values
//! that arrived as `Other`, notes the library couldn't read, and notes the API listed by ID but
//! didn't send whole. A panic is a finding.
//!
//! The access token comes from `WP_COM_API_KEY`, or from `wp_com_test_credentials.json`.
//!
//! The run's directory gets `report.json`. Without `--report-only` it also gets `responses/`, the
//! body of every response as the server sent it, and `index.jsonl`, one line per request. Those
//! are a real person's notifications, so the directory can't be inside a Git checkout. The report
//! holds only counts, note type names, the kind names the library knows, key paths and kinds of
//! error, so it can be sent on.
//!
//! It exits with `1` if anything couldn't be read, a request failed, or the walk stopped before
//! the end, and with `2` if it couldn't start. It stops at the first failed request: a `429`
//! means WordPress.com asked it to slow down.

// The trials use the parts of it the sweep doesn't.
#[allow(dead_code)]
#[path = "../../notification_checks.rs"]
mod notification_checks;
#[path = "../../sweep_support.rs"]
mod sweep_support;
mod tally;
mod walk;

use async_trait::async_trait;
use clap::Parser;
use integration_test_credentials::WpComTestCredentials;
use notification_checks::{check_note, without_values};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use sweep_support::{Captured, RecordingExecutor, check_output_dir};
use tally::{END_REACHED, Report};
use walk::{Page, Step, Walk};
use wp_api::prelude::*;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::notifications::{NotificationId, NotificationsParams, NotificationsQuery};

#[derive(Parser, Debug)]
#[command(about = "Walk an account's whole notification history and check every note")]
struct Args {
    /// Directory for the run. It can't be inside a Git checkout, and must be new or empty.
    #[arg(long)]
    out: PathBuf,

    /// Write only `report.json`, with no note content in it: no responses and no index.
    #[arg(long)]
    report_only: bool,

    /// Notes per page: 200 at most.
    #[arg(long, default_value_t = 100)]
    page_size: u32,

    /// Milliseconds to wait after each request.
    #[arg(long, default_value_t = 1000)]
    pause_ms: u64,

    /// The most requests to send. The walk stops there, and the run exits with `1`.
    #[arg(long, default_value_t = 1000)]
    max_requests: u64,
}

#[derive(Debug)]
struct NoNotifier;

#[async_trait]
impl WpAppNotifier for NoNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {}
}

#[tokio::main]
async fn main() -> ExitCode {
    // A panic's message can quote a note, so only where it happened is printed.
    std::panic::set_hook(Box::new(|info| {
        let place = info
            .location()
            .map_or("an unknown place".to_string(), |at| at.to_string());
        eprintln!("panicked at {place}");
    }));
    match run(Args::parse()).await {
        Ok(report) if report.all_read() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

/// One line of `index.jsonl`.
#[derive(Serialize)]
struct IndexLine<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    /// `notes` or `ids`.
    request: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    millis: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unreadable: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Where a normal run keeps its responses. A report-only run has none.
struct Keeper {
    dir: PathBuf,
    index: File,
}

impl Keeper {
    fn create(dir: &Path) -> io::Result<Self> {
        fs::create_dir_all(dir.join("responses"))?;
        let index = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join("index.jsonl"))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            index,
        })
    }

    fn keep(&mut self, mut line: IndexLine, captured: Option<&Captured>) -> io::Result<()> {
        if let Some(captured) = captured {
            let file = format!("responses/{:06}.json", captured.seq);
            fs::write(self.dir.join(&file), &captured.body)?;
            line.file = Some(file);
        }
        let mut text = serde_json::to_string(&line).map_err(io::Error::other)?;
        text.push('\n');
        self.index.write_all(text.as_bytes())
    }
}

async fn run(args: Args) -> Result<Report, String> {
    let out = check_output_dir(&args.out)?;
    fs::create_dir_all(&out).map_err(|err| format!("{}: {err}", out.display()))?;
    let empty = fs::read_dir(&out)
        .map_err(|err| format!("{}: {err}", out.display()))?
        .next()
        .is_none();
    if !empty {
        return Err(format!(
            "{} isn't empty: a run's directory must be new",
            out.display()
        ));
    }
    let token = access_token()?;
    let mut keeper = match args.report_only {
        true => None,
        false => Some(Keeper::create(&out).map_err(|err| format!("{}: {err}", out.display()))?),
    };

    let executor = Arc::new(RecordingExecutor::new(
        Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
        Arc::new(AtomicU64::new(0)),
    ));
    let client = WpComApiClient::new(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::static_with_auth(WpAuthentication::Bearer {
            token,
        })
        .into(),
        request_executor: executor.clone(),
        middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
        app_notifier: Arc::new(NoNotifier),
        // No language: a request in another language would change every note's hash.
        language_provider: None,
    });

    let mut report = Report::default();
    let mut walk = Walk::new(args.page_size);
    let mut counted: HashSet<NotificationId> = HashSet::new();
    let pause = Duration::from_millis(args.pause_ms);
    let keep = |keeper: &mut Option<Keeper>, line: IndexLine, captured: Option<&Captured>| {
        if let Some(keeper) = keeper
            && let Err(err) = keeper.keep(line, captured)
        {
            eprintln!("couldn't keep a response: {err}");
        }
    };

    report.end = loop {
        if report.requests + 2 > args.max_requests {
            break format!("stopped at --max-requests {}", args.max_requests);
        }
        let query = NotificationsQuery {
            number: Some(walk.number),
            before: walk.before.map(at_second),
            ..Default::default()
        };

        report.requests += 1;
        let params = NotificationsParams {
            query: query.clone(),
            include_body: true,
            include_context: true,
        };
        let whole = client.notifications().list(&params).await;
        let captured = executor.take_last();
        tokio::time::sleep(pause).await;
        let whole = match whole {
            Ok(response) => response.data,
            Err(error) => {
                let kind = failure(&mut report, &error);
                keep(
                    &mut keeper,
                    line("notes", captured.as_ref(), Some(kind.clone())),
                    captured.as_ref(),
                );
                break format!("stopped: {kind}");
            }
        };
        let mut index = line("notes", captured.as_ref(), None);
        index.notes = Some(whole.notes.len());
        index.unreadable = Some(whole.unreadable_notes.len()).filter(|n| *n > 0);
        keep(&mut keeper, index, captured.as_ref());

        report.requests += 1;
        let listed = client.notifications().list_ids(&query).await;
        let captured = executor.take_last();
        tokio::time::sleep(pause).await;
        let listed = match listed {
            Ok(response) => response.data,
            Err(error) => {
                let kind = failure(&mut report, &error);
                keep(
                    &mut keeper,
                    line("ids", captured.as_ref(), Some(kind.clone())),
                    captured.as_ref(),
                );
                break format!("stopped: {kind}");
            }
        };
        let mut index = line("ids", captured.as_ref(), None);
        index.notes = Some(listed.ids.len());
        index.unreadable = Some(listed.unreadable_notes.len()).filter(|n| *n > 0);
        keep(&mut keeper, index, captured.as_ref());
        report.pages += 1;

        // A note comes back on the next page when it's the oldest of this one, so each is
        // counted once.
        for note in &whole.notes {
            if counted.insert(note.id) {
                let check = check_note(note);
                report.note(note, &check);
            }
        }
        let new_unreadable: Vec<_> = whole
            .unreadable_notes
            .iter()
            .filter(|note| note.id.is_none_or(|id| counted.insert(id)))
            .cloned()
            .collect();
        report.unreadable(&new_unreadable);
        report.unreadable(&listed.unreadable_notes);
        let undated = |time: Option<i64>| time.is_none_or(|time| time < walk::UNDATED_BEFORE);
        let times: Vec<_> = whole
            .notes
            .iter()
            .map(|note| (note.id, note.timestamp.map(|time| time.0.timestamp())))
            .collect();
        let dated_after_undated = times
            .iter()
            .skip_while(|(_, time)| !undated(*time))
            .any(|(_, time)| !undated(*time));
        report.pages_with_an_undated_note_before_a_dated_one += u64::from(dated_after_undated);

        let unreadable_ids: Vec<_> = whole
            .unreadable_notes
            .iter()
            .filter_map(|note| note.id)
            .collect();
        let (step, _) = walk.take(Page {
            notes: &times,
            unreadable: &unreadable_ids,
            listed: &listed.ids,
        });
        report.notes_listed_only = walk.listed_only() as u64;
        match step {
            Step::Continue => {}
            Step::End => break END_REACHED.to_string(),
            Step::Stuck => {
                break "stopped: a full page brought nothing older, so more notes share one \
                       second, or more notes in a row can't be shown, than a page holds"
                    .to_string();
            }
        }
        if report.pages.is_multiple_of(10) {
            println!("{}", report.summary());
        }
    };

    let text = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())? + "\n";
    fs::write(out.join("report.json"), text).map_err(|err| format!("report.json: {err}"))?;
    println!("{}", report.summary());
    println!("report written to {}", out.join("report.json").display());
    Ok(report)
}

fn at_second(seconds: i64) -> WpGmtDateTime {
    WpGmtDateTime(
        chrono::DateTime::from_timestamp(seconds, 0).expect("a note's second is a valid time"),
    )
}

/// Counts a failed request, and returns what kind of failure it was, without any value in it.
fn failure(report: &mut Report, error: &WpApiError) -> String {
    let kind = match (error, error.status_code()) {
        (WpApiError::ResponseParsingError { reason, .. }, _) => {
            let kind = without_values(reason);
            *report
                .pages_that_did_not_parse
                .entry(kind.clone())
                .or_default() += 1;
            return format!("a page didn't parse: {kind}");
        }
        (_, Some(429)) => "HTTP 429: WordPress.com asked the sweep to slow down".to_string(),
        (_, Some(status)) => format!("HTTP {status}"),
        (error, None) => format!("the request failed: {}", without_values(&error.to_string())),
    };
    *report.failed_requests.entry(kind.clone()).or_default() += 1;
    kind
}

fn line<'a>(
    request: &'a str,
    captured: Option<&'a Captured>,
    error: Option<String>,
) -> IndexLine<'a> {
    IndexLine {
        seq: captured.map(|c| c.seq),
        file: None,
        request,
        url: captured.map(|c| c.url.as_str()),
        status: captured.map(|c| c.status),
        bytes: captured.map(|c| c.body.len()),
        millis: captured.map(|c| c.millis),
        notes: None,
        unreadable: None,
        error,
    }
}

fn access_token() -> Result<String, String> {
    if let Ok(token) = std::env::var("WP_COM_API_KEY")
        && !token.is_empty()
    {
        return Ok(token);
    }
    let token = WpComTestCredentials::instance().bearer_token;
    if token.is_empty() {
        return Err("set WP_COM_API_KEY, or add wp_com_test_credentials.json".to_string());
    }
    Ok(token.to_string())
}
