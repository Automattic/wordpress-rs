//! Runs the library's Reader code against a large amount of real data from WordPress.com.
//!
//! A sweep walks Reader streams page by page through the same client the apps use, and checks that
//! every response parses, and every post in it. It starts from the streams it's given and, with
//! `--discover`, goes on to the site and feed of every post it finds. Every response is kept, so a
//! run can be replayed without the network, and a response that didn't parse can be looked at.
//!
//! ```bash
//! # Two public streams, three pages each, into a directory outside the checkout
//! cargo run -p wp_com_e2e --bin reader_sweep -- \
//!     --out ~/reader-sweep/first --stream site:3584907 --stream feed:8857199 --pages-per-stream 3
//!
//! # From the signed-in user's Following stream outwards, up to 2,000 requests
//! cargo run -p wp_com_e2e --bin reader_sweep -- \
//!     --out ~/reader-sweep/wide --stream following --discover --max-streams 500 --max-requests 2000
//!
//! # Discover's cards and a search, 50 pages each
//! cargo run -p wp_com_e2e --bin reader_sweep -- \
//!     --out ~/reader-sweep/cards --stream discover --stream search:date:photography \
//!     --pages-per-stream 50
//! ```
//!
//! Every stream the library has can be walked: see `--stream` for how to name each. A stream
//! ordered by a date pages by it, as the library does: Liked by when each post was liked, Freshly
//! Pressed by when each is shown, and the others by when each was published. The rest page by the
//! handle each page gives. A stream of cards counts each kind of card it sees, and a kind the
//! library doesn't know is a finding.
//!
//! Each post that's read also has its card's fields worked out (its summary, reading time and
//! layout). A panic is a finding, and so is a post with words that gets no summary or reading time.
//!
//! The access token comes from `WP_COM_API_KEY`, or from `wp_com_test_credentials.json`, as it
//! does for `wp_com_e2e`. `--anonymous` sends none, which is how a signed-out reader sees the API.
//!
//! The run's directory holds real posts, some of them private, so it can't be inside a Git
//! checkout. It exits with `1` if any response didn't parse, any post or card in one couldn't be
//! read, a card was of a kind the library doesn't know, or working out a post's card fields
//! panicked.

mod checks;
mod recording;
mod sweep;

use async_trait::async_trait;
use checks::{Census, Finding};
use clap::Parser;
use integration_test_credentials::WpComTestCredentials;
use recording::{Captured, IndexLine, Recorder, RecordingExecutor};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sweep::{Cursor, Queue, StreamKey, check_output_dir, next_before, parse_seeds, streams_of};
use wp_api::prelude::*;
use wp_api::wp_com::WpComSiteId;
use wp_api::wp_com::client::WpComApiClient;
use wp_api::wp_com::reader_post_display::{
    reader_post_card_type, reader_post_reading_time_minutes, reader_post_summary,
};
use wp_api::wp_com::reader_posts::{
    ReaderCard, ReaderDiscoverParams, ReaderFirstPostsParams, ReaderFreshlyPressedParams,
    ReaderLikedPostsParams, ReaderListOwner, ReaderListPostsParams, ReaderListSlug,
    ReaderOrganizationPostsParams, ReaderPost, ReaderPostsResponse, ReaderSearchPostsParams,
    ReaderStreamParams, ReaderTagPostsParams, ReaderTagPostsResponse, ReaderTagSlug,
    ReaderTagsPostsParams, ReaderUnreadableCard, ReaderUnreadablePost,
};
use wp_api::wp_com::reader_saved::ReaderFeedId;

#[derive(Parser, Debug)]
#[command(
    about = "Run the Reader endpoints against real WordPress.com data, keeping every response"
)]
struct Args {
    /// Directory for the run's responses and index. It can't be inside a Git checkout, or hold a
    /// run already.
    #[arg(long)]
    out: PathBuf,

    /// A stream to start from: `following`, `liked`, `a8c` (Automattic's posts), `p2`,
    /// `freshly-pressed`, `site:<id>`, `feed:<id>`, `tag:<slug>`, `tags:<slug>,<slug>` (several
    /// tags' newest posts), `search:<relevance|date>:<query>`, `list:<owner>/<slug>`, `discover`,
    /// `discover:<slug>,<slug>` (leaning toward those tags) or `first-posts`. Repeat it for more.
    #[arg(long = "stream")]
    streams: Vec<StreamKey>,

    /// A file of streams to start from, one per line. `#` starts a comment.
    #[arg(long)]
    seeds: Option<PathBuf>,

    /// Also walk the site and the feed of every post found.
    #[arg(long)]
    discover: bool,

    /// Also walk the stream of every tag on every post found. There are far more tags than a run
    /// can walk, so this only ends at one of the run's limits.
    #[arg(long)]
    discover_tags: bool,

