use async_trait::async_trait;
use integration_test_credentials::WpComTestCredentials;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use tokio::runtime::Runtime;
use wp_api::{
    prelude::*,
    wp_com::{WpComSiteId, client::WpComApiClient},
};
use wp_mobile::service::WpService;
use wp_mobile::service::sites::SiteInfo;
use wp_mobile_cache::WpApiCache;

pub struct TestContext {
    pub client: WpComApiClient,
    /// A client with no account, for the endpoints that answer signed out.
    pub signed_out_client: WpComApiClient,
    pub delegate: WpApiClientDelegate,
    pub service: WpService,
    pub site_id: WpComSiteId,
    pub token: String,
    pub runtime: Runtime,
    pub locks: AccountLocks,
}

/// Account state that some trials change and put back, and others read and compare exactly. CI
/// runs every trial at once, so a trial that changes some holds its lock for writing for the whole
/// trial, and one whose checks it would break holds it for reading. A trial that needs more than
/// one takes them in the order of these fields, at its start, so no two trials wait on each other.
#[derive(Default)]
pub struct AccountLocks {
    /// Any change of the account. Every trial that sends one holds it with [`making_a_change`],
    /// so they still run side by side. The tag follow trial holds it with [`changing_alone`],
    /// so it runs while no other trial changes anything: a tag follow sent while the account's
    /// other changes are sent is often answered 200 and not kept (24 of 37 on 2026-11-20, see
    /// `ReaderService::set_tag_followed`).
    pub changes: RwLock<()>,
    /// The tags the account follows, which the menu lists, and the streams made of them.
    pub tags: RwLock<()>,
    /// The sites the account follows, how each reaches it (emails and notifications), and the
    /// sites it blocks: a blocked site reads as not followed, and a follow of one is refused. The
    /// Following stream is made of them.
    pub follows: RwLock<()>,
    /// The seen state of the account's followed posts: the trials that change it can pick the
    /// same post.
    pub seen: Mutex<()>,
    /// The newest post of the account's own test site: the account's like of it, its comments,
    /// and the account's subscription to them. The site's comment emails change that
    /// subscription too: turning them on subscribes the account to the post's comments, and
    /// turning them off unsubscribes it.
    pub own_post: Mutex<()>,
}

/// Holds account state for a trial that changes it. A trial that failed by panicking leaves the
/// lock poisoned, which says nothing about the next trial, so it's taken anyway.
pub fn changing(lock: &RwLock<()>) -> RwLockWriteGuard<'_, ()> {
    lock.write().unwrap_or_else(PoisonError::into_inner)
}

/// Holds account state for a trial that reads it and expects it not to change.
pub fn reading(lock: &RwLock<()>) -> RwLockReadGuard<'_, ()> {
    lock.read().unwrap_or_else(PoisonError::into_inner)
}

/// Holds account state that only trials that change it use.
pub fn alone(lock: &Mutex<()>) -> MutexGuard<'_, ()> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Held for the whole of a trial that sends any change of the account. See
/// [`AccountLocks::changes`].
pub fn making_a_change(locks: &AccountLocks) -> RwLockReadGuard<'_, ()> {
    reading(&locks.changes)
}

/// Held for the whole of a trial whose changes must not overlap any other trial's. See
/// [`AccountLocks::changes`].
pub fn changing_alone(locks: &AccountLocks) -> RwLockWriteGuard<'_, ()> {
    changing(&locks.changes)
}

/// How many times [`read_back`] asks, and how long it waits between asks.
const READ_BACK_TRIES: u32 = 5;
const READ_BACK_PAUSE: Duration = Duration::from_secs(2);

/// Reads back something the server took, asking again until `shows_it` accepts the answer.
///
/// For a moment after a change, a read can be answered with what the server had before it: seen
/// marks and a followed site's notifications have read back stale straight after the server took
/// the change, and then caught up. (A post's comment subscription once seemed to, but that was
/// another trial changing the site's comment emails meanwhile, which the `own_post` lock now
/// keeps apart.) So the read is asked
/// again, [`READ_BACK_PAUSE`] apart, until `shows_it` accepts it or it has been asked
/// [`READ_BACK_TRIES`] times. The last answer is returned either way, for the trial to compare
/// exactly and fail with its own message. A read that fails isn't asked again: its error is
/// returned.
pub async fn read_back<T>(
    mut read: impl AsyncFnMut() -> Result<T, libtest_mimic::Failed>,
    shows_it: impl Fn(&T) -> bool,
) -> Result<T, libtest_mimic::Failed> {
    let mut answer = read().await?;
    for _ in 1..READ_BACK_TRIES {
        if shows_it(&answer) {
            break;
        }
        tokio::time::sleep(READ_BACK_PAUSE).await;
        answer = read().await?;
    }
    Ok(answer)
}

#[derive(Debug)]
struct EmptyAppNotifier;

#[async_trait]
impl WpAppNotifier for EmptyAppNotifier {
    async fn requested_with_invalid_authentication(&self, _request_url: String) {
        // no-op
    }
}

impl TestContext {
    pub fn new(token: String) -> Self {
        let runtime = Runtime::new().expect("Failed to create Tokio runtime");

        let delegate = WpApiClientDelegate {
            auth_provider: WpAuthenticationProvider::static_with_auth(WpAuthentication::Bearer {
                token: token.clone(),
            })
            .into(),
            request_executor: Arc::new(ReqwestRequestExecutor::new(false, Duration::from_secs(60))),
            middleware_pipeline: Arc::new(WpApiMiddlewarePipeline::default()),
            app_notifier: Arc::new(EmptyAppNotifier),
            language_provider: None,
        };

        let client = WpComApiClient::new(delegate.clone());
        let signed_out_client = WpComApiClient::new(WpApiClientDelegate {
            auth_provider: WpAuthenticationProvider::none().into(),
            ..delegate.clone()
        });

        let site_id = WpComTestCredentials::instance().site_id;
        // Use a test site on the test account used in CI.
        let site_id = if site_id == 0 { 229889220 } else { site_id };

        let cache = Arc::new(WpApiCache::new(None).expect("Failed to create in-memory cache"));
        cache
            .perform_migrations()
            .expect("Migrations should succeed");

        let service = WpService::new(
            SiteInfo::WordPressCom {
                site_id: WpComSiteId(site_id),
            },
            delegate.clone(),
            cache.clone(),
        )
        .expect("Failed to create WpService");

        Self {
            client,
            signed_out_client,
            delegate,
            service,
            site_id: WpComSiteId(site_id),
            token,
            runtime,
            locks: AccountLocks::default(),
        }
    }
}