    /// The most streams to walk.
    #[arg(long, default_value_t = 50)]
    max_streams: usize,

    /// The most pages to read from one stream.
    #[arg(long, default_value_t = 5)]
    pages_per_stream: u32,

    /// The most requests to send in the whole run.
    #[arg(long, default_value_t = 500)]
    max_requests: u64,

    /// Posts per page. The API returns at most 40, and at most 10 for a tag. A search is sent 20 at
    /// most, which is its limit, and the first posts stream 15, since it answers no posts to more.
    #[arg(long, default_value_t = 40)]
    page_size: u32,

    /// The `meta` parameter to send with each request, such as `site,feed`. A tag's stream doesn't
    /// take one.
    #[arg(long)]
    meta: Option<String>,

    /// The `fields` parameter to send with each request: the fields of each post to return, such
    /// as `ID,site_ID,date`. It needs `ID`, or no post can be read, and `date`, or a stream stops
    /// after its first page. A tag's stream doesn't take one.
    #[arg(long)]
    fields: Option<String>,

    /// How many streams to walk at once.
    #[arg(long, default_value_t = 4)]
    concurrency: usize,

    /// Milliseconds each worker waits between its requests.
    #[arg(long, default_value_t = 0)]
    delay_ms: u64,

    /// Send no access token, as a signed-out reader would. `following`, `liked`, `a8c` and `p2`
    /// need one.
    #[arg(long)]
    anonymous: bool,

    /// Which responses to keep: `all`, or only the `interesting` ones, which didn't parse or hold
    /// something that isn't right. A long run should keep only those, or it fills the disk.
    #[arg(long, value_enum, default_value_t = Keep::All)]
    keep: Keep,

    /// With `--keep interesting`, how many responses to keep for each kind of finding.
    #[arg(long, default_value_t = 50)]
    examples_per_kind: u64,

    /// Stop starting requests after this many minutes, and finish up.
    #[arg(long)]
    max_minutes: Option<u64>,
}

impl Args {
    /// How many requests between progress lines: often enough to watch, without a long run
    /// printing thousands of them.
    fn progress_every(&self) -> u64 {
        (self.max_requests / 40).clamp(25, 5000)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Keep {
    All,
    Interesting,
}

/// What a run did, printed at the end and written to `summary.json`.
#[derive(Debug, Default, Clone, Serialize)]
struct Totals {
    requests: u64,
    ok: u64,
    parse_errors: u64,
    /// Posts the library couldn't read, on pages that otherwise parsed.
    unreadable_posts: u64,
    http_errors: u64,
    request_failures: u64,
    streams_walked: u64,
    posts: u64,
    /// The cards a stream of cards sent, by the kind the API gave each, posts among them.
    cards: BTreeMap<String, u64>,
    /// Cards of a kind the library doesn't know, which nothing can show.
    unknown_cards: u64,
    /// The cards on a list's page that came with `false` where the post would be.
    missing_posts: u64,
    /// What working out each post's card fields gave.
    computed: Computed,
    wordpress_com_posts: u64,
    jetpack_posts: u64,
    rss_posts: u64,
    bytes: u64,
    /// Values that parsed but aren't right, by kind.
    findings: BTreeMap<&'static str, u64>,
    responses_kept: u64,
}

/// What working out each post's card fields gave: its summary, reading time and layout.
#[derive(Debug, Default, Clone, Serialize)]
struct Computed {
    posts: u64,
    /// Posts one of the functions panicked on. Each is a finding too.
    panics: u64,
    /// Posts with no summary, and how many of those have no words in their excerpt or content to
    /// make one from, such as a post that is only an image.
    no_summary: u64,
    no_summary_and_no_words: u64,
    /// Posts with no reading time, and how many of those have no words in their content to time.
    no_reading_time: u64,
    no_reading_time_and_no_words: u64,
    card_types: BTreeMap<String, u64>,
}

struct Shared {
    args: Args,
    queue: Mutex<Queue>,
    /// Workers walking a stream now. A worker only stops once the queue is empty and nobody is
    /// walking, since a stream being walked can still add more.
    walking: AtomicUsize,
    requests: AtomicU64,
    totals: Mutex<Totals>,
    census: Mutex<Census>,
    /// With `--keep interesting`, how many responses have been kept for each kind of finding.
    examples: Mutex<BTreeMap<&'static str, u64>>,
    recorder: Arc<Recorder>,
    started: Instant,
    this_year: i32,
}

#[derive(Debug)]
struct NoNotifier;

#[async_trait]
impl WpAppNotifier for NoNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {}
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(totals)
            if totals.parse_errors == 0
                && totals.unreadable_posts == 0
                && totals.unknown_cards == 0
                && totals.computed.panics == 0 =>
        {
            ExitCode::SUCCESS
        }
        Ok(_) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

async fn run(args: Args) -> Result<Totals, String> {
    let out = check_output_dir(&args.out)?;
    let mut seeds = args.streams.clone();
    if let Some(path) = &args.seeds {
        let text =
            std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
        seeds.extend(parse_seeds(&text)?);
    }
    if seeds.is_empty() {
        return Err("give at least one stream to start from, with --stream or --seeds".to_string());
    }
    if args.anonymous
        && let Some(stream) = seeds.iter().find(|stream| stream.needs_account())
    {
        return Err(format!(
            "`{stream}` needs a signed-in user, so it can't be swept with --anonymous"
        ));
    }
    let token = if args.anonymous {
        None
    } else {
        Some(access_token()?)
    };
    let recorder = Recorder::create(&out)
        .map_err(|err| format!("{}: {err} (a run's directory must be new)", out.display()))?;

    let workers = args.concurrency.max(1);
    let shared = Arc::new(Shared {
        queue: Mutex::new(Queue::new(seeds, args.max_streams)),
        walking: AtomicUsize::new(0),
        requests: AtomicU64::new(0),
        totals: Mutex::new(Totals::default()),
        census: Mutex::new(Census::default()),
        examples: Mutex::new(BTreeMap::new()),
        recorder: Arc::new(recorder),
        started: Instant::now(),
        this_year: this_year(),
        args,
    });
    let tasks: Vec<_> = (0..workers)
        .map(|_| tokio::spawn(work(Arc::clone(&shared), token.clone())))
        .collect();
    for task in tasks {
        task.await
            .map_err(|err| format!("a worker stopped: {err}"))?;
    }

    let totals = shared.totals.lock().expect("totals lock poisoned").clone();
    let seen = shared.queue.lock().expect("queue lock poisoned").seen();
    let elapsed = shared.started.elapsed().as_secs_f64();
    let summary = serde_json::json!({
        "totals": totals,
        "streams_seen": seen,
        "elapsed_seconds": (elapsed * 10.0).round() / 10.0,
    });
    shared
        .recorder
        .write_json("summary.json", &summary)
        .map_err(|err| format!("summary.json: {err}"))?;
    let census = shared.census.lock().expect("census lock poisoned").clone();
    shared
        .recorder
        .write_json("census.json", &census)
        .map_err(|err| format!("census.json: {err}"))?;
    println!("{}", progress_line(&totals, seen, elapsed));
    println!("run kept in {}", out.display());
    Ok(totals)
}

fn access_token() -> Result<String, String> {
    if let Ok(token) = std::env::var("WP_COM_API_KEY")
        && !token.is_empty()
    {
        return Ok(token);
    }
    let token = WpComTestCredentials::instance().bearer_token;
    if token.is_empty() {
        return Err(
            "set WP_COM_API_KEY, add wp_com_test_credentials.json, or pass --anonymous".to_string(),
        );
    }
    Ok(token.to_string())
}

/// The current year, for telling a date that's too far in the future.
fn this_year() -> i32 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    1970 + (seconds / 31_556_952) as i32
}

fn progress_line(totals: &Totals, streams_seen: usize, elapsed: f64) -> String {
    format!(
        "{} requests in {:.0}s: {} ok, {} didn't parse, {} HTTP errors, {} failed | {} posts \
         ({} WordPress.com, {} Jetpack, {} RSS), {} more unreadable | cards {:?}, {} unknown | \
         {} of {} streams walked | {:.1} MB | findings {:?}",
        totals.requests,
        elapsed,
        totals.ok,
        totals.parse_errors,
        totals.http_errors,
        totals.request_failures,
        totals.posts,
        totals.wordpress_com_posts,
        totals.jetpack_posts,
        totals.rss_posts,
        totals.unreadable_posts,
        totals.cards,
        totals.unknown_cards,
        totals.streams_walked,
        streams_seen,
        totals.bytes as f64 / 1_000_000.0,
        totals.findings,
    )
}

/// One worker: takes streams off the queue and walks them until there are none left.
async fn work(shared: Arc<Shared>, token: Option<String>) {
    let executor = Arc::new(RecordingExecutor::new(
        Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
        Arc::clone(&shared.recorder),
    ));
    let authentication = match token {
        Some(token) => WpAuthentication::Bearer { token },
        None => WpAuthentication::None,
    };
    let client = WpComApiClient::new(WpApiClientDelegate {
        auth_provider: WpAuthenticationProvider::static_with_auth(authentication).into(),
        request_executor: executor.clone(),
        middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
        app_notifier: Arc::new(NoNotifier),
        language_provider: None,
    });
    loop {
        let next = {
            let mut queue = shared.queue.lock().expect("queue lock poisoned");
            let next = queue.next();
            if next.is_some() {
                shared.walking.fetch_add(1, Ordering::SeqCst);
            }
            next
        };
        match next {
            Some(_) if out_of_time(&shared) => {
                shared.walking.fetch_sub(1, Ordering::SeqCst);
                break;
            }
            Some(stream) => {
                walk(&shared, &client, &executor, &stream).await;
                shared.walking.fetch_sub(1, Ordering::SeqCst);
            }
            None if shared.walking.load(Ordering::SeqCst) == 0 => break,
            None => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

/// Whether the run has used the time it was given.
fn out_of_time(shared: &Shared) -> bool {
    shared
        .args
        .max_minutes
        .is_some_and(|minutes| shared.started.elapsed() >= Duration::from_secs(minutes * 60))
}

/// A page of a stream, and where the next one starts.
#[derive(Default)]
struct Page {
    /// The posts on the page, the posts of its cards among them.
    posts: Vec<ReaderPost>,
    /// The posts or cards on the page that the library couldn't read.
    unreadable: Vec<ReaderUnreadablePost>,
    /// For a stream of cards, how many of each kind it had, by the name the API gives the kind.
    cards: BTreeMap<String, u64>,
    /// The kinds of the page's cards that the library doesn't know, one entry per card.
    unknown_cards: Vec<String>,
    /// Cards on a list's page with no post.
    missing_posts: u32,
    next: Option<Cursor>,
}

/// Reads a stream page by page until it ends or the run's limits are reached.
async fn walk(
    shared: &Shared,
    client: &WpComApiClient,
    executor: &RecordingExecutor,
    stream: &StreamKey,
) {
    let name = stream.to_string();
    let mut cursor = None;
    for _ in 0..shared.args.pages_per_stream {
        if out_of_time(shared)
            || shared.requests.fetch_add(1, Ordering::SeqCst) >= shared.args.max_requests
        {
            break;
        }
        let result = fetch(client, stream, cursor.as_ref(), &shared.args).await;
        let captured = executor.take_last();
        let next = match &result {
            Ok(page) => {
                if shared.args.discover || shared.args.discover_tags {
                    let mut queue = shared.queue.lock().expect("queue lock poisoned");
                    for post in &page.posts {
                        if shared.args.discover {
                            for found in streams_of(post) {
                                queue.add(found);
                            }
                        }
                        if shared.args.discover_tags {
                            for tag in post.tags.iter().flatten() {
                                queue.add(StreamKey::Tag(tag.slug.clone()));
                            }
                        }
                    }
                }
                page.next.clone()
            }
            Err(_) => None,
        };
        report(shared, &name, captured.as_ref(), result.as_ref());
        match next {
            Some(next) => cursor = Some(next),
            None => break,
        }
        if shared.args.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(shared.args.delay_ms)).await;
        }
    }
    shared
        .totals
        .lock()
        .expect("totals lock poisoned")
        .streams_walked += 1;
}

/// One page of a stream. A server that's busy is asked again after a wait, up to three times; only
/// the last answer gets a line in the index.
async fn fetch(
    client: &WpComApiClient,
    stream: &StreamKey,
    cursor: Option<&Cursor>,
    args: &Args,
) -> Result<Page, WpApiError> {
    let mut waits = [2u64, 10, 30].into_iter();
    loop {
        match fetch_once(client, stream, cursor, args).await {
            Ok(page) => return Ok(page),
            Err(error) => {
                let busy = matches!(error.status_code(), Some(429 | 500..=599));
                match waits.next() {
                    Some(seconds) if busy => tokio::time::sleep(Duration::from_secs(seconds)).await,
                    _ => return Err(error),
                }
            }
        }
    }
}

async fn fetch_once(
    client: &WpComApiClient,
    stream: &StreamKey,
    cursor: Option<&Cursor>,
    args: &Args,
) -> Result<Page, WpApiError> {
    let posts = client.reader_posts();
    let before = match cursor {
        Some(Cursor::Before(date)) => Some(*date),
        _ => None,
    };
    let page_handle = match cursor {
        Some(Cursor::Handle(handle)) => Some(handle.clone()),
        _ => None,
    };
    let number = Some(args.page_size);
    let (meta, fields) = (args.meta.clone(), args.fields.clone());

    // Each endpoint has its own response type around a page of posts, or of cards.
    let page = match stream {
        StreamKey::Following | StreamKey::Site(_) | StreamKey::Feed(_) => {
            let params = ReaderStreamParams {
                number,
                before,
                after: None,
                meta,
                fields,
            };
            dated(match stream {
                StreamKey::Following => posts.following(&params).await?.data,
                StreamKey::Site(id) => posts.site_posts(&WpComSiteId(*id), &params).await?.data,
                StreamKey::Feed(id) => posts.feed_posts(&ReaderFeedId(*id), &params).await?.data,
                _ => unreachable!("only these three take these parameters"),
            })
        }
        StreamKey::Liked => {
            let params = ReaderLikedPostsParams {
                number,
                before,
                meta,
                fields,
                ..Default::default()
            };
            dated(posts.liked(&params).await?.data)
        }
        StreamKey::Automattic | StreamKey::P2 => {
            let params = ReaderOrganizationPostsParams {
                number,
                before,
                meta,
                fields,
                ..Default::default()
            };
            dated(match stream {
                StreamKey::Automattic => posts.automattic(&params).await?.data,
                _ => posts.following_p_2(&params).await?.data,
            })
        }
        StreamKey::FreshlyPressed => {
            let params = ReaderFreshlyPressedParams {
                number,
                before,
                meta,
                fields,
                ..Default::default()
            };
            dated(posts.freshly_pressed(&params).await?.data)
        }
        StreamKey::Tag(slug) => {
            let params = ReaderTagPostsParams {
                number,
                page_handle,
                ..Default::default()
            };
            tag_page(
                posts
                    .tag_posts(&ReaderTagSlug(slug.clone()), &params)
                    .await?
                    .data,
            )
        }
        StreamKey::Tags(slugs) => {
            let params = ReaderTagsPostsParams {
                tags: slugs.iter().cloned().map(ReaderTagSlug).collect(),
                number,
                page_handle,
                ..Default::default()
            };
            tag_page(posts.tags_posts(&params).await?.data)
        }
        StreamKey::Search { sort, query } => {
            let params = ReaderSearchPostsParams {
                q: query.clone(),
                number: Some(args.page_size.min(SEARCH_PAGE_SIZE_MAX)),
                page_handle,
                sort: Some((*sort).into()),
                meta,
                fields,
                ..Default::default()
            };
            let page = posts.search(&params).await?.data;
            Page {
                posts: page.posts.into_iter().map(|result| result.post).collect(),
                unreadable: page.unreadable_posts,
                next: page.next_page_handle.map(Cursor::Handle),
                ..Default::default()
            }
        }
        StreamKey::List { owner, slug } => {
            let params = ReaderListPostsParams {
                number,
                page_handle,
                ..Default::default()
            };
            let page = posts
                .list_posts(
                    &ReaderListOwner(owner.clone()),
                    &ReaderListSlug(slug.clone()),
                    &params,
                )
                .await?
                .data;
            // A page of only missing posts still moves on, so the handle decides.
            let has_cards = !page.posts.is_empty()
                || !page.unreadable_posts.is_empty()
                || page.missing_posts > 0;
            Page {
                posts: page.posts,
                unreadable: page.unreadable_posts,
                missing_posts: page.missing_posts,
                next: page
                    .next_page_handle
                    .filter(|_| has_cards)
                    .map(Cursor::Handle),
                ..Default::default()
            }
        }
        StreamKey::Discover(tags) => {
            let params = ReaderDiscoverParams {
                count: number,
                page_handle,
                tags: tags.iter().cloned().map(ReaderTagSlug).collect(),
                ..Default::default()
            };
            let page = posts.discover(&params).await?.data;
            card_page(page.cards, page.unreadable_cards, page.next_page_handle)
        }
        StreamKey::FirstPosts => {
            let params = ReaderFirstPostsParams {
                count: Some(args.page_size.min(FIRST_POSTS_PAGE_SIZE_MAX)),
                page_handle,
            };
            let page = posts.first_posts(&params).await?.data;
            card_page(page.cards, page.unreadable_cards, page.next_page_handle)
        }
    };
    if !stream.pages_by_date() {
        return Ok(page);
    }
    // An unreadable post has no date to page from, so the next page starts from the oldest post
    // that was read, and returns the unreadable one again if that was older.
    let next = next_before(page.posts.iter().map(|post| stream.sort_date(post)), before)
        .map(Cursor::Before);
    Ok(Page { next, ..page })
}

/// The most posts a page of search results can ask for: the API answers 400 to more.
const SEARCH_PAGE_SIZE_MAX: u32 = 20;

/// The most cards a page of the first posts stream can ask for: asked for more, the API answers
/// no posts and no handle.
const FIRST_POSTS_PAGE_SIZE_MAX: u32 = 15;

/// A page of a stream ordered by date. Where the next page starts is worked out from its posts.
fn dated(page: ReaderPostsResponse) -> Page {
    Page {
        posts: page.posts,
        unreadable: page.unreadable_posts,
        ..Default::default()
    }
}

fn tag_page(page: ReaderTagPostsResponse) -> Page {
    // A page without posts ends the stream, whatever handle it has.
    let next = match page.posts.is_empty() {
        true => None,
        false => page.next_page_handle.map(Cursor::Handle),
    };
    Page {
        posts: page.posts,
        unreadable: page.unreadable_posts,
        next,
        ..Default::default()
    }
}

/// A page of cards: its posts, how many of each kind of card it had, and the kinds the library
/// doesn't know.
fn card_page(
    cards: Vec<ReaderCard>,
    unreadable: Vec<ReaderUnreadableCard>,
    next_page_handle: Option<String>,
) -> Page {
    let mut page = Page::default();
    for card in cards {
        let kind = match card {
            ReaderCard::Post { post } => {
                page.posts.push(post);
                "post".to_string()
            }
            ReaderCard::RecommendedSites { .. } => "recommended_blogs".to_string(),
            ReaderCard::NewSites { .. } => "new_sites".to_string(),
            ReaderCard::SuggestedTags { .. } => "interests_you_may_like".to_string(),
            ReaderCard::Unknown { card_type } => {
                page.unknown_cards.push(card_type.clone());
                card_type
            }
        };
        *page.cards.entry(kind).or_default() += 1;
    }
    for card in unreadable {
        let kind = card.card_type.unwrap_or_else(|| "(no type)".to_string());
        *page.cards.entry(kind.clone()).or_default() += 1;
        page.unreadable.push(ReaderUnreadablePost {
            identity: card.post,
            error: format!("a `{kind}` card: {}", card.error),
        });
    }
    // The API sends an empty handle on some last pages.
    page.next = next_page_handle
        .filter(|handle| !handle.is_empty())
        .map(Cursor::Handle);
    page
}

/// Looks at a request's result, keeps the response if it's worth keeping, and writes its lines.
fn report(
    shared: &Shared,
    stream: &str,
    captured: Option<&Captured>,
    result: Result<&Page, &WpApiError>,
) {
    let (outcome, error) = match result {
        Ok(_) => ("ok", None),
        Err(WpApiError::ResponseParsingError { reason, .. }) => {
            ("parse_error", Some(reason.clone()))
        }
        Err(error) if error.status_code().is_some() => {
            ("http_error", Some(short(&error.to_string())))
        }
        Err(error) => ("request_failed", Some(short(&error.to_string()))),
    };
    let posts = result.ok().map(|page| page.posts.len());
    let unreadable = result.map_or(&[][..], |page| page.unreadable.as_slice());

    // What parsing can't see: values that fit the types and aren't right.
    let (mut findings, census) = match captured {
        Some(captured) if matches!(outcome, "ok" | "parse_error") => {
            checks::examine(&captured.body, shared.this_year)
        }
        _ => (Vec::new(), Census::default()),
    };
    findings.extend(unreadable.iter().map(unreadable_post));
    if let Ok(page) = result {
        findings.extend(page.unknown_cards.iter().map(|kind| Finding {
            kind: "unknown_card",
            site_id: None,
            post_id: None,
            field: "type".to_string(),
            value: kind.clone(),
            usual_offset: None,
        }));
    }
    let computed: Vec<_> = result
        .map(|page| page.posts.iter().map(computed_fields).collect())
        .unwrap_or_default();
    findings.extend(
        computed
            .iter()
            .flat_map(|fields| fields.findings.iter().cloned())
            .collect::<Vec<_>>(),
    );
    shared
        .census
        .lock()
        .expect("census lock poisoned")
        .merge(census);

    let file = captured
        .filter(|_| !unreadable.is_empty() || worth_keeping(shared, outcome, &findings))
        .and_then(|captured| shared.recorder.keep_body(captured).ok());
    let line = IndexLine {
        stream,
        at_millis: shared.started.elapsed().as_millis() as u64,
        seq: captured.map(|c| c.seq),
        file: file.as_deref(),
        method: captured.map(|c| c.method.as_str()),
        url: captured.map(|c| c.url.as_str()),
        status: captured.map(|c| c.status),
        bytes: captured.map(|c| c.body.len() as u64),
        millis: captured.map(|c| c.millis),
        outcome,
        posts,
        unreadable_posts: Some(unreadable.len()).filter(|count| *count > 0),
        cards: result
            .ok()
            .map(|page| &page.cards)
            .filter(|cards| !cards.is_empty()),
        findings: Some(findings.len()).filter(|count| *count > 0),
        error: error.as_deref(),
    };
    if let Err(err) = shared.recorder.write_index_line(&line) {
        eprintln!("couldn't write to index.jsonl: {err}");
    }
    for finding in &findings {
        let line = AnomalyLine {
            stream,
            seq: captured.map(|c| c.seq),
            file: file.as_deref(),
            finding,
        };
        if let Err(err) = shared.recorder.write_anomaly(&line) {
            eprintln!("couldn't write to anomalies.jsonl: {err}");
        }
    }
    // The reason can quote the post, so only where to look is printed.
    if outcome == "parse_error" {
        eprintln!(
            "didn't parse: {stream}, kept as {}",
            file.as_deref().unwrap_or("(no file)")
        );
    }
    if !unreadable.is_empty() {
        eprintln!(
            "{} of the page's posts couldn't be read: {stream}, kept as {}",
            unreadable.len(),
            file.as_deref().unwrap_or("(no file)")
        );
    }

    let mut totals = shared.totals.lock().expect("totals lock poisoned");
    totals.requests += 1;
    totals.bytes += captured.map_or(0, |c| c.body.len() as u64);
    totals.responses_kept += u64::from(file.is_some());
    totals.unreadable_posts += unreadable.len() as u64;
    for finding in &findings {
        *totals.findings.entry(finding.kind).or_default() += 1;
    }
    match outcome {
        "ok" => totals.ok += 1,
        "parse_error" => totals.parse_errors += 1,
        "http_error" => totals.http_errors += 1,
        _ => totals.request_failures += 1,
    }
    for fields in &computed {
        let totals = &mut totals.computed;
        totals.posts += 1;
        totals.panics += u64::from(fields.panicked);
        totals.no_summary += u64::from(!fields.summary);
        totals.no_summary_and_no_words += u64::from(!fields.summary && !fields.has_words);
        totals.no_reading_time += u64::from(!fields.reading_time);
        totals.no_reading_time_and_no_words +=
            u64::from(!fields.reading_time && !fields.content_has_words);
        if let Some(card_type) = &fields.card_type {
            *totals.card_types.entry(card_type.clone()).or_default() += 1;
        }
    }
    if let Ok(page) = result {
        totals.posts += page.posts.len() as u64;
        totals.unknown_cards += page.unknown_cards.len() as u64;
        totals.missing_posts += u64::from(page.missing_posts);
        for (kind, count) in &page.cards {
            *totals.cards.entry(kind.clone()).or_default() += count;
        }
        for post in &page.posts {
            match kind(post) {
                PostKind::WordPressCom => totals.wordpress_com_posts += 1,
                PostKind::Jetpack => totals.jetpack_posts += 1,
                PostKind::Rss => totals.rss_posts += 1,
            }
        }
    }
    if totals.requests.is_multiple_of(shared.args.progress_every()) {
        let seen = shared.queue.lock().expect("queue lock poisoned").seen();
        println!(
            "{}",
            progress_line(&totals, seen, shared.started.elapsed().as_secs_f64())
        );
    }
}

/// A post the library couldn't read, as a finding. The checks say what's wrong with a value where
/// they know to look; this says the post was lost to it, or to something they don't look for.
fn unreadable_post(post: &ReaderUnreadablePost) -> Finding {
    let identity = post.identity.as_ref();
    Finding {
        kind: "unreadable_post",
        site_id: identity
            .and_then(|identity| identity.site_id)
            .map(|id| id.0),
        post_id: identity.map(|identity| identity.id),
        field: "post".to_string(),
        value: short(&post.error),
        usual_offset: None,
    }
}

/// What working out a post's card fields gave.
struct ComputedFields {
    summary: bool,
    reading_time: bool,
    card_type: Option<String>,
    /// Whether the post's excerpt or content has a word in it, and whether its content does.
    has_words: bool,
    content_has_words: bool,
    /// Whether one of the functions panicked on the post.
    panicked: bool,
    /// A panic, or a summary or reading time missing from a post with words to make one from.
    findings: Vec<Finding>,
}

/// Works out a post's summary, reading time and layout, as an app showing it would.
fn computed_fields(post: &ReaderPost) -> ComputedFields {
    let content_has_words = has_words(post.content.as_deref());
    let mut fields = ComputedFields {
        summary: false,
        reading_time: false,
        card_type: None,
        has_words: content_has_words || has_words(post.excerpt.as_deref()),
        content_has_words,
        panicked: false,
        findings: Vec::new(),
    };
    let finding = |kind, value: String| Finding {
        kind,
        site_id: post.identity.site_id.map(|id| id.0),
        post_id: Some(post.identity.id),
        field: "post".to_string(),
        value,
        usual_offset: None,
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        (
            reader_post_summary(post).is_some(),
            reader_post_reading_time_minutes(post).is_some(),
            reader_post_card_type(post).to_string(),
        )
    }));
    match outcome {
        Ok((summary, reading_time, card_type)) => {
            fields.summary = summary;
            fields.reading_time = reading_time;
            fields.card_type = Some(card_type);
            // Not wrong by itself, but worth a look: the words may be ones the rules leave out.
            if !summary && fields.has_words {
                let words = "its excerpt or content has words".to_string();
                fields
                    .findings
                    .push(finding("no_summary_with_words", words));
            }
            if !reading_time && fields.content_has_words {
                let words = "its content has words".to_string();
                fields
                    .findings
                    .push(finding("no_reading_time_with_words", words));
            }
        }
        Err(panic) => {
            let message = panic
                .downcast_ref::<&str>()
                .map(|text| text.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "(no message)".to_string());
            fields.panicked = true;
            fields
                .findings
                .push(finding("computed_field_panic", short(&message)));
        }
    }
    fields
}

/// Whether HTML has a letter or a digit a reader would see: outside tags, their attribute values,
/// comments, character references, and elements that aren't shown, such as `<style>`. A rough
/// check, to tell a post with nothing to summarize, such as one that is only an image, from one
/// the functions should have summarized.
fn has_words(html: Option<&str>) -> bool {
    const HIDDEN: [&str; 5] = ["script", "style", "noscript", "iframe", "svg"];
    let html = html.unwrap_or_default();
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while at < html.len() {
        let rest = &html[at..];
        let c = rest.chars().next().expect("not at the end");
        if rest.starts_with("<!--") {
            at = lower[at..]
                .find("-->")
                .map_or(html.len(), |end| at + end + 3);
            continue;
        }
        if c == '<' && rest[1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/') {
            // The tag ends at the first `>` outside a quoted attribute value.
            let mut quote = None;
            let mut end = html.len();
            for (offset, c) in rest.char_indices().skip(1) {
                match (quote, c) {
                    (None, '"' | '\'') => quote = Some(c),
                    (Some(open), c) if c == open => quote = None,
                    (None, '>') => {
                        end = at + offset + 1;
                        break;
                    }
                    _ => {}
                }
            }
            let tag = &lower[at + 1..end.min(html.len())];
            let name: String = tag
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            let hidden = HIDDEN.contains(&name.as_str())
                || (name == "template" && !tag.contains("shadowrootmode"));
            at = end;
            if hidden && !tag.trim_end_matches('>').ends_with('/') {
                let close = format!("</{name}");
                at = lower[at..]
                    .find(&close)
                    .map_or(html.len(), |found| at + found);
            }
            continue;
        }
        if c == '&' {
            let reference = rest[1..]
                .find(|c: char| c == ';' || c.is_whitespace())
                .filter(|&end| rest.as_bytes()[end + 1] == b';');
            if let Some(end) = reference {
                at += end + 2;
                continue;
            }
        }
        if c.is_alphanumeric() {
            return true;
        }
        at += c.len_utf8();
    }
    false
}

/// One line of `anomalies.jsonl`.
#[derive(Serialize)]
struct AnomalyLine<'a> {
    stream: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    seq: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a str>,
    #[serde(flatten)]
    finding: &'a Finding,
}

/// Whether to keep a response's body. A run keeping everything keeps it. One keeping only what's
/// interesting keeps a response that didn't get a good answer, and the first few with each kind of
/// finding: enough to look at, without keeping one site's thousand copies of the same thing.
fn worth_keeping(shared: &Shared, outcome: &str, findings: &[Finding]) -> bool {
    if shared.args.keep == Keep::All || outcome != "ok" {
        return true;
    }
    let mut examples = shared.examples.lock().expect("examples lock poisoned");
    let mut keep = false;
    for finding in findings {
        let kept = examples.entry(finding.kind).or_default();
        if *kept < shared.args.examples_per_kind {
            *kept += 1;
            keep = true;
        }
    }
    keep
}

enum PostKind {
    WordPressCom,
    Jetpack,
    Rss,
}

fn kind(post: &ReaderPost) -> PostKind {
    // A site's own stream sends a Jetpack post as `is_external: false`, so `is_jetpack` decides first.
    match (post.is_jetpack, post.identity.is_external) {
        (Some(true), _) => PostKind::Jetpack,
        (_, true) => PostKind::Rss,
        (_, false) => PostKind::WordPressCom,
    }
}

/// The start of an error's text, for the index. An error can carry a whole response.
fn short(text: &str) -> String {
    const MAX: usize = 300;
    match text.char_indices().nth(MAX) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_looked_for_outside_tags_and_references() {
        assert!(has_words(Some("<p>A word</p>")));
        assert!(has_words(Some(
            "<figure><img alt=\"x\"></figure><p>写真</p>"
        )));
        assert!(has_words(Some("Fish &chips")));
        assert!(!has_words(None));
        assert!(!has_words(Some("")));
        assert!(!has_words(Some(
            "<figure><img src=\"a.jpg\" alt=\"A photo\"></figure>"
        )));
        assert!(!has_words(Some("<p>&nbsp;&#8230;</p>\n")));
        assert!(has_words(Some("<p>1 < 2</p>")));
        assert!(!has_words(Some(
            "<style>#gallery-1 { margin: auto; }</style><div></div>"
        )));
        assert!(!has_words(Some(
            "<SCRIPT>var a = 1;</SCRIPT><svg><title>Icon</title></svg>"
        )));
        assert!(!has_words(Some(
            "<img data-meta='{\"a\": \"b > c\"}' alt=\"x\">"
        )));
        assert!(!has_words(Some(
            "<!-- wp:gallery --><template><p>Hidden</p></template>"
        )));
        assert!(has_words(Some(
            "<template shadowrootmode=\"open\"><p>Shown</p></template>"
        )));
    }
}
